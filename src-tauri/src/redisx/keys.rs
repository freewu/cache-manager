use std::collections::HashMap;

use fred::prelude::*;
use fred::types::scan::ScanType;
use fred::types::CustomCommand;
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::model::EncodedValue;
use crate::redisx::{value_to_json, RedisManager};

/// 单页扫描上限
const SCAN_PAGE_MAX: usize = 1000;
/// 大集合/哈希的读取上限
const COLLECTION_MAX: usize = 5000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanPage {
    pub cursor: String,
    pub keys: Vec<String>,
    pub truncated: bool,
    /// key -> 类型（单机/主从模式下批量查询；集群模式暂不提供）
    pub types: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyInfo {
    pub key: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub ttl: i64,
    pub pttl: i64,
    pub length: i64,
    pub memory: Option<i64>,
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HashField {
    pub field: EncodedValue,
    pub value: EncodedValue,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZMember {
    pub member: EncodedValue,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamEntry {
    pub id: String,
    pub fields: Vec<HashField>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ValuePayload {
    #[serde(rename_all = "camelCase")]
    String { value: EncodedValue },
    #[serde(rename_all = "camelCase")]
    List { values: Vec<EncodedValue>, truncated: bool },
    #[serde(rename_all = "camelCase")]
    Hash { fields: Vec<HashField>, truncated: bool },
    #[serde(rename_all = "camelCase")]
    Set { values: Vec<EncodedValue>, truncated: bool },
    #[serde(rename_all = "camelCase")]
    ZSet { members: Vec<ZMember>, truncated: bool },
    #[serde(rename_all = "camelCase")]
    Stream { entries: Vec<StreamEntry>, truncated: bool },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueView {
    pub key: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub ttl: i64,
    pub length: i64,
    pub payload: ValuePayload,
}

/// 执行一条自定义命令并返回解析后的 Value
pub async fn run_custom(pool: &Pool, cmd: &str, args: Vec<String>) -> AppResult<Value> {
    let frame = pool
        .custom_raw(
            CustomCommand::new(cmd, None::<u16>, false),
            args,
        )
        .await
        .map_err(|e| AppError::new(format!("{} 执行失败: {}", cmd, e)))?;
    Value::try_from(frame).map_err(|e| AppError::new(format!("响应解析失败: {}", e)))
}

fn value_to_display(value: &Value) -> EncodedValue {
    match value {
        Value::String(s) => EncodedValue::from_string(s.to_string()),
        Value::Bytes(b) => EncodedValue::from_bytes(b.as_ref()),
        Value::Integer(i) => EncodedValue::from_string(i.to_string()),
        Value::Double(d) => EncodedValue::from_string(d.to_string()),
        Value::Boolean(b) => EncodedValue::from_string(b.to_string()),
        Value::Null => EncodedValue::from_string("(nil)".into()),
        Value::Queued => EncodedValue::from_string("QUEUED".into()),
        Value::Map(m) => EncodedValue::from_string(value_to_json(&Value::Map(m.clone())).to_string()),
        Value::Array(a) => EncodedValue::from_string(value_to_json(&Value::Array(a.clone())).to_string()),
    }
}

/// 扫描键
pub async fn scan_keys(
    manager: &RedisManager,
    conn_id: &str,
    cursor: &str,
    pattern: &str,
    count: Option<u32>,
    type_filter: Option<String>,
    replica: Option<usize>,
) -> AppResult<ScanPage> {
    let handle = manager.get(conn_id)?;
    let scan_type = match type_filter.as_deref() {
        None | Some("") | Some("all") | Some("none") => None,
        Some("string") => Some(ScanType::String),
        Some("hash") => Some(ScanType::Hash),
        Some("list") => Some(ScanType::List),
        Some("set") => Some(ScanType::Set),
        Some("zset") => Some(ScanType::ZSet),
        Some("stream") => Some(ScanType::Stream),
        Some(other) => return Err(AppError::new(format!("未知键类型: {}", other))),
    };

    if handle.config.mode == crate::model::ConnMode::Cluster {
        let (next_cursor, keys) =
            RedisManager::cluster_scan(&handle, cursor, pattern, count, scan_type).await?;
        let truncated = keys.len() >= SCAN_PAGE_MAX;
        Ok(ScanPage {
            cursor: next_cursor,
            keys,
            truncated,
            types: HashMap::new(),
        })
    } else {
        let pool = RedisManager::pool_for(&handle, replica)?;
        let (next_cursor, keys) = pool
            .scan_page::<(String, Vec<String>), _, _>(cursor, pattern, count, scan_type)
            .await
            .map_err(|e| AppError::new(format!("扫描失败: {}", e)))?;
        let truncated = keys.len() >= SCAN_PAGE_MAX;
        // 批量查询每个键的类型（pipeline TYPE）
        let types = fetch_types(&pool, &keys).await;
        Ok(ScanPage {
            cursor: next_cursor,
            keys,
            truncated,
            types,
        })
    }
}

/// 批量查询键类型（单机/主从模式）
async fn fetch_types(pool: &Pool, keys: &[String]) -> HashMap<String, String> {
    if keys.is_empty() {
        return HashMap::new();
    }
    let client = pool.next();
    let pipe = client.pipeline();
    for k in keys {
        let _ = pipe.r#type::<String, _>(k).await;
    }
    match pipe.all::<Vec<String>>().await {
        Ok(vals) => keys.iter().cloned().zip(vals).collect(),
        Err(_) => HashMap::new(),
    }
}

/// 获取键的基础信息（类型 / TTL / 长度 / 内存）
pub async fn key_info(
    manager: &RedisManager,
    conn_id: &str,
    key: &str,
    replica: Option<usize>,
) -> AppResult<KeyInfo> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, replica)?;

    let r#type = pool.r#type::<String, _>(key).await?;
    let ttl: i64 = pool.ttl(key).await?;
    let pttl: i64 = pool.pttl(key).await?;

    let length = match r#type.as_str() {
        "string" => pool.strlen::<i64, _>(key).await.unwrap_or(0),
        "list" => pool.llen::<i64, _>(key).await.unwrap_or(0),
        "hash" => pool.hlen::<i64, _>(key).await.unwrap_or(0),
        "set" => pool.scard::<i64, _>(key).await.unwrap_or(0),
        "zset" => pool.zcard::<i64, _>(key).await.unwrap_or(0),
        "stream" => pool.xlen::<i64, _>(key).await.unwrap_or(0),
        _ => 0,
    };

    // MEMORY USAGE
    let memory = match run_custom(pool, "MEMORY", vec!["USAGE".into(), key.to_string()]).await {
        Ok(Value::Integer(i)) => Some(i),
        _ => None,
    };

    // OBJECT ENCODING
    let encoding =
        match run_custom(pool, "OBJECT", vec!["ENCODING".into(), key.to_string()]).await {
            Ok(v) => Some(value_to_display(&v).value),
            _ => None,
        };

    Ok(KeyInfo {
        key: key.to_string(),
        r#type,
        ttl,
        pttl,
        length,
        memory,
        encoding,
    })
}

/// 读取键的值（按类型）
pub async fn get_value(
    manager: &RedisManager,
    conn_id: &str,
    key: &str,
    replica: Option<usize>,
) -> AppResult<ValueView> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, replica)?;

    let r#type = pool.r#type::<String, _>(key).await?;
    let ttl: i64 = pool.ttl(key).await?;

    let (length, payload) = match r#type.as_str() {
        "string" => {
            let value: Option<Vec<u8>> = pool.get(key).await?;
            match value {
                Some(bytes) => (bytes.len() as i64, ValuePayload::String {
                    value: EncodedValue::from_bytes(&bytes),
                }),
                None => (0, ValuePayload::String {
                    value: EncodedValue::from_string("(key 不存在)".into()),
                }),
            }
        }
        "list" => {
            let len: i64 = pool.llen(key).await?;
            let values: Vec<Vec<u8>> = pool.lrange(key, 0, -1).await?;
            let truncated = values.len() > COLLECTION_MAX;
            let values: Vec<EncodedValue> = values
                .iter()
                .take(COLLECTION_MAX)
                .map(|v| EncodedValue::from_bytes(v))
                .collect();
            (len, ValuePayload::List { values, truncated })
        }
        "hash" => {
            let len: i64 = pool.hlen(key).await?;
            let mut fields = Vec::new();
            let mut truncated = false;
            if len as usize <= COLLECTION_MAX {
                let map: HashMap<String, Vec<u8>> = pool.hgetall(key).await?;
                for (f, v) in map {
                    fields.push(HashField {
                        field: EncodedValue::from_string(f),
                        value: EncodedValue::from_bytes(&v),
                    });
                }
            } else {
                // HSCAN 分页读取
                let mut cursor = String::from("0");
                loop {
                    let (next, page) = hscan_page_custom(pool, key, &cursor, 500).await?;
                    fields.extend(page);
                    cursor = next;
                    if cursor == "0" || fields.len() >= COLLECTION_MAX {
                        truncated = fields.len() >= COLLECTION_MAX;
                        break;
                    }
                }
            }
            (len, ValuePayload::Hash { fields, truncated })
        }
        "set" => {
            let len: i64 = pool.scard(key).await?;
            let mut values: Vec<EncodedValue> = Vec::new();
            let mut truncated = false;
            if len as usize <= COLLECTION_MAX {
                let members: Vec<Vec<u8>> = pool.smembers(key).await?;
                values = members.iter().map(|m| EncodedValue::from_bytes(m)).collect();
            } else {
                let mut cursor = String::from("0");
                loop {
                    let (next, page) = sscan_page_custom(pool, key, &cursor, 500).await?;
                    values.extend(page);
                    cursor = next;
                    if cursor == "0" || values.len() >= COLLECTION_MAX {
                        truncated = values.len() >= COLLECTION_MAX;
                        break;
                    }
                }
            }
            (len, ValuePayload::Set { values, truncated })
        }
        "zset" => {
            let len: i64 = pool.zcard(key).await?;
            let mut members: Vec<ZMember> = Vec::new();
            let mut truncated = false;
            if len as usize <= COLLECTION_MAX {
                let items: Vec<(Vec<u8>, f64)> = pool
                    .zrange(key, 0, -1, None, false, None, true)
                    .await?;
                members = items
                    .iter()
                    .map(|(m, s)| ZMember {
                        member: EncodedValue::from_bytes(m),
                        score: *s,
                    })
                    .collect();
            } else {
                let mut cursor = String::from("0");
                loop {
                    let (next, page) = zscan_page_custom(pool, key, &cursor, 500).await?;
                    members.extend(page);
                    cursor = next;
                    if cursor == "0" || members.len() >= COLLECTION_MAX {
                        truncated = members.len() >= COLLECTION_MAX;
                        break;
                    }
                }
            }
            (len, ValuePayload::ZSet { members, truncated })
        }
        "stream" => {
            let len: i64 = pool.xlen(key).await?;
            let entries: Vec<(String, HashMap<String, Vec<u8>>)> = pool
                .xrange_values::<String, String, Vec<u8>, _, _, _>(key, "-", "+", Some(1000))
                .await?;
            let truncated = entries.len() >= 1000;
            let entries = entries
                .into_iter()
                .map(|(id, map)| StreamEntry {
                    id,
                    fields: map
                        .into_iter()
                        .map(|(f, v)| HashField {
                            field: EncodedValue::from_string(f),
                            value: EncodedValue::from_bytes(&v),
                        })
                        .collect(),
                })
                .collect();
            (len, ValuePayload::Stream { entries, truncated })
        }
        other => {
            return Err(AppError::new(format!("暂不支持的值类型: {}", other)));
        }
    };

    Ok(ValueView {
        key: key.to_string(),
        r#type,
        ttl,
        length,
        payload,
    })
}

