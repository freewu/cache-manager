pub mod console;
pub mod keys;
pub mod server;
pub mod stream;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use fred::prelude::*;
use fred::types::scan::ScanType;
use parking_lot::{Mutex, RwLock};

use crate::error::{AppError, AppResult};
use crate::model::{ConnConfig, ConnMode, ConnStatus, ConnStatusInfo, NodeSpec};

pub const STATUS_CONNECTING: u8 = 1;
pub const STATUS_CONNECTED: u8 = 2;
pub const STATUS_ERROR: u8 = 3;

/// 一个已建立的连接
pub struct ConnHandle {
    pub config: ConnConfig,
    /// 主池（单机 / 主 / 哨兵主节点 / 集群）
    pub pool: Pool,
    /// 主从模式下的从库池
    pub replicas: Vec<Pool>,
    pub status: AtomicU8,
    pub error: RwLock<Option<String>>,
    pub connected_at: AtomicU64,
    /// 订阅 / 监控后台任务
    pub sub_tasks: Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

impl ConnHandle {
    pub fn status(&self) -> ConnStatus {
        match self.status.load(Ordering::Relaxed) {
            STATUS_CONNECTING => ConnStatus::Connecting,
            STATUS_CONNECTED => ConnStatus::Connected,
            STATUS_ERROR => ConnStatus::Error,
            _ => ConnStatus::Disconnected,
        }
    }

    pub fn error_message(&self) -> Option<String> {
        self.error.read().clone()
    }
}

/// 连接管理器
pub struct RedisManager {
    pub conns: Mutex<HashMap<String, Arc<ConnHandle>>>,
}

impl Default for RedisManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RedisManager {
    pub fn new() -> Self {
        Self {
            conns: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, id: &str) -> AppResult<Arc<ConnHandle>> {
        self.conns
            .lock()
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::new("连接不存在，请先建立连接"))
    }

    pub fn is_connected(&self, id: &str) -> bool {
        #[allow(dead_code)]
        {
            let _ = id;
        }
        self.conns
            .lock()
            .get(id)
            .map(|h| h.status.load(Ordering::Relaxed) == STATUS_CONNECTED)
            .unwrap_or(false)
    }

    pub fn list_status(&self) -> Vec<ConnStatusInfo> {
        let conns = self.conns.lock();
        conns.values().map(|h| h.to_status_info()).collect()
    }

    /// 建立连接（会替换同名 id 的旧连接）
    pub async fn connect(&self, config: ConnConfig) -> AppResult<String> {
        let id = config.id.clone();
        if id.is_empty() {
            return Err(AppError::new("连接 id 不能为空"));
        }

        // 断开旧的（避免在持有锁的情况下 await）
        let old = self.conns.lock().remove(&id);
        if let Some(old) = old {
            let _ = old.pool.quit().await;
            for r in &old.replicas {
                let _ = r.quit().await;
            }
        }

        let handle = Self::create_handle(config).await?;
        self.conns.lock().insert(id.clone(), Arc::new(handle));
        Ok(id)
    }

    async fn create_handle(config: ConnConfig) -> AppResult<ConnHandle> {
        let pool = Self::init_pool(&config, None).await.map_err(|e| {
            config_error(&config, e)
        })?;

        let mut replicas = Vec::new();
        if config.mode == ConnMode::MasterSlave {
            for node in &config.nodes {
                match Self::init_pool(&config, Some(node)).await {
                    Ok(p) => replicas.push(p),
                    Err(e) => {
                        let _ = pool.quit().await;
                        return Err(config_error(&config, e));
                    }
                }
            }
        }

        let connected_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Ok(ConnHandle {
            config,
            pool,
            replicas,
            status: AtomicU8::new(STATUS_CONNECTED),
            error: RwLock::new(None),
            connected_at: AtomicU64::new(connected_at),
            sub_tasks: Mutex::new(Vec::new()),
        })
    }

