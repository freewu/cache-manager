use serde::{Deserialize, Serialize};

/// 一个节点地址（host:port）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeSpec {
    pub host: String,
    pub port: u16,
}

impl std::fmt::Display for NodeSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

/// 连接模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConnMode {
    /// 单机
    Single,
    /// 主从（读写分离）
    MasterSlave,
    /// Sentinel 哨兵
    Sentinel,
    /// Cluster 集群
    Cluster,
    /// Memcached
    Memcached,
}

/// 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnConfig {
    pub id: String,
    pub name: String,
    pub mode: ConnMode,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database: Option<i64>,
    /// 附加节点：
    /// - MasterSlave: 从库列表
    /// - Sentinel: 其他哨兵节点
    /// - Cluster: 其他种子节点
    pub nodes: Vec<NodeSpec>,
    /// Sentinel 主节点服务名（master name）
    pub service_name: Option<String>,
    pub tls: bool,
    /// 连接超时（毫秒）
    pub connect_timeout_ms: u64,
}

impl Default for ConnConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            mode: ConnMode::Single,
            host: "127.0.0.1".into(),
            port: 6379,
            username: None,
            password: None,
            database: Some(0),
            nodes: Vec::new(),
            service_name: None,
            tls: false,
            connect_timeout_ms: 10_000,
        }
    }
}

impl ConnConfig {
    /// 生成展示用 URL（密码脱敏）
    pub fn display_url(&self) -> String {
        let scheme = match (self.mode, self.tls) {
            (ConnMode::Cluster, true) => "rediss-cluster",
            (ConnMode::Cluster, false) => "redis-cluster",
            (ConnMode::Sentinel, true) => "rediss-sentinel",
            (ConnMode::Sentinel, false) => "redis-sentinel",
            (ConnMode::Memcached, _) => "memcached",
            (_, true) => "rediss",
            (_, false) => "redis",
        };
        let mut url = format!("{}://", scheme);
        if self.mode != ConnMode::Memcached {
            if self.username.is_some() || self.password.is_some() {
                url.push_str("***@");
            }
        }
        url.push_str(&format!("{}:{}", self.host, self.port));
        if self.mode != ConnMode::Memcached {
            if let Some(db) = self.database {
                url.push_str(&format!("/{}", db));
            }
            for node in &self.nodes {
                url.push_str(&format!(",{}", node));
            }
            if self.mode == ConnMode::Sentinel {
                if let Some(sn) = &self.service_name {
                    url.push_str(&format!(" (sentinel: {})", sn));
                }
            }
        }
        url
    }
}

/// 连接状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConnStatus {
    /// 未连接
    Disconnected,
    /// 连接中
    Connecting,
    /// 已连接
    Connected,
    /// 连接失败 / 出错
    Error,
}

/// 连接信息（返回给前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnStatusInfo {
    pub id: String,
    pub name: String,
    pub mode: ConnMode,
    pub status: ConnStatus,
    pub error: Option<String>,
    /// 主从模式下各节点的角色信息
    pub nodes: Vec<NodeStatus>,
    /// 单机 / 主从 / 哨兵模式下的数据库数量（从 info keyspace 解析）
    pub databases: Option<Vec<i64>>,
    pub connected_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeStatus {
    pub host: String,
    pub port: u16,
    /// master / replica / sentinel
    pub role: String,
    /// link status: up / down
    pub status: String,
    /// 复制偏移量等附加信息
    pub extra: Option<String>,
}

/// 编码后的值（支持二进制安全）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedValue {
    /// "utf8" | "base64"
    pub encoding: String,
    pub value: String,
}

impl EncodedValue {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        match std::str::from_utf8(bytes) {
            Ok(s) => Self {
                encoding: "utf8".into(),
                value: s.to_string(),
            },
            Err(_) => Self {
                encoding: "base64".into(),
                value: base64_encode(bytes),
            },
        }
    }

    pub fn from_string(s: String) -> Self {
        Self {
            encoding: "utf8".into(),
            value: s,
        }
    }
}

pub fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

pub fn base64_decode(s: &str) -> Result<Vec<u8>, AppError> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| AppError::new(format!("Base64 解码失败: {}", e)))
}

use crate::error::AppError;
