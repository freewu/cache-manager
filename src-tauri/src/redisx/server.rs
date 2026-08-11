use fred::prelude::*;
use fred::types::CustomCommand;
use fred::types::InfoKind;
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::model::{ConnMode, NodeStatus};
use crate::redisx::{display_bytes, value_to_json, ConnHandle, RedisManager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfoSection {
    pub name: String,
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub sections: Vec<InfoSection>,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientEntry {
    pub id: String,
    pub addr: String,
    pub laddr: String,
    pub fd: String,
    pub name: String,
    pub age: String,
    pub idle: String,
    pub flags: String,
    pub db: String,
    pub sub: String,
    pub psub: String,
    pub multi: String,
    pub watch: String,
    pub qbuf: String,
    pub qbuf_free: String,
    pub argv_mem: String,
    pub multi_mem: String,
    pub tot_mem: String,
    pub rbuf: String,
    pub rbuf_alloc: String,
    pub obl: String,
    pub oll: String,
    pub omem: String,
    pub events: String,
    pub cmd: String,
    pub user: String,
    pub redir: String,
    pub resp: String,
    pub lib_name: String,
    pub lib_ver: String,
    pub other: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlowlogEntry {
    pub id: i64,
    pub timestamp: i64,
    pub duration_us: i64,
    pub args: Vec<String>,
    pub client_addr: String,
    pub client_name: String,
}

/// 获取 INFO 信息，解析为分节结构
pub async fn get_info(
    manager: &RedisManager,
    conn_id: &str,
    section: Option<String>,
) -> AppResult<ServerInfo> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;

    let kind = match section.as_deref() {
        None | Some("") => None,
        Some(s) => Some(parse_info_kind(s)?),
    };
    let raw: String = pool.info(kind).await?;
    Ok(ServerInfo {
        sections: parse_info(&raw),
        raw,
    })
}

fn parse_info_kind(s: &str) -> AppResult<InfoKind> {
    Ok(match s.to_lowercase().as_str() {
        "default" => InfoKind::Default,
        "all" => InfoKind::All,
        "keyspace" => InfoKind::Keyspace,
        "cluster" => InfoKind::Cluster,
        "commandstats" => InfoKind::CommandStats,
        "cpu" => InfoKind::Cpu,
        "replication" => InfoKind::Replication,
        "stats" => InfoKind::Stats,
        "persistence" => InfoKind::Persistence,
        "memory" => InfoKind::Memory,
        "clients" => InfoKind::Clients,
        "server" => InfoKind::Server,
        other => return Err(AppError::new(format!("未知 INFO 段: {}", other))),
    })
}

pub fn parse_info(raw: &str) -> Vec<InfoSection> {
    let mut sections = Vec::new();
    let mut current: Option<InfoSection> = None;

    for line in raw.lines() {
        let line = line.trim_end();
        if line.starts_with('#') {
            if let Some(sec) = current.take() {
                sections.push(sec);
            }
            current = Some(InfoSection {
                name: line[1..].trim().to_string(),
                fields: Vec::new(),
            });
        } else if line.is_empty() {
            if let Some(sec) = current.take() {
                sections.push(sec);
            }
        } else if let Some((k, v)) = line.split_once(':') {
            if let Some(sec) = current.as_mut() {
                sec.fields.push((k.trim().to_string(), v.to_string()));
            }
        }
    }
    if let Some(sec) = current {
        sections.push(sec);
    }
    sections
}

/// 获取 CONFIG GET
pub async fn get_config(
    manager: &RedisManager,
    conn_id: &str,
    pattern: &str,
) -> AppResult<Vec<(String, String)>> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    let map: Vec<(String, String)> = pool.config_get(pattern).await?;
    Ok(map)
}

/// 修改 CONFIG SET
pub async fn set_config(
    manager: &RedisManager,
    conn_id: &str,
    key: &str,
    value: &str,
) -> AppResult<()> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    pool.config_set(key, value).await?;
    Ok(())
}