    async fn init_pool(base: &ConnConfig, replica: Option<&NodeSpec>) -> AppResult<Pool> {
        let config = match replica {
            Some(node) => build_replica_config(base, node)?,
            None => build_fred_config(base)?,
        };
        // 主池使用 2 个连接：一个用于普通命令，一个用于 Pub/Sub（订阅模式不允许执行其他命令）
        let pool_size = if replica.is_some() { 1 } else { 2 };

        let connection = ConnectionConfig {
            connection_timeout: Duration::from_millis(base.connect_timeout_ms.max(1000)),
            internal_command_timeout: Duration::from_secs(30),
            ..Default::default()
        };

        let pool = Pool::new(config, None, Some(connection), None, pool_size)
            .map_err(|e| AppError::new(format!("创建连接失败: {}", e)))?;

        tokio::time::timeout(
            Duration::from_millis(base.connect_timeout_ms.max(1000) + 2000),
            pool.init(),
        )
        .await
        .map_err(|_| AppError::new("连接超时"))??;

        // 验证可用
        let pong: String = pool.ping(None).await.map_err(|e| {
            AppError::new(format!("连接失败: {}", e))
        })?;
        if pong != "PONG" {
            let _ = pool.quit().await;
            return Err(AppError::new("连接验证失败 (PING 无响应)"));
        }

        Ok(pool)
    }

    /// 断开连接
    pub async fn disconnect(&self, id: &str) -> AppResult<()> {
        let handle = self.conns.lock().remove(id);
        if let Some(handle) = handle {
            stream::stop_all(&handle).await;
            let _ = handle.pool.quit().await;
            for r in &handle.replicas {
                let _ = r.quit().await;
            }
        }
        Ok(())
    }

    /// 断开全部
    pub async fn disconnect_all(&self) {
        let conns: Vec<Arc<ConnHandle>> = self.conns.lock().values().cloned().collect();
        for handle in conns {
            stream::stop_all(&handle).await;
            let _ = handle.pool.quit().await;
        }
        self.conns.lock().clear();
    }

    /// 测试连接（不保存）
    pub async fn test(config: &ConnConfig) -> AppResult<String> {
        let pool = Self::init_pool(config, None).await?;
        let pong: String = pool.ping(None).await?;
        let _ = pool.quit().await;
        Ok(pong)
    }

    /// 选择一个可执行命令的池：
    /// - replica = None: 主池
    /// - replica = Some(i): 第 i 个从库（仅主从模式）
    pub fn pool_for<'a>(handle: &'a ConnHandle, replica: Option<usize>) -> AppResult<&'a Pool> {
        if let Some(idx) = replica {
            if handle.replicas.is_empty() {
                return Err(AppError::new("当前连接没有配置从库"));
            }
            let pool = handle
                .replicas
                .get(idx)
                .ok_or_else(|| AppError::new(format!("从库索引 {} 无效", idx)))?;
            Ok(pool)
        } else {
            Ok(&handle.pool)
        }
    }

    /// 集群模式下，对所有节点执行 SCAN 并合并结果
    pub async fn cluster_scan(
        handle: &ConnHandle,
        cursor: &str,
        pattern: &str,
        count: Option<u32>,
        r#type: Option<ScanType>,
    ) -> AppResult<(String, Vec<String>)> {
        let servers = handle.pool.active_connections();
        if servers.is_empty() {
            return Err(AppError::new("集群中没有可用节点"));
        }

        let client = handle.pool.clients()[0].clone();
        let mut all_keys: Vec<String> = Vec::new();
        let mut max_cursor = String::from("0");

        for server in servers {
            let node_client = client.with_cluster_node(server.clone());
            let (next_cursor, keys) = node_client
                .scan_page::<(String, Vec<String>), _, _>(cursor, pattern, count, r#type.clone())
                .await
                .map_err(|e| AppError::new(format!("节点 {} 扫描失败: {}", server, e)))?;
            if next_cursor != "0" && next_cursor > max_cursor {
                max_cursor = next_cursor.clone();
            }
            all_keys.extend(keys);
        }

        Ok((max_cursor, all_keys))
    }
}

impl ConnHandle {
    pub fn to_status_info(&self) -> ConnStatusInfo {
        ConnStatusInfo {
            id: self.config.id.clone(),
            name: self.config.name.clone(),
            mode: self.config.mode,
            status: self.status(),
            error: self.error_message(),
            nodes: Vec::new(),
            databases: None,
            connected_at: Some(self.connected_at.load(Ordering::Relaxed)),
        }
    }
}

