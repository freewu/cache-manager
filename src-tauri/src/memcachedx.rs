//! Memcached 支持：自实现文本协议客户端（零外部依赖）
//!
//! 支持：get / set / delete / touch / flush_all / stats / lru_crawler metadump / version / raw

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use parking_lot::{Mutex, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::error::{AppError, AppResult};
use crate::model::{ConnConfig, ConnMode, ConnStatus, ConnStatusInfo, EncodedValue};
use crate::redisx::keys::{KeyInfo, ScanPage, ValuePayload, ValueView};
use crate::redisx::server::{InfoSection, ServerInfo};
use crate::redisx::{STATUS_CONNECTED, STATUS_CONNECTING, STATUS_ERROR};

// ==================== Memcached 文本协议客户端 ====================

struct MemcachedInner {
    stream: TcpStream,
    buf: Vec<u8>,
}

impl MemcachedInner {
    async fn fill(&mut self) -> AppResult<()> {
        let mut tmp = [0u8; 8192];
        let n = self
            .stream
            .read(&mut tmp)
            .await
            .map_err(|e| AppError::new(format!("读取响应失败: {}", e)))?;
        if n == 0 {
            return Err(AppError::new("连接已关闭"));
        }
        self.buf.extend_from_slice(&tmp[..n]);
        Ok(())
    }

    /// 读一行（以 \r\n 结尾）
    async fn read_line(&mut self) -> AppResult<String> {
        loop {
            if let Some(pos) = self.buf.windows(2).position(|w| w == b"\r\n") {
                let line = String::from_utf8_lossy(&self.buf[..pos]).to_string();
                self.buf.drain(..pos + 2);
                return Ok(line);
            }
            self.fill().await?;
        }
    }

    async fn read_bytes(&mut self, n: usize) -> AppResult<Vec<u8>> {
        while self.buf.len() < n {
            self.fill().await?;
        }
        let data = self.buf[..n].to_vec();
        self.buf.drain(..n);
        Ok(data)
    }

    async fn write_raw(&mut self, data: &[u8]) -> AppResult<()> {
        self.stream
            .write_all(data)
            .await
            .map_err(|e| AppError::new(format!("发送命令失败: {}", e)))?;
        self.stream
            .flush()
            .await
            .map_err(|e| AppError::new(format!("发送命令失败: {}", e)))?;
        Ok(())
    }

    async fn write_cmd(&mut self, cmd: &str) -> AppResult<()> {
        self.write_raw(format!("{}\r\n", cmd).as_bytes()).await
    }
}

/// Memcached 客户端（连接内串行）
pub struct MemcachedClient {
    inner: Arc<tokio::sync::Mutex<MemcachedInner>>,
}

/// get 返回的值
pub struct MemcachedValue {
    pub flags: u32,
    pub data: Vec<u8>,
}

/// metadump 中的键元信息
pub struct MemcachedKeyMeta {
    pub key: String,
    /// 过期时间（Unix 秒，-1 表示永不过期）
    pub exp: i64,
    pub size: i64,
}

impl MemcachedClient {
    pub async fn connect(host: &str, port: u16, timeout_ms: u64) -> AppResult<Self> {
        let addr = format!("{}:{}", host, port);
        let stream = tokio::time::timeout(
            Duration::from_millis(timeout_ms.max(100)),
            TcpStream::connect(&addr),
        )
        .await
        .map_err(|_| AppError::new(format!("连接超时（{}ms）: {}", timeout_ms, addr)))?
        .map_err(|e| AppError::new(format!("连接失败: {} ({})", addr, e)))?;
        stream
            .set_nodelay(true)
            .map_err(|e| AppError::new(format!("设置连接参数失败: {}", e)))?;
        Ok(Self {
            inner: Arc::new(tokio::sync::Mutex::new(MemcachedInner {
                stream,
                buf: Vec::new(),
            })),
        })
    }

    /// get 单个键
    pub async fn get(&self, key: &str) -> AppResult<Option<MemcachedValue>> {
        let mut inner = self.inner.lock().await;
        inner.write_cmd(&format!("get {}", key)).await?;
        let line = inner.read_line().await?;
        if line.starts_with("VALUE ") {
            let parts: Vec<&str> = line.split(' ').collect();
            let flags = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
            let bytes = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
            let data = inner.read_bytes(bytes).await?;
            // 消费数据后的 \r\n
            let _ = inner.read_line().await;
            // END
            let _ = inner.read_line().await;
            Ok(Some(MemcachedValue { flags, data }))
        } else if line == "END" {
            Ok(None)
        } else {
            Err(AppError::new(format!("get 失败: {}", line)))
        }
    }

    /// set 单个键
    pub async fn set(&self, key: &str, value: &[u8], flags: u32, exptime: i64) -> AppResult<()> {
        let mut inner = self.inner.lock().await;
        let head = format!("set {} {} {} {}\r\n", key, flags, exptime, value.len());
        let mut out = Vec::with_capacity(head.len() + value.len() + 2);
        out.extend_from_slice(head.as_bytes());
        out.extend_from_slice(value);
        out.extend_from_slice(b"\r\n");
        inner.write_raw(&out).await?;
        let line = inner.read_line().await?;
        match line.as_str() {
            "STORED" => Ok(()),
            "NOT_STORED" => Err(AppError::new("NOT_STORED：存储被拒绝")),
            other => Err(AppError::new(format!("set 失败: {}", other))),
        }
    }

    /// delete 单个键
    pub async fn delete(&self, key: &str) -> AppResult<bool> {
        let mut inner = self.inner.lock().await;
        inner.write_cmd(&format!("delete {}", key)).await?;
        let line = inner.read_line().await?;
        match line.as_str() {
            "DELETED" => Ok(true),
            "NOT_FOUND" => Ok(false),
            other => Err(AppError::new(format!("delete 失败: {}", other))),
        }
    }

    /// touch（设置/更新过期时间，exptime=0 永久）
    pub async fn touch(&self, key: &str, exptime: i64) -> AppResult<bool> {
        let mut inner = self.inner.lock().await;
        inner.write_cmd(&format!("touch {} {}", key, exptime)).await?;
        let line = inner.read_line().await?;
        match line.as_str() {
            "TOUCHED" => Ok(true),
            "NOT_FOUND" => Ok(false),
            other => Err(AppError::new(format!("touch 失败: {}", other))),
        }
    }

    /// flush_all
    pub async fn flush_all(&self) -> AppResult<()> {
        let mut inner = self.inner.lock().await;
        inner.write_cmd("flush_all").await?;
        let line = inner.read_line().await?;
        if line == "OK" {
            Ok(())
        } else {
            Err(AppError::new(format!("flush_all 失败: {}", line)))
        }
    }

    /// version
    pub async fn version(&self) -> AppResult<String> {
        let mut inner = self.inner.lock().await;
        inner.write_cmd("version").await?;
        let line = inner.read_line().await?;
        if let Some(v) = line.strip_prefix("VERSION ") {
            Ok(v.to_string())
        } else {
            Err(AppError::new(format!("version 失败: {}", line)))
        }
    }

    /// stats（多行 STAT）
    pub async fn stats(&self) -> AppResult<Vec<(String, String)>> {
        let mut inner = self.inner.lock().await;
        inner.write_cmd("stats").await?;
        let mut out = Vec::new();
        loop {
            let line = inner.read_line().await?;
            if line == "END" {
                break;
            }
            if let Some(rest) = line.strip_prefix("STAT ") {
                let mut it = rest.splitn(2, ' ');
                let k = it.next().unwrap_or("").to_string();
                let v = it.next().unwrap_or("").to_string();
                out.push((k, v));
            }
        }
        Ok(out)
    }

    /// 枚举所有键（lru_crawler metadump all）
    pub async fn keys(&self) -> AppResult<Vec<MemcachedKeyMeta>> {
        let mut inner = self.inner.lock().await;
        // 主方案：stats items → stats cachedump（遍历每个 slab，limit 0 = 全部）
        let mut out = Vec::new();
        inner.write_cmd("stats items").await?;
        let mut slabs: Vec<u32> = Vec::new();
        loop {
            let line = inner.read_line().await?;
            if line == "END" {
                break;
            }
            // STAT items:<slab>:number <n>
            if let Some(rest) = line.strip_prefix("STAT items:") {
                if let Some(id) = rest.split(':').next().and_then(|s| s.parse::<u32>().ok()) {
                    if !slabs.contains(&id) {
                        slabs.push(id);
                    }
                }
            }
        }
        for slab in slabs {
            inner.write_cmd(&format!("stats cachedump {} 0", slab)).await?;
            loop {
                let line = inner.read_line().await?;
                if line == "END" {
                    break;
                }
                if let Some(rest) = line.strip_prefix("ITEM ") {
                    // ITEM <key> [<size> b; <exp> s]
                    let mut parts = rest.splitn(2, ' ');
                    let key = parts.next().unwrap_or("").to_string();
                    let mut exp: i64 = -1;
                    let mut size: i64 = -1;
                    if let Some(meta) = parts.next() {
                        let m = meta.trim().trim_start_matches('[').trim_end_matches(']');
                        for part in m.split(';') {
                            let t = part.trim();
                            if let Some(v) = t.strip_suffix(" b") {
                                size = v.trim().parse().unwrap_or(-1);
                            } else if let Some(v) = t.strip_suffix(" s") {
                                exp = v.trim().parse().unwrap_or(-1);
                            }
                        }
                    }
                    if !key.is_empty() {
                        out.push(MemcachedKeyMeta { key, exp, size });
                    }
                }
            }
        }
        if !out.is_empty() {
            return Ok(out);
        }
        // 回退方案：lru_crawler metadump all（老版本/禁用 cachedump 时）
        inner.write_cmd("lru_crawler metadump all").await?;
        loop {
            let line = inner.read_line().await?;
            if line == "END" {
                break;
            }
            if let Some(rest) = line.strip_prefix("key=") {
                let mut parts = rest.splitn(2, ' ');
                let key = parts.next().unwrap_or("").to_string();
                let mut exp: i64 = -1;
                let mut size: i64 = -1;
                if let Some(meta) = parts.next() {
                    for part in meta.split(' ') {
                        if let Some(v) = part.strip_prefix("exp=") {
                            exp = v.parse().unwrap_or(-1);
                        } else if let Some(v) = part.strip_prefix("size=") {
                            size = v.parse().unwrap_or(-1);
                        }
                    }
                }
                if !key.is_empty() {
                    out.push(MemcachedKeyMeta { key, exp, size });
                }
            }
        }
        Ok(out)
    }

    /// 发送任意命令并读取完整响应（set 系列自动补全协议参数）
    pub async fn raw(&self, cmd: &str) -> AppResult<String> {
        // memcached 协议：set <key> <flags> <exptime> <bytes>\r\n<data>\r\n
        // 对 set/add/replace/append/prepend 做智能补全，支持两种写法：
        //   1) set <key> <value> [exptime]        —— 简化写法
        //   2) set <key> <flags> <exp> <bytes> <data...> —— 完整协议写法
        let tokens: Vec<&str> = cmd.split_whitespace().collect();
        let mut wire = cmd.to_string();
        let mut data_block: Option<Vec<u8>> = None;
        if tokens.len() >= 3 {
            let verb = tokens[0].to_ascii_lowercase();
            if matches!(verb.as_str(), "set" | "add" | "replace" | "append" | "prepend") {
                let key = tokens[1];
                let rest = &tokens[2..];
                if rest.len() >= 4 && rest[1].parse::<i64>().is_ok() && rest[2].parse::<i64>().is_ok() {
                    // 完整协议：set <key> <flags> <exp> <bytes> <data...>
                    let data = rest[3..].join(" ");
                    wire = format!("{} {} {} {} {}", verb, key, rest[0], rest[1], data.len());
                    data_block = Some(data.into_bytes());
                } else {
                    // 简化写法：set <key> <value> [exptime]
                    let mut exp = "0";
                    let mut value_parts: Vec<&str> = rest.to_vec();
                    if rest.len() >= 2 && rest[rest.len() - 1].parse::<i64>().is_ok() {
                        exp = rest[rest.len() - 1];
                        value_parts = rest[..rest.len() - 1].to_vec();
                    }
                    let value = value_parts.join(" ");
                    wire = format!("{} {} 0 {} {}", verb, key, exp, value.len());
                    data_block = Some(value.into_bytes());
                }
            }
        }
        let mut inner = self.inner.lock().await;
        if let Some(data) = data_block {
            inner.write_raw(format!("{}\r\n", wire).as_bytes()).await?;
            inner.write_raw(&data).await?;
            inner.write_raw(b"\r\n").await?;
        } else {
            inner.write_cmd(&wire).await?;
        }
        let first = inner.read_line().await?;
        if first.starts_with("VALUE ") {
            // get/gets 多行
            let mut out = first.clone();
            loop {
                let line = inner.read_line().await?;
                if line == "END" {
                    out.push_str("\nEND");
                    break;
                }
                let parts: Vec<&str> = line.split(' ').collect();
                if parts.len() >= 4 && parts[0] == "VALUE" {
                    let bytes = parts[3].parse().unwrap_or(0);
                    let data = inner.read_bytes(bytes).await?;
                    let _ = inner.read_line().await;
                    out.push_str(&format!(
                        "\nVALUE {} {} {}\n{}",
                        parts[1],
                        parts[2],
                        parts[3],
                        String::from_utf8_lossy(&data)
                    ));
                } else {
                    out.push_str(&format!("\n{}", line));
                }
            }
            Ok(out)
        } else if first.starts_with("STAT ") || first.starts_with("key=") {
            let mut out = first.clone();
            loop {
                let line = inner.read_line().await?;
                if line == "END" {
                    out.push_str("\nEND");
                    break;
                }
                out.push_str(&format!("\n{}", line));
            }
            Ok(out)
        } else {
            Ok(first)
        }
    }
}

// ==================== 连接管理 ====================

pub struct MemcachedConnState {
    pub config: ConnConfig,
    pub client: MemcachedClient,
    pub status: AtomicU8,
    pub error: RwLock<Option<String>>,
    pub connected_at: AtomicU64,
}

impl MemcachedConnState {
    pub fn status(&self) -> ConnStatus {
        match self.status.load(Ordering::Relaxed) {
            STATUS_CONNECTING => ConnStatus::Connecting,
            STATUS_CONNECTED => ConnStatus::Connected,
            STATUS_ERROR => ConnStatus::Error,
            _ => ConnStatus::Disconnected,
        }
    }

    pub fn to_status_info(&self) -> ConnStatusInfo {
        ConnStatusInfo {
            id: self.config.id.clone(),
            name: self.config.name.clone(),
            mode: ConnMode::Memcached,
            status: self.status(),
            error: self.error.read().clone(),
            nodes: Vec::new(),
            databases: None,
            connected_at: Some(self.connected_at.load(Ordering::Relaxed)),
        }
    }
}

/// Memcached 连接管理器
pub struct MemcachedManager {
    pub conns: Mutex<HashMap<String, Arc<MemcachedConnState>>>,
}

impl Default for MemcachedManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MemcachedManager {
    pub fn new() -> Self {
        Self {
            conns: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, conn_id: &str) -> AppResult<Arc<MemcachedConnState>> {
        self.conns
            .lock()
            .get(conn_id)
            .cloned()
            .ok_or_else(|| AppError::new(format!("连接不存在或未建立: {}", conn_id)))
    }

    pub fn has(&self, conn_id: &str) -> bool {
        self.conns.lock().contains_key(conn_id)
    }

    /// 测试连接（建立→version→关闭）
    pub async fn test(&self, config: &ConnConfig) -> AppResult<String> {
        let client = MemcachedClient::connect(&config.host, config.port, config.connect_timeout_ms)
            .await?;
        let ver = client.version().await?;
        Ok(format!("Memcached {}", ver))
    }

    pub async fn connect(&self, config: ConnConfig) -> AppResult<String> {
        let id = config.id.clone();
        let state = self.conns.lock().get(&id).cloned();
        if let Some(st) = state {
            if st.status() == ConnStatus::Connected {
                return Ok(id);
            }
        }
        let client = MemcachedClient::connect(&config.host, config.port, config.connect_timeout_ms)
            .await?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let st = Arc::new(MemcachedConnState {
            config,
            client,
            status: AtomicU8::new(STATUS_CONNECTED),
            error: RwLock::new(None),
            connected_at: AtomicU64::new(now),
        });
        self.conns.lock().insert(id.clone(), st);
        Ok(id)
    }

    pub async fn disconnect(&self, conn_id: &str) {
        self.conns.lock().remove(conn_id);
    }

    pub fn disconnect_all(&self) {
        self.conns.lock().clear();
    }

    pub fn list_status(&self) -> Vec<ConnStatusInfo> {
        self.conns
            .lock()
            .values()
            .map(|c| c.to_status_info())
            .collect()
    }
}

// ==================== 数据操作（供 commands.rs 分派） ====================

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// glob 匹配（* ? []），用于键名过滤
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    fn inner(p: &[char], t: &[char]) -> bool {
        if p.is_empty() {
            return t.is_empty();
        }
        match p[0] {
            '*' => {
                inner(&p[1..], t) || (!t.is_empty() && inner(p, &t[1..]))
            }
            '?' => !t.is_empty() && inner(&p[1..], &t[1..]),
            '[' => {
                if t.is_empty() {
                    return false;
                }
                let mut i = 1;
                let mut negate = false;
                if i < p.len() && (p[i] == '!' || p[i] == '^') {
                    negate = true;
                    i += 1;
                }
                let mut matched = false;
                let mut first = true;
                let mut last = '\0';
                while i < p.len() && p[i] != ']' {
                    if !first && p[i] == '-' && i + 1 < p.len() && p[i + 1] != ']' {
                        matched |= t[0] >= last && t[0] <= p[i + 1];
                        i += 2;
                    } else {
                        matched |= t[0] == p[i];
                        last = p[i];
                        i += 1;
                    }
                    first = false;
                }
                if negate {
                    matched = !matched;
                }
                matched && inner(&p[(i + 1).min(p.len())..], &t[1..])
            }
            c => !t.is_empty() && t[0] == c && inner(&p[1..], &t[1..]),
        }
    }
    inner(&p, &t)
}

/// 扫描键（memcached）：metadump 全量 + glob 过滤
pub async fn scan_keys(
    manager: &MemcachedManager,
    conn_id: &str,
    pattern: &str,
) -> AppResult<ScanPage> {
    let st = manager.get(conn_id)?;
    let metas = st.client.keys().await?;
    let keys: Vec<String> = metas
        .into_iter()
        .map(|m| m.key)
        .filter(|k| glob_match(pattern, k))
        .collect();
    Ok(ScanPage {
        cursor: "0".into(),
        keys,
        truncated: false,
        types: HashMap::new(),
    })
}

/// 键信息
pub async fn key_info(
    manager: &MemcachedManager,
    conn_id: &str,
    key: &str,
) -> AppResult<KeyInfo> {
    let st = manager.get(conn_id)?;
    let metas = st.client.keys().await?;
    let meta = metas.iter().find(|m| m.key == key);
    let (ttl, size) = match meta {
        Some(m) => {
            let ttl = if m.exp <= 0 { -1 } else { m.exp - now_secs() };
            (ttl, m.size)
        }
        None => (-1, -1),
    };
    Ok(KeyInfo {
        key: key.to_string(),
        r#type: "string".into(),
        ttl,
        pttl: if ttl < 0 { -1 } else { ttl * 1000 },
        length: -1,
        memory: if size >= 0 { Some(size) } else { None },
        encoding: Some("memcached".into()),
    })
}

/// 读取值
pub async fn get_value(
    manager: &MemcachedManager,
    conn_id: &str,
    key: &str,
) -> AppResult<ValueView> {
    let st = manager.get(conn_id)?;
    let val = st.client.get(key).await?;
    let (ttl, _size) = {
        let metas = st.client.keys().await?;
        match metas.iter().find(|m| m.key == key) {
            Some(m) => {
                let ttl = if m.exp <= 0 { -1 } else { m.exp - now_secs() };
                (ttl, m.size)
            }
            None => (-1, -1),
        }
    };
    match val {
        Some(v) => Ok(ValueView {
            key: key.to_string(),
            r#type: "string".into(),
            ttl,
            length: v.data.len() as i64,
            payload: ValuePayload::String {
                value: EncodedValue::from_bytes(&v.data),
            },
        }),
        None => Err(AppError::new(format!("键不存在: {}", key))),
    }
}

/// 写入字符串值
pub async fn set_string_value(
    manager: &MemcachedManager,
    conn_id: &str,
    key: &str,
    value: &[u8],
    ttl: i64,
) -> AppResult<()> {
    let st = manager.get(conn_id)?;
    let exptime = if ttl < 0 { 0 } else { now_secs() + ttl };
    st.client.set(key, value, 0, exptime).await
}

/// 删除键
pub async fn delete_keys(
    manager: &MemcachedManager,
    conn_id: &str,
    keys: &[String],
) -> AppResult<u64> {
    let st = manager.get(conn_id)?;
    let mut n = 0u64;
    for k in keys {
        if st.client.delete(k).await? {
            n += 1;
        }
    }
    Ok(n)
}

/// 设置 TTL（touch）
pub async fn set_ttl(
    manager: &MemcachedManager,
    conn_id: &str,
    key: &str,
    ttl: i64,
) -> AppResult<bool> {
    let st = manager.get(conn_id)?;
    let exptime = if ttl < 0 { 0 } else { now_secs() + ttl };
    st.client.touch(key, exptime).await
}

/// 新建键（set）
pub async fn create_key(
    manager: &MemcachedManager,
    conn_id: &str,
    key: &str,
    value: &[u8],
    ttl: i64,
) -> AppResult<()> {
    let st = manager.get(conn_id)?;
    if st.client.get(key).await?.is_some() {
        return Err(AppError::new(format!("键已存在: {}", key)));
    }
    let exptime = if ttl < 0 { 0 } else { now_secs() + ttl };
    st.client.set(key, value, 0, exptime).await
}

/// flush
pub async fn flush_db(manager: &MemcachedManager, conn_id: &str) -> AppResult<()> {
    let st = manager.get(conn_id)?;
    st.client.flush_all().await
}

/// 服务信息（stats）
pub async fn server_info(manager: &MemcachedManager, conn_id: &str) -> AppResult<ServerInfo> {
    let st = manager.get(conn_id)?;
    let stats = st.client.stats().await?;
    let mut raw = String::new();
    for (k, v) in &stats {
        raw.push_str(&format!("STAT {} {}\r\n", k, v));
    }
    raw.push_str("END\r\n");
    Ok(ServerInfo {
        sections: vec![InfoSection {
            name: "stats".into(),
            fields: stats,
        }],
        raw,
    })
}

/// 执行原始命令
pub async fn execute_command(
    manager: &MemcachedManager,
    conn_id: &str,
    command: String,
) -> AppResult<crate::redisx::console::CommandResult> {
    let st = manager.get(conn_id)?;
    let started = std::time::Instant::now();
    let result = st.client.raw(&command).await?;
    let ok = !result.starts_with("ERROR") && !result.starts_with("CLIENT_ERROR")
        && !result.starts_with("SERVER_ERROR");
    Ok(crate::redisx::console::CommandResult {
        command,
        elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
        ok,
        error: if ok { None } else { Some(result.clone()) },
        value: None,
        text: Some(result),
    })
}