/// 获取 CLIENT LIST
pub async fn get_clients(
    manager: &RedisManager,
    conn_id: &str,
) -> AppResult<Vec<ClientEntry>> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    let raw: String = pool
        .client_list::<String, Option<Vec<String>>>(None, None)
        .await?;

    let mut out = Vec::new();
    for line in raw.lines() {
        let mut fields: Vec<(String, String)> = Vec::new();
        for part in line.split_whitespace() {
            if let Some((k, v)) = part.split_once('=') {
                fields.push((k.to_string(), v.to_string()));
            }
        }
        let get = |k: &str| -> String {
            fields
                .iter()
                .find(|(fk, _)| fk == k)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        out.push(ClientEntry {
            id: get("id"),
            addr: get("addr"),
            laddr: get("laddr"),
            fd: get("fd"),
            name: get("name"),
            age: get("age"),
            idle: get("idle"),
            flags: get("flags"),
            db: get("db"),
            sub: get("sub"),
            psub: get("psub"),
            multi: get("multi"),
            watch: get("watch"),
            qbuf: get("qbuf"),
            qbuf_free: get("qbuf_free"),
            argv_mem: get("argv-mem"),
            multi_mem: get("multi-mem"),
            tot_mem: get("tot-mem"),
            rbuf: get("rbuf"),
            rbuf_alloc: get("rbuf-alloc"),
            obl: get("obl"),
            oll: get("oll"),
            omem: get("omem"),
            events: get("events"),
            cmd: get("cmd"),
            user: get("user"),
            redir: get("redir"),
            resp: get("resp"),
            lib_name: get("lib-name"),
            lib_ver: get("lib-ver"),
            other: fields
                .into_iter()
                .filter(|(k, _)| !CLIENT_KNOWN_FIELDS.contains(&k.as_str()))
                .collect(),
        });
    }
    Ok(out)
}

const CLIENT_KNOWN_FIELDS: &[&str] = &[
    "id", "addr", "laddr", "fd", "name", "age", "idle", "flags", "db", "sub", "psub", "multi",
    "watch", "qbuf", "qbuf-free", "argv-mem", "multi-mem", "tot-mem", "rbuf", "rbuf-alloc",
    "obl", "oll", "omem", "events", "cmd", "user", "redir", "resp", "lib-name", "lib-ver",
];

/// 获取 SLOWLOG
pub async fn get_slowlog(
    manager: &RedisManager,
    conn_id: &str,
    count: Option<i64>,
) -> AppResult<Vec<SlowlogEntry>> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    let value = pool.slowlog_get::<Value>(count).await?;

    let mut out = Vec::new();
    if let Value::Array(entries) = value {
        for entry in entries {
            if let Value::Array(items) = entry {
                let idx = |i: usize| items.get(i).cloned().unwrap_or(Value::Null);
                let args = match idx(3) {
                    Value::Array(args) => args
                        .iter()
                        .map(|a| display_bytes(&value_bytes(a)).value)
                        .collect(),
                    _ => Vec::new(),
                };
                out.push(SlowlogEntry {
                    id: int_of(&idx(0)),
                    timestamp: int_of(&idx(1)),
                    duration_us: int_of(&idx(2)),
                    args,
                    client_addr: display_bytes(&value_bytes(&idx(4))).value,
                    client_name: display_bytes(&value_bytes(&idx(5))).value,
                });
            }
        }
    }
    Ok(out)
}

fn value_bytes(v: &Value) -> Vec<u8> {
    match v {
        Value::String(s) => s.as_bytes().to_vec(),
        Value::Bytes(b) => b.as_ref().to_vec(),
        Value::Integer(i) => i.to_string().into_bytes(),
        Value::Double(d) => d.to_string().into_bytes(),
        Value::Boolean(b) => b.to_string().into_bytes(),
        _ => Vec::new(),
    }
}

fn int_of(v: &Value) -> i64 {
    match v {
        Value::Integer(i) => *i,
        Value::String(s) => s.parse().unwrap_or(0),
        _ => 0,
    }
}

/// 获取拓扑节点列表（主从 / 集群 / 哨兵）
pub async fn get_topology(
    manager: &RedisManager,
    conn_id: &str,
) -> AppResult<Vec<NodeStatus>> {
    let handle = manager.get(conn_id)?;
    let mode = handle.config.mode;

    match mode {
        ConnMode::Single | ConnMode::MasterSlave => {
            topology_from_replication(&handle).await
        }
        ConnMode::Cluster => topology_from_cluster(&handle).await,
        ConnMode::Sentinel => topology_from_sentinel(&handle).await,
        ConnMode::Memcached => Err(AppError::new("Memcached 不支持拓扑查看")),
    }
}

