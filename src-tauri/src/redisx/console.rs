use fred::prelude::*;
use fred::types::CustomCommand;
use serde::Serialize;
use std::time::Instant;

use crate::error::{AppError, AppResult};
use crate::redisx::{value_to_json, ConnHandle, RedisManager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub command: String,
    pub elapsed_ms: f64,
    pub ok: bool,
    pub error: Option<String>,
    /// 结果 JSON（二进制安全）
    pub value: Option<serde_json::Value>,
    /// 单值文本（若结果只有一个标量值时使用）
    pub text: Option<String>,
}

/// 会阻塞连接的命令，需要标记 blocking
const BLOCKING_COMMANDS: &[&str] = &[
    "BLPOP", "BRPOP", "BRPOPLPUSH", "BLMOVE", "BLMPOP", "BZPOPMIN", "BZPOPMAX", "BZMPOP",
    "WAIT", "SUBSCRIBE", "PSUBSCRIBE", "SSUBSCRIBE", "MONITOR", "XREAD", "XREADGROUP",
];

/// 在连接上执行任意 Redis 命令
pub async fn execute(
    manager: &RedisManager,
    conn_id: &str,
    line: &str,
    replica: Option<usize>,
) -> AppResult<CommandResult> {
    let handle = manager.get(conn_id)?;
    let pool = RedisManager::pool_for(&handle, replica)?;
    execute_on_pool(&handle, pool, line).await
}

pub async fn execute_on_pool(
    _handle: &ConnHandle,
    pool: &Pool,
    line: &str,
) -> AppResult<CommandResult> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(AppError::new("命令不能为空"));
    }

    let args = shlex::split(trimmed).ok_or_else(|| AppError::new("命令解析失败（引号不匹配）"))?;
    if args.is_empty() {
        return Err(AppError::new("命令不能为空"));
    }

    let cmd = args[0].to_uppercase();
    if cmd == "SUBSCRIBE" || cmd == "PSUBSCRIBE" || cmd == "SSUBSCRIBE" {
        return Err(AppError::new("请使用「实时监控」页面进行订阅"));
    }
    if cmd == "MONITOR" {
        return Err(AppError::new("请使用「实时监控」页面开启 Monitor"));
    }
    if cmd == "SELECT" {
        return Err(AppError::new("请通过连接设置切换数据库 (Database)"));
    }

    let blocking = BLOCKING_COMMANDS.contains(&cmd.as_str());
    let started = Instant::now();

    // 非阻塞命令用 30s 超时兜底
    let future = pool.custom_raw(
        CustomCommand::new(cmd.as_str(), None::<u16>, blocking),
        args[1..].to_vec(),
    );
    let result = if blocking {
        future.await
    } else {
        tokio::time::timeout(std::time::Duration::from_secs(30), future)
            .await
            .map_err(|_| AppError::new("命令执行超时 (30s)"))?
    };

    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;

    match result {
        Ok(frame) => {
            let value = Value::try_from(frame).map_err(|e| AppError::new(format!("响应解析失败: {}", e)))?;
            let json = value_to_json(&value);
            let text = scalar_text(&value);
            Ok(CommandResult {
                command: trimmed.to_string(),
                elapsed_ms,
                ok: true,
                error: None,
                value: Some(json),
                text,
            })
        }
        Err(e) => Ok(CommandResult {
            command: trimmed.to_string(),
            elapsed_ms,
            ok: false,
            error: Some(e.to_string()),
            value: None,
            text: None,
        }),
    }
}

fn scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.to_string()),
        Value::Integer(i) => Some(i.to_string()),
        Value::Double(d) => Some(d.to_string()),
        Value::Boolean(b) => Some(b.to_string()),
        Value::Null => Some("(nil)".into()),
        Value::Bytes(_) | Value::Map(_) | Value::Array(_) | Value::Queued => None,
    }
}