async fn hscan_page_custom(
    pool: &Pool,
    key: &str,
    cursor: &str,
    count: u32,
) -> AppResult<(String, Vec<HashField>)> {
    let value = run_custom(
        pool,
        "HSCAN",
        vec![
            key.to_string(),
            cursor.to_string(),
            "COUNT".into(),
            count.to_string(),
        ],
    )
    .await?;
    if let Value::Array(items) = value {
        if items.len() >= 2 {
            let next = value_to_display(&items[0]).value;
            let mut fields = Vec::new();
            if let Value::Array(pairs) = &items[1] {
                let mut i = 0;
                while i + 1 < pairs.len() {
                    fields.push(HashField {
                        field: value_to_display(&pairs[i]),
                        value: value_to_display(&pairs[i + 1]),
                    });
                    i += 2;
                }
            }
            return Ok((next, fields));
        }
    }
    Err(AppError::new("HSCAN 响应格式异常"))
}

async fn sscan_page_custom(
    pool: &Pool,
    key: &str,
    cursor: &str,
    count: u32,
) -> AppResult<(String, Vec<EncodedValue>)> {
    let value = run_custom(
        pool,
        "SSCAN",
        vec![
            key.to_string(),
            cursor.to_string(),
            "COUNT".into(),
            count.to_string(),
        ],
    )
    .await?;
    if let Value::Array(items) = value {
        if items.len() >= 2 {
            let next = value_to_display(&items[0]).value;
            let mut values = Vec::new();
            if let Value::Array(members) = &items[1] {
                values = members.iter().map(value_to_display).collect();
            }
            return Ok((next, values));
        }
    }
    Err(AppError::new("SSCAN 响应格式异常"))
}