async fn topology_from_replication(handle: &ConnHandle) -> AppResult<Vec<NodeStatus>> {
    let pool = RedisManager::pool_for(handle, None)?;
    let raw: String = pool.info(Some(InfoKind::Replication)).await?;
    let sections = parse_info(&raw);
    let fields: Vec<(String, String)> = sections
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case("Replication"))
        .map(|s| s.fields.clone())
        .unwrap_or_default();

    let get = |k: &str| -> String {
        fields
            .iter()
            .find(|(fk, _)| fk == k)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };

    let mut nodes = Vec::new();
    let role = get("role");
    let master_host = get("master_host");
    let master_port = get("master_port");
    let master_link = get("master_link_status");

    if role == "slave" || role == "replica" {
        nodes.push(NodeStatus {
            host: master_host.clone(),
            port: master_port.parse().unwrap_or(0),
            role: "master".into(),
            status: if master_link == "up" { "up".into() } else { "down".into() },
            extra: Some(format!("master_link_status={}", master_link)),
        });
    } else if role == "master" {
        nodes.push(NodeStatus {
            host: handle.config.host.clone(),
            port: handle.config.port,
            role: "master".into(),
            status: "up".into(),
            extra: None,
        });
    }

    // 从库列表 slave0/slave1...
    let mut idx = 0;
    loop {
        let key = format!("slave{}", idx);
        let value = get(&key);
        if value.is_empty() {
            break;
        }
        // ip=...,port=...,state=online,offset=...,lag=...
        let mut host = String::new();
        let mut port = 0u16;
        let mut state = String::from("unknown");
        for kv in value.split(',') {
            if let Some((k, v)) = kv.split_once('=') {
                match k {
                    "ip" => host = v.to_string(),
                    "port" => port = v.parse().unwrap_or(0),
                    "state" => state = v.to_string(),
                    _ => {}
                }
            }
        }
        nodes.push(NodeStatus {
            host,
            port,
            role: "replica".into(),
            status: if state == "online" { "up".into() } else { state.clone() },
            extra: Some(value),
        });
        idx += 1;
    }

    Ok(nodes)
}

async fn topology_from_cluster(handle: &ConnHandle) -> AppResult<Vec<NodeStatus>> {
    let pool = RedisManager::pool_for(handle, None)?;
    // CLUSTER NODES
    let value = pool.custom_raw(
        CustomCommand::new_static("CLUSTER", None::<u16>, false),
        vec!["NODES"],
    ).await.map_err(|e| AppError::new(format!("CLUSTER NODES 失败: {}", e)))?;
    let value = Value::try_from(value).map_err(|e| AppError::new(e.to_string()))?;

    let mut nodes = Vec::new();
    if let Value::String(s) = &value {
        for line in s.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 8 {
                continue;
            }
            // parts[1] = host:port, parts[2] = flags
            let hostport = parts[1];
            let flags = parts[2];
            let mut host = hostport.to_string();
            let mut port = 0u16;
            if let Some((h, p)) = hostport.rsplit_once(':') {
                host = h.to_string();
                port = p.parse().unwrap_or(0);
            }
            let is_master = flags.contains("master");
            let is_fail = flags.contains("fail") || flags.contains("disconnected");
            nodes.push(NodeStatus {
                host,
                port,
                role: if is_master { "master".into() } else { "replica".into() },
                status: if is_fail { "down".into() } else { "up".into() },
                extra: Some(line.to_string()),
            });
        }
    }
    Ok(nodes)
}

