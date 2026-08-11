use fred::prelude::*;
use serde::Serialize;
use tauri::ipc::Channel;

use crate::error::{AppError, AppResult};
use crate::redisx::{display_bytes, value_to_json, ConnHandle, RedisManager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PubSubEvent {
    /// "message" | "pmessage" | "smessage"
    pub kind: String,
    /// 频道名（pmessage 时为实际频道）
    pub channel: String,
    pub text: Option<String>,
    pub json: Option<serde_json::Value>,
    pub server: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorEvent {
    pub timestamp: f64,
    pub db: u8,
    pub client: String,
    pub command: String,
    pub args: Vec<String>,
    pub raw: String,
}

/// 订阅频道，消息通过 channel 推送到前端
pub async fn subscribe(
    manager: &RedisManager,
    conn_id: &str,
    channels: Vec<String>,
    patterns: Vec<String>,
    out: Channel<PubSubEvent>,
) -> AppResult<()> {
    let handle = manager.get(conn_id)?;
    // 使用池中第一个连接作为订阅连接（订阅模式不能执行其他命令）
    let client = handle.pool.clients()[0].clone();
    let inner = client.on_message(move |msg| {
        let out = out.clone();
        async move {
            let text = match &msg.value {
                Value::String(s) => Some(s.to_string()),
                Value::Bytes(b) => Some(display_bytes(b.as_ref()).value),
                Value::Integer(i) => Some(i.to_string()),
                Value::Double(d) => Some(d.to_string()),
                Value::Boolean(b) => Some(b.to_string()),
                Value::Null => Some("(nil)".into()),
                _ => Some(value_to_json(&msg.value).to_string()),
            };
            let kind = format!("{:?}", msg.kind).to_lowercase();
            let _ = out.send(PubSubEvent {
                kind,
                channel: msg.channel.to_string(),
                text,
                json: Some(value_to_json(&msg.value)),
                server: format!("{}", msg.server),
            });
            Ok(())
        }
    });
    let task = tokio::spawn(async move {
        let _ = inner.await;
    });

    // 订阅
    if !channels.is_empty() {
        client.subscribe(channels).await?;
    }
    if !patterns.is_empty() {
        client.psubscribe(patterns).await?;
    }

    handle.sub_tasks.lock().push(task);
    Ok(())
}

/// 退订
pub async fn unsubscribe(
    manager: &RedisManager,
    conn_id: &str,
    channels: Vec<String>,
    patterns: Vec<String>,
) -> AppResult<()> {
    let handle = manager.get(conn_id)?;
    let client = handle.pool.clients()[0].clone();
    if !channels.is_empty() {
        let _ = client.unsubscribe(channels).await;
    }
    if !patterns.is_empty() {
        let _ = client.punsubscribe(patterns).await;
    }
    Ok(())
}

/// 发布消息（使用池中第二个连接，避免订阅模式限制）
pub async fn publish(
    manager: &RedisManager,
    conn_id: &str,
    channel: &str,
    message: &str,
) -> AppResult<u64> {
    let handle = manager.get(conn_id)?;
    let client = handle
        .pool
        .clients()
        .get(1)
        .cloned()
        .unwrap_or_else(|| handle.pool.clients()[0].clone());
    let count: u64 = client.publish(channel, message).await?;
    Ok(count)
}

/// 开启 MONITOR，命令流通过 channel 推送到前端（仅支持单机/主从）
pub async fn start_monitor(
    manager: &RedisManager,
    conn_id: &str,
    out: Channel<MonitorEvent>,
) -> AppResult<()> {
    let handle = manager.get(conn_id)?;

    if handle.config.mode != crate::model::ConnMode::Single
        && handle.config.mode != crate::model::ConnMode::MasterSlave
    {
        return Err(AppError::new(
            "MONITOR 仅支持单机 / 主从模式（Cluster/Sentinel 请使用订阅功能）",
        ));
    }

    let config = crate::redisx::build_fred_config(&handle.config)?;
    let stream = fred::monitor::run(config)
        .await
        .map_err(|e| AppError::new(format!("启动 MONITOR 失败: {}", e)))?;

    let task = tokio::spawn(async move {
        use futures::StreamExt;
        let mut stream = stream;
        while let Some(cmd) = stream.next().await {
            let raw = format!("{}", cmd);
            let args: Vec<String> = cmd
                .args
                .iter()
                .map(|a| display_bytes(&value_bytes(a)).value)
                .collect();
            let ok = out.send(MonitorEvent {
                timestamp: cmd.timestamp,
                db: cmd.db,
                client: cmd.client,
                command: cmd.command,
                args,
                raw,
            });
            if ok.is_err() {
                break;
            }
        }
    });

    handle.sub_tasks.lock().push(task);
    Ok(())
}

fn value_bytes(v: &Value) -> Vec<u8> {
    match v {
        Value::String(s) => s.as_bytes().to_vec(),
        Value::Bytes(b) => b.as_ref().to_vec(),
        Value::Integer(i) => i.to_string().into_bytes(),
        Value::Double(d) => d.to_string().into_bytes(),
        Value::Boolean(b) => b.to_string().into_bytes(),
        Value::Null => Vec::new(),
        _ => value_to_json(v).to_string().into_bytes(),
    }
}

/// 停止该连接上的所有后台任务（订阅 / Monitor）
pub async fn stop_all(handle: &ConnHandle) {
    let tasks = std::mem::take(&mut *handle.sub_tasks.lock());
    for task in tasks {
        task.abort();
    }
}