async fn zscan_page_custom(
    pool: &Pool,
    key: &str,
    cursor: &str,
    count: u32,
) -> AppResult<(String, Vec<ZMember>)> {
    let value = run_custom(
        pool,
        "ZSCAN",
        vec![
            key.to_string(),
            cursor.to_string(),
            "COUNT".into(),
            count.to_string(),
        ],
    )
    .await?;
    if let Value::Array(items) = value {
        if items.len() >= 2 {
            let next = value_to_display(&items[0]).value;
            let mut members = Vec::new();
            if let Value::Array(pairs) = &items[1] {
                let mut i = 0;
                while i + 1 < pairs.len() {
                    let score = match &pairs[i + 1] {
                        Value::Integer(n) => *n as f64,
                        Value::Double(d) => *d,
                        _ => 0.0,
                    };
                    members.push(ZMember {
                        member: value_to_display(&pairs[i]),
                        score,
                    });
                    i += 2;
                }
            }
            return Ok((next, members));
        }
    }
    Err(AppError::new("ZSCAN 响应格式异常"))
}

/// 删除键
pub async fn delete_keys(manager: &RedisManager, conn_id: &str, keys: Vec<String>) -> AppResult<u64> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    let count: u64 = pool.del(keys).await?;
    Ok(count)
}