async fn topology_from_sentinel(handle: &ConnHandle) -> AppResult<Vec<NodeStatus>> {
    use fred::clients::SentinelClient;
    use fred::types::config::SentinelConfig;
    use std::time::Duration;

    let service = handle.config.service_name.clone().unwrap_or_default();
    let mut sentinel_hosts: Vec<(String, u16)> =
        vec![(handle.config.host.clone(), handle.config.port)];
    for n in &handle.config.nodes {
        sentinel_hosts.push((n.host.clone(), n.port));
    }

    let mut nodes: Vec<NodeStatus> = Vec::new();
    let mut last_err: Option<String> = None;

    for (host, port) in sentinel_hosts {
        let config = SentinelConfig {
            host: host.clone(),
            port,
            username: handle.config.username.clone(),
            password: handle.config.password.clone(),
            ..Default::default()
        };
        let client = SentinelClient::new(config, None, None, None);
        match tokio::time::timeout(Duration::from_secs(5), client.init()).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                last_err = Some(format!("{}:{}", host, port));
                let _ = e;
                continue;
            }
            Err(_) => {
                last_err = Some(format!("{}:{} 连接超时", host, port));
                continue;
            }
        }

        // 主节点
        if let Ok(masters) = client.masters::<Vec<serde_json::Value>>().await {
            for m in masters {
                let mhost = m.get("ip").and_then(|v| v.as_str()).unwrap_or("");
                let mport = m.get("port").and_then(|v| v.as_i64()).unwrap_or(0);
                let flags = m.get("flags").and_then(|v| v.as_str()).unwrap_or("");
                nodes.push(NodeStatus {
                    host: mhost.to_string(),
                    port: mport as u16,
                    role: "master".into(),
                    status: if flags.contains("s_down") || flags.contains("odown") {
                        "down".into()
                    } else {
                        "up".into()
                    },
                    extra: Some(m.to_string()),
                });
            }
        }

        // 从库（Redis 5.0+ 用 REPLICAS，旧版本用 SLAVES）
        let mut replicas_json: Vec<serde_json::Value> = Vec::new();
        for cmd in ["REPLICAS", "SLAVES"] {
            if let Ok(v) = client
                .custom::<Value, _>(CustomCommand::new("SENTINEL", None::<u16>, false), vec![cmd, service.as_str()])
                .await
            {
                if let Value::Array(items) = v {
                    replicas_json = items
                        .iter()
                        .map(value_to_json)
                        .collect::<Vec<serde_json::Value>>();
                    break;
                }
            }
        }
        for r in replicas_json {
                let rhost = r.get("ip").and_then(|v| v.as_str()).unwrap_or("");
                let rport = r.get("port").and_then(|v| v.as_i64()).unwrap_or(0);
                let flags = r.get("flags").and_then(|v| v.as_str()).unwrap_or("");
                nodes.push(NodeStatus {
                    host: rhost.to_string(),
                    port: rport as u16,
                    role: "replica".into(),
                    status: if flags.contains("disconnected") || flags.contains("s_down") {
                        "down".into()
                    } else {
                        "up".into()
                    },
                    extra: Some(r.to_string()),
                });
            }
        // 哨兵节点（SENTINEL SENTINELS 只返回其他哨兵，补充当前连接的哨兵）
        let mut has_self_sentinel = false;
        if let Ok(sentinels) = client.sentinels::<Vec<serde_json::Value>, _>(&service).await {
            for s in sentinels {
                let shost = s.get("ip").and_then(|v| v.as_str()).unwrap_or("");
                let sport = s.get("port").and_then(|v| v.as_i64()).unwrap_or(0);
                if shost == host && sport as u16 == port {
                    has_self_sentinel = true;
                }
                nodes.push(NodeStatus {
                    host: shost.to_string(),
                    port: sport as u16,
                    role: "sentinel".into(),
                    status: "up".into(),
                    extra: Some(s.to_string()),
                });
            }
        }
        if !has_self_sentinel {
            nodes.push(NodeStatus {
                host: host.clone(),
                port,
                role: "sentinel".into(),
                status: "up".into(),
                extra: Some("当前连接的哨兵节点".into()),
            });
        }

        let _ = client.quit().await;
        last_err = None;
        break;
    }

    if nodes.is_empty() {
        if let Some(err) = last_err {
            return Err(AppError::new(format!("无法获取哨兵信息: {}", err)));
        }
        return Err(AppError::new("无法获取哨兵信息（服务名可能不存在）"));
    }

    Ok(nodes)
}

/// 获取数据库列表（单机 / 主从 / 哨兵）
pub async fn list_databases(
    manager: &RedisManager,
    conn_id: &str,
) -> AppResult<Vec<i64>> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;

    // 从 CONFIG databases 读取
    let configs: Vec<(String, String)> = pool.config_get("databases").await?;
    let total = configs
        .iter()
        .find(|(k, _)| k == "databases")
        .and_then(|(_, v)| v.parse::<i64>().ok())
        .unwrap_or(16)
        .min(64);

    // 从 INFO keyspace 找有键的库
    let raw: String = pool.info(Some(InfoKind::Keyspace)).await?;
    let mut used = Vec::new();
    for section in parse_info(&raw) {
        for (k, _) in section.fields {
            if let Some(db) = k.strip_prefix("db") {
                if let Ok(n) = db.parse::<i64>() {
                    used.push(n);
                }
            }
        }
    }

    let mut all: Vec<i64> = (0..total).collect();
    if !used.is_empty() {
        all.sort();
        all.dedup();
    }
    Ok(all)
}