fn config_error(config: &ConnConfig, e: AppError) -> AppError {
    AppError::new(format!("连接 {} ({}) 失败: {}", config.name, config.display_url(), e))
}

/// 根据模式构建 fred Config
pub fn build_fred_config(cfg: &ConnConfig) -> AppResult<Config> {
    let url = build_url(cfg)?;
    Config::from_url(&url).map_err(|e| AppError::new(format!("配置解析失败: {}", e)))
}

/// 主从模式下从库的 Config
pub fn build_replica_config(master: &ConnConfig, replica: &NodeSpec) -> AppResult<Config> {
    let mut cfg = master.clone();
    cfg.host = replica.host.clone();
    cfg.port = replica.port;
    build_fred_config(&cfg)
}

fn build_url(cfg: &ConnConfig) -> AppResult<String> {
    let scheme = match (cfg.mode, cfg.tls) {
        (ConnMode::Cluster, true) => "rediss-cluster",
        (ConnMode::Cluster, false) => "redis-cluster",
        (ConnMode::Sentinel, true) => "rediss-sentinel",
        (ConnMode::Sentinel, false) => "redis-sentinel",
        (_, true) => "rediss",
        (_, false) => "redis",
    };

    let mut url = format!("{}://", scheme);
    let has_user = cfg.username.as_deref().map(|s| !s.is_empty()).unwrap_or(false);
    let has_pass = cfg.password.as_deref().map(|s| !s.is_empty()).unwrap_or(false);
    if has_user || has_pass {
        if has_user {
            url.push_str(cfg.username.as_deref().unwrap_or(""));
        }
        if has_pass {
            url.push(':');
            url.push_str(cfg.password.as_deref().unwrap_or(""));
        }
        url.push('@');
    }

    url.push_str(&format!("{}:{}", cfg.host, cfg.port));

    // 单机 / 主从 支持 db
    if cfg.mode != ConnMode::Cluster {
        if let Some(db) = cfg.database {
            url.push_str(&format!("/{}", db));
        }
    }

    // 附加节点
    let extra_nodes: Vec<&NodeSpec> = match cfg.mode {
        // 主从的从库不放在主 URL 里，单独建池
        ConnMode::MasterSlave => vec![],
        _ => cfg.nodes.iter().collect(),
    };
    if !extra_nodes.is_empty() {
        url.push('?');
        let mut first = true;
        for node in extra_nodes {
            if !first {
                url.push('&');
            }
            url.push_str(&format!("node={}", node));
            first = false;
        }
        if cfg.mode == ConnMode::Sentinel {
            let service = cfg
                .service_name
                .as_deref()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| AppError::new("Sentinel 模式必须填写主节点服务名 (master name)"))?;
            url.push_str(&format!("&sentinelServiceName={}", service));
        }
    } else if cfg.mode == ConnMode::Sentinel {
        let service = cfg
            .service_name
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| AppError::new("Sentinel 模式必须填写主节点服务名 (master name)"))?;
        url.push_str(&format!("?sentinelServiceName={}", service));
    }

    Ok(url)
}

/// 值转 JSON（二进制安全）
pub fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Integer(i) => serde_json::json!(i),
        Value::Double(d) => serde_json::json!(d),
        Value::String(s) => serde_json::Value::String(s.to_string()),
        Value::Bytes(b) => serde_json::Value::String(display_bytes(b.as_ref()).value),
        Value::Null => serde_json::Value::Null,
        Value::Queued => serde_json::Value::String("QUEUED".into()),
        Value::Map(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map.iter() {
                obj.insert(display_bytes(k.as_bytes()).value, value_to_json(v));
            }
            serde_json::Value::Object(obj)
        }
        Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(value_to_json).collect())
        }
    }
}

/// 字节转展示字符串
pub fn display_bytes(bytes: &[u8]) -> crate::model::EncodedValue {
    crate::model::EncodedValue::from_bytes(bytes)
}