/// 重命名键
pub async fn rename_key(manager: &RedisManager, conn_id: &str, source: &str, dest: &str) -> AppResult<()> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    pool.rename::<(), _, _>(source, dest).await?;
    Ok(())
}

/// 创建键（写入初始内容；键已存在时报错）
#[allow(clippy::too_many_arguments)]
pub async fn create_key(
    manager: &RedisManager,
    conn_id: &str,
    key: &str,
    kind: &str,
    field: Option<&str>,
    value: &str,
    score: Option<f64>,
) -> AppResult<()> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;

    let exists: bool = pool.exists(key).await?;
    if exists {
        return Err(crate::error::AppError::new(format!("键已存在: {key}")));
    }

    match kind {
        "string" => {
            pool.set::<(), _, _>(key, value.as_bytes().to_vec(), None, None, false)
                .await?;
        }
        "hash" => {
            run_custom(
                pool,
                "HSET",
                vec![key.into(), field.unwrap_or("").into(), value.into()],
            )
            .await?;
        }
        "list" => {
            run_custom(pool, "RPUSH", vec![key.into(), value.into()]).await?;
        }
        "set" => {
            run_custom(pool, "SADD", vec![key.into(), value.into()]).await?;
        }
        "zset" => {
            let s = score.unwrap_or(1.0);
            run_custom(pool, "ZADD", vec![key.into(), s.to_string(), value.into()])
                .await?;
        }
        "stream" => {
            run_custom(
                pool,
                "XADD",
                vec![key.into(), "*".into(), field.unwrap_or("f").into(), value.into()],
            )
            .await?;
        }
        _ => {
            return Err(crate::error::AppError::new(format!("未知类型: {kind}")));
        }
    }
    Ok(())
}

/// 设置 TTL（秒）。ttl < 0 表示取消过期时间
pub async fn set_ttl(manager: &RedisManager, conn_id: &str, key: &str, ttl: i64) -> AppResult<()> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    if ttl < 0 {
        pool.persist::<i64, _>(key).await?;
    } else {
        pool.expire::<i64, _>(key, ttl, None).await?;
    }
    Ok(())
}

/// 清空当前数据库
pub async fn flush_db(manager: &RedisManager, conn_id: &str) -> AppResult<()> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    run_custom(pool, "FLUSHDB", vec![]).await?;
    Ok(())
}

/// 修改 String 值
pub async fn set_string_value(
    manager: &RedisManager,
    conn_id: &str,
    key: &str,
    value: &str,
    ttl: Option<i64>,
    encoding: &str,
) -> AppResult<()> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;

    let bytes: Vec<u8> = if encoding == "base64" {
        crate::model::base64_decode(value)?
    } else {
        value.as_bytes().to_vec()
    };

    match ttl {
        Some(t) if t > 0 => {
            pool.set::<(), _, _>(key, bytes, Some(Expiration::EX(t)), None, false)
                .await?;
        }
        _ => {
            pool.set::<(), _, _>(key, bytes, None, None, false).await?;
        }
    }
    Ok(())
}

/// Hash: 新增/修改字段
pub async fn set_hash_field(
    manager: &RedisManager,
    conn_id: &str,
    key: &str,
    field: &str,
    value: &str,
) -> AppResult<()> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    run_custom(pool, "HSET", vec![key.into(), field.into(), value.into()]).await?;
    Ok(())
}

/// Hash: 删除字段
pub async fn delete_hash_fields(
    manager: &RedisManager,
    conn_id: &str,
    key: &str,
    fields: Vec<String>,
) -> AppResult<u64> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    let mut args = vec![key.to_string()];
    args.extend(fields);
    let value = run_custom(pool, "HDEL", args).await?;
    match value {
        Value::Integer(n) => Ok(n as u64),
        _ => Ok(0),
    }
}

/// List: 推入元素（head = LPUSH, tail = RPUSH）
pub async fn push_list(
    manager: &RedisManager,
    conn_id: &str,
    key: &str,
    values: Vec<String>,
    tail: bool,
) -> AppResult<u64> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    let cmd = if tail { "RPUSH" } else { "LPUSH" };
    let mut args = vec![key.to_string()];
    args.extend(values);
    let value = run_custom(pool, cmd, args).await?;
    match value {
        Value::Integer(n) => Ok(n as u64),
        _ => Ok(0),
    }
}

/// List: 删除元素
pub async fn remove_list(
    manager: &RedisManager,
    conn_id: &str,
    key: &str,
    value: &str,
    count: i64,
) -> AppResult<u64> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    let value = run_custom(
        pool,
        "LREM",
        vec![key.into(), count.to_string(), value.into()],
    )
    .await?;
    match value {
        Value::Integer(n) => Ok(n as u64),
        _ => Ok(0),
    }
}

/// Set: 新增成员
pub async fn add_set_members(
    manager: &RedisManager,
    conn_id: &str,
    key: &str,
    members: Vec<String>,
) -> AppResult<u64> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    let mut args = vec![key.to_string()];
    args.extend(members);
    let value = run_custom(pool, "SADD", args).await?;
    match value {
        Value::Integer(n) => Ok(n as u64),
        _ => Ok(0),
    }
}

/// Set: 删除成员
pub async fn remove_set_members(
    manager: &RedisManager,
    conn_id: &str,
    key: &str,
    members: Vec<String>,
) -> AppResult<u64> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    let mut args = vec![key.to_string()];
    args.extend(members);
    let value = run_custom(pool, "SREM", args).await?;
    match value {
        Value::Integer(n) => Ok(n as u64),
        _ => Ok(0),
    }
}

/// ZSet: 新增成员（score, member）
pub async fn add_zset_members(
    manager: &RedisManager,
    conn_id: &str,
    key: &str,
    members: Vec<(f64, String)>,
) -> AppResult<u64> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    let mut args = vec![key.to_string()];
    for (score, member) in members {
        args.push(score.to_string());
        args.push(member);
    }
    let value = run_custom(pool, "ZADD", args).await?;
    match value {
        Value::Integer(n) => Ok(n as u64),
        _ => Ok(0),
    }
}

/// ZSet: 删除成员
pub async fn remove_zset_members(
    manager: &RedisManager,
    conn_id: &str,
    key: &str,
    members: Vec<String>,
) -> AppResult<u64> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    let mut args = vec![key.to_string()];
    args.extend(members);
    let value = run_custom(pool, "ZREM", args).await?;
    match value {
        Value::Integer(n) => Ok(n as u64),
        _ => Ok(0),
    }
}

/// Stream: 追加条目
pub async fn xadd_stream(
    manager: &RedisManager,
    conn_id: &str,
    key: &str,
    id: &str,
    fields: Vec<(String, String)>,
) -> AppResult<String> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    let mut args = vec![key.to_string(), id.to_string()];
    for (f, v) in fields {
        args.push(f);
        args.push(v);
    }
    let value = run_custom(pool, "XADD", args).await?;
    Ok(value_to_display(&value).value)
}

/// Stream: 删除条目
pub async fn xdel_stream(
    manager: &RedisManager,
    conn_id: &str,
    key: &str,
    ids: Vec<String>,
) -> AppResult<u64> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, None)?;
    let mut args = vec![key.to_string()];
    args.extend(ids);
    let value = run_custom(pool, "XDEL", args).await?;
    match value {
        Value::Integer(n) => Ok(n as u64),
        _ => Ok(0),
    }
}
