use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager, State};
use tauri::ipc::Channel;

use crate::error::AppResult;
use crate::memcachedx::MemcachedManager;
use crate::model::{ConnConfig, ConnStatusInfo, ImportResult, NodeStatus};
use crate::redisx::console::CommandResult;
use crate::redisx::keys::{
    KeyInfo, ScanPage, ValueView,
};
use crate::redisx::server::{ClientEntry, ServerInfo, SlowlogEntry};
use crate::redisx::stream::{MonitorEvent, PubSubEvent};
use crate::redisx::RedisManager;

pub struct AppState {
    pub manager: Arc<RedisManager>,
    pub memcached: Arc<MemcachedManager>,
}

/// 判断该连接是否为 Memcached 连接
fn is_memcached(state: &State<'_, AppState>, conn_id: &str) -> bool {
    state.memcached.has(conn_id)
}

// ==================== 连接管理 ====================

#[tauri::command]
pub async fn test_connection(config: ConnConfig) -> AppResult<String> {
    if config.mode == crate::model::ConnMode::Memcached {
        MemcachedManager::new().test(&config).await
    } else {
        RedisManager::test(&config).await
    }
}

#[tauri::command]
pub async fn connect_connection(
    state: State<'_, AppState>,
    config: ConnConfig,
) -> AppResult<String> {
    if config.mode == crate::model::ConnMode::Memcached {
        state.memcached.connect(config).await
    } else {
        state.manager.connect(config).await
    }
}

#[tauri::command]
pub async fn disconnect_connection(
    state: State<'_, AppState>,
    conn_id: String,
) -> AppResult<()> {
    if is_memcached(&state, &conn_id) {
        state.memcached.disconnect(&conn_id).await;
    } else {
        let _ = state.manager.disconnect(&conn_id).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn disconnect_all(state: State<'_, AppState>) -> AppResult<()> {
    state.manager.disconnect_all().await;
    state.memcached.disconnect_all();
    Ok(())
}

#[tauri::command]
pub async fn connection_status(
    state: State<'_, AppState>,
    conn_id: String,
) -> AppResult<ConnStatusInfo> {
    if is_memcached(&state, &conn_id) {
        Ok(state.memcached.get(&conn_id)?.to_status_info())
    } else {
        let handle = state.manager.get(&conn_id)?;
        Ok(handle.to_status_info())
    }
}

#[tauri::command]
pub async fn list_connections(state: State<'_, AppState>) -> AppResult<Vec<ConnStatusInfo>> {
    let mut all = state.manager.list_status();
    all.extend(state.memcached.list_status());
    Ok(all)
}

/// 切换数据库（通过重新连接实现）
#[tauri::command]
pub async fn switch_database(
    state: State<'_, AppState>,
    conn_id: String,
    database: i64,
) -> AppResult<()> {
    if is_memcached(&state, &conn_id) {
        return Err(crate::error::AppError::new("Memcached 不支持数据库切换"));
    }
    let handle = state.manager.get(&conn_id)?;
    let mut config = handle.config.clone();
    config.database = Some(database);
    state.manager.connect(config).await?;
    Ok(())
}

#[tauri::command]
pub fn load_saved_connections(app: AppHandle) -> AppResult<Vec<ConnConfig>> {
    crate::store::load(&app)
}

#[tauri::command]
pub fn save_connections(app: AppHandle, connections: Vec<ConnConfig>) -> AppResult<()> {
    crate::store::save(&app, &connections)?;
    crate::tray::update_tray_menu(&app);
    Ok(())
}

/// 导出连接列表到 JSON 文件，返回文件路径
/// 目录优先级：设置里的默认导出目录 → 系统下载目录 → 应用配置目录
#[tauri::command]
pub fn export_connections(app: AppHandle) -> AppResult<String> {
    let configs = crate::store::load(&app)?;
    let json = serde_json::to_string_pretty(&configs)?;
    let settings = crate::store::load_settings(&app);
    // 1) 用户配置的默认导出目录（存在或可创建）
    let mut dir: Option<std::path::PathBuf> = settings
        .export_dir
        .filter(|d| !d.trim().is_empty())
        .map(std::path::PathBuf::from);
    // 2) 系统下载目录
    if dir.is_none() {
        dir = std::env::var("USERPROFILE")
            .ok()
            .map(|u| std::path::PathBuf::from(u).join("Downloads"))
            .filter(|p| p.is_dir());
    }
    // 3) 应用配置目录
    let dir = match dir {
        Some(d) => d,
        None => {
            let cfg = app
                .path()
                .app_config_dir()
                .map_err(|e| crate::error::AppError::new(format!("无法获取配置目录: {}", e)))?;
            cfg
        }
    };
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = dir.join(format!("cache-manager-connections-{}.json", secs));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(&path, json)?;
    Ok(path.to_string_lossy().into_owned())
}

/// 导入连接列表（JSON）。host:port 与现有连接一致的跳过，返回导入/跳过数量
#[tauri::command]
pub fn import_connections(app: AppHandle, json: String) -> AppResult<ImportResult> {
    let incoming: Vec<ConnConfig> = serde_json::from_str(&json)
        .map_err(|e| crate::error::AppError::new(format!("导入文件格式错误: {}", e)))?;
    let mut existing = crate::store::load(&app)?;
    let mut imported = 0usize;
    let mut duplicated = 0usize;
    let mut used_ids: std::collections::HashSet<String> =
        existing.iter().map(|c| c.id.clone()).collect();
    for mut cfg in incoming {
        // host:port 重复（忽略大小写）→ 跳过
        let dup = existing.iter().any(|c| {
            c.host.eq_ignore_ascii_case(&cfg.host) && c.port == cfg.port
        });
        if dup {
            duplicated += 1;
            continue;
        }
        // id 冲突时生成新 id
        if cfg.id.is_empty() || used_ids.contains(&cfg.id) {
            let secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            cfg.id = format!("imp-{}-{}", secs, imported);
        }
        used_ids.insert(cfg.id.clone());
        existing.push(cfg);
        imported += 1;
    }
    if imported > 0 {
        crate::store::save(&app, &existing)?;
        crate::tray::update_tray_menu(&app);
    }
    Ok(ImportResult {
        imported,
        duplicated,
    })
}

/// 重建托盘快速连接菜单（保存/删除连接后由前端调用）
#[tauri::command]
pub fn update_tray_menu(app: AppHandle) -> AppResult<()> {
    crate::tray::update_tray_menu(&app);
    Ok(())
}

/// 读取应用设置
#[tauri::command]
pub fn get_app_settings(app: AppHandle) -> crate::store::AppSettings {
    crate::store::load_settings(&app)
}

/// 保存应用设置（保留语言字段，语言由托盘/前端 locale 同步维护）
#[tauri::command]
pub fn set_app_settings(
    app: AppHandle,
    mut settings: crate::store::AppSettings,
) -> AppResult<()> {
    // 语言由 set_locale 统一维护，此处保留已有值，避免覆盖
    let prev = crate::store::load_settings(&app);
    if settings.locale == crate::store::AppSettings::default().locale {
        settings.locale = prev.locale;
    }
    // 导出目录当前无设置入口，保留旧值避免覆盖
    if settings.export_dir.is_none() {
        settings.export_dir = prev.export_dir;
    }
    crate::store::save_settings(&app, &settings)
}

/// 前端同步语言到后端（设置页切换 / 托盘切换），并重建托盘菜单
#[tauri::command]
pub fn set_locale(app: AppHandle, locale: String) -> AppResult<()> {
    let mut settings = crate::store::load_settings(&app);
    settings.locale = locale;
    crate::store::save_settings(&app, &settings)?;
    crate::tray::update_tray_menu(&app);
    Ok(())
}

/// 使用系统默认浏览器打开外部链接（项目主页 / Issue 等）
#[tauri::command]
pub fn open_external(app: AppHandle, url: String) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| crate::error::AppError::new(format!("打开链接失败: {}", e)))?;
    Ok(())
}

// ==================== 键操作 ====================

#[tauri::command]
pub async fn scan_keys(
    state: State<'_, AppState>,
    conn_id: String,
    cursor: String,
    pattern: String,
    count: Option<u32>,
    type_filter: Option<String>,
    replica: Option<usize>,
) -> AppResult<ScanPage> {
    if is_memcached(&state, &conn_id) {
        crate::memcachedx::scan_keys(&state.memcached, &conn_id, &pattern).await
    } else {
        crate::redisx::keys::scan_keys(
            &state.manager,
            &conn_id,
            &cursor,
            &pattern,
            count,
            type_filter,
            replica,
        )
        .await
    }
}

#[tauri::command]
pub async fn key_info(
    state: State<'_, AppState>,
    conn_id: String,
    key: String,
    replica: Option<usize>,
) -> AppResult<KeyInfo> {
    if is_memcached(&state, &conn_id) {
        crate::memcachedx::key_info(&state.memcached, &conn_id, &key).await
    } else {
        crate::redisx::keys::key_info(&state.manager, &conn_id, &key, replica).await
    }
}

#[tauri::command]
pub async fn get_value(
    state: State<'_, AppState>,
    conn_id: String,
    key: String,
    replica: Option<usize>,
) -> AppResult<ValueView> {
    if is_memcached(&state, &conn_id) {
        crate::memcachedx::get_value(&state.memcached, &conn_id, &key).await
    } else {
        crate::redisx::keys::get_value(&state.manager, &conn_id, &key, replica).await
    }
}

#[tauri::command]
pub async fn delete_keys(
    state: State<'_, AppState>,
    conn_id: String,
    keys: Vec<String>,
) -> AppResult<u64> {
    if is_memcached(&state, &conn_id) {
        crate::memcachedx::delete_keys(&state.memcached, &conn_id, &keys).await
    } else {
        crate::redisx::keys::delete_keys(&state.manager, &conn_id, keys).await
    }
}

#[tauri::command]
pub async fn rename_key(
    state: State<'_, AppState>,
    conn_id: String,
    source: String,
    dest: String,
) -> AppResult<()> {
    if is_memcached(&state, &conn_id) {
        Err(crate::error::AppError::new("Memcached 不支持键重命名"))
    } else {
        crate::redisx::keys::rename_key(&state.manager, &conn_id, &source, &dest).await
    }
}

#[tauri::command]
pub async fn create_key(
    state: State<'_, AppState>,
    conn_id: String,
    key: String,
    kind: String,
    field: Option<String>,
    value: String,
    score: Option<f64>,
) -> AppResult<()> {
    if is_memcached(&state, &conn_id) {
        crate::memcachedx::create_key(&state.memcached, &conn_id, &key, value.as_bytes(), -1).await
    } else {
        crate::redisx::keys::create_key(
            &state.manager,
            &conn_id,
            &key,
            &kind,
            field.as_deref(),
            &value,
            score,
        )
        .await
    }
}

#[tauri::command]
pub async fn set_ttl(
    state: State<'_, AppState>,
    conn_id: String,
    key: String,
    ttl: i64,
) -> AppResult<()> {
    if is_memcached(&state, &conn_id) {
        crate::memcachedx::set_ttl(&state.memcached, &conn_id, &key, ttl).await?;
        Ok(())
    } else {
        crate::redisx::keys::set_ttl(&state.manager, &conn_id, &key, ttl).await
    }
}

#[tauri::command]
pub async fn flush_db(state: State<'_, AppState>, conn_id: String) -> AppResult<()> {
    if is_memcached(&state, &conn_id) {
        crate::memcachedx::flush_db(&state.memcached, &conn_id).await
    } else {
        crate::redisx::keys::flush_db(&state.manager, &conn_id).await
    }
}

// ==================== 值编辑 ====================

#[tauri::command]
pub async fn set_string_value(
    state: State<'_, AppState>,
    conn_id: String,
    key: String,
    value: String,
    ttl: Option<i64>,
    encoding: String,
) -> AppResult<()> {
    if is_memcached(&state, &conn_id) {
        let bytes = if encoding == "base64" {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode(value.trim())
                .map_err(|e| crate::error::AppError::new(format!("base64 解码失败: {}", e)))?
        } else {
            value.into_bytes()
        };
        crate::memcachedx::set_string_value(
            &state.memcached,
            &conn_id,
            &key,
            &bytes,
            ttl.unwrap_or(-1),
        )
        .await
    } else {
        crate::redisx::keys::set_string_value(&state.manager, &conn_id, &key, &value, ttl, &encoding)
            .await
    }
}

#[tauri::command]
pub async fn set_hash_field(
    state: State<'_, AppState>,
    conn_id: String,
    key: String,
    field: String,
    value: String,
) -> AppResult<()> {
    crate::redisx::keys::set_hash_field(&state.manager, &conn_id, &key, &field, &value).await
}

#[tauri::command]
pub async fn delete_hash_fields(
    state: State<'_, AppState>,
    conn_id: String,
    key: String,
    fields: Vec<String>,
) -> AppResult<u64> {
    crate::redisx::keys::delete_hash_fields(&state.manager, &conn_id, &key, fields).await
}

#[tauri::command]
pub async fn push_list(
    state: State<'_, AppState>,
    conn_id: String,
    key: String,
    values: Vec<String>,
    tail: bool,
) -> AppResult<u64> {
    crate::redisx::keys::push_list(&state.manager, &conn_id, &key, values, tail).await
}

#[tauri::command]
pub async fn remove_list(
    state: State<'_, AppState>,
    conn_id: String,
    key: String,
    value: String,
    count: i64,
) -> AppResult<u64> {
    crate::redisx::keys::remove_list(&state.manager, &conn_id, &key, &value, count).await
}

#[tauri::command]
pub async fn add_set_members(
    state: State<'_, AppState>,
    conn_id: String,
    key: String,
    members: Vec<String>,
) -> AppResult<u64> {
    crate::redisx::keys::add_set_members(&state.manager, &conn_id, &key, members).await
}

#[tauri::command]
pub async fn remove_set_members(
    state: State<'_, AppState>,
    conn_id: String,
    key: String,
    members: Vec<String>,
) -> AppResult<u64> {
    crate::redisx::keys::remove_set_members(&state.manager, &conn_id, &key, members).await
}

#[tauri::command]
pub async fn add_zset_members(
    state: State<'_, AppState>,
    conn_id: String,
    key: String,
    members: Vec<(f64, String)>,
) -> AppResult<u64> {
    crate::redisx::keys::add_zset_members(&state.manager, &conn_id, &key, members).await
}

#[tauri::command]
pub async fn remove_zset_members(
    state: State<'_, AppState>,
    conn_id: String,
    key: String,
    members: Vec<String>,
) -> AppResult<u64> {
    crate::redisx::keys::remove_zset_members(&state.manager, &conn_id, &key, members).await
}

#[tauri::command]
pub async fn xadd_stream(
    state: State<'_, AppState>,
    conn_id: String,
    key: String,
    id: String,
    fields: Vec<(String, String)>,
) -> AppResult<String> {
    crate::redisx::keys::xadd_stream(&state.manager, &conn_id, &key, &id, fields).await
}

#[tauri::command]
pub async fn xdel_stream(
    state: State<'_, AppState>,
    conn_id: String,
    key: String,
    ids: Vec<String>,
) -> AppResult<u64> {
    crate::redisx::keys::xdel_stream(&state.manager, &conn_id, &key, ids).await
}

// ==================== 命令行 ====================

#[tauri::command]
pub async fn execute_command(
    state: State<'_, AppState>,
    conn_id: String,
    command: String,
    replica: Option<usize>,
) -> AppResult<CommandResult> {
    if is_memcached(&state, &conn_id) {
        crate::memcachedx::execute_command(&state.memcached, &conn_id, command).await
    } else {
        crate::redisx::console::execute(&state.manager, &conn_id, &command, replica).await
    }
}

// ==================== 服务器信息 ====================

#[tauri::command]
pub async fn get_server_info(
    state: State<'_, AppState>,
    conn_id: String,
    section: Option<String>,
) -> AppResult<ServerInfo> {
    if is_memcached(&state, &conn_id) {
        crate::memcachedx::server_info(&state.memcached, &conn_id).await
    } else {
        crate::redisx::server::get_info(&state.manager, &conn_id, section).await
    }
}

#[tauri::command]
pub async fn get_server_config(
    state: State<'_, AppState>,
    conn_id: String,
    pattern: String,
) -> AppResult<Vec<(String, String)>> {
    crate::redisx::server::get_config(&state.manager, &conn_id, &pattern).await
}

#[tauri::command]
pub async fn set_server_config(
    state: State<'_, AppState>,
    conn_id: String,
    key: String,
    value: String,
) -> AppResult<()> {
    crate::redisx::server::set_config(&state.manager, &conn_id, &key, &value).await
}

#[tauri::command]
pub async fn get_clients(
    state: State<'_, AppState>,
    conn_id: String,
) -> AppResult<Vec<ClientEntry>> {
    crate::redisx::server::get_clients(&state.manager, &conn_id).await
}

#[tauri::command]
pub async fn get_slowlog(
    state: State<'_, AppState>,
    conn_id: String,
    count: Option<i64>,
) -> AppResult<Vec<SlowlogEntry>> {
    crate::redisx::server::get_slowlog(&state.manager, &conn_id, count).await
}

#[tauri::command]
pub async fn get_topology(
    state: State<'_, AppState>,
    conn_id: String,
) -> AppResult<Vec<NodeStatus>> {
    crate::redisx::server::get_topology(&state.manager, &conn_id).await
}

#[tauri::command]
pub async fn list_databases(state: State<'_, AppState>, conn_id: String) -> AppResult<Vec<i64>> {
    if is_memcached(&state, &conn_id) {
        Ok(Vec::new())
    } else {
        crate::redisx::server::list_databases(&state.manager, &conn_id).await
    }
}

// ==================== 实时监控 ====================

#[tauri::command]
pub async fn pubsub_subscribe(
    state: State<'_, AppState>,
    conn_id: String,
    channels: Vec<String>,
    patterns: Vec<String>,
    on_event: Channel<PubSubEvent>,
) -> AppResult<()> {
    crate::redisx::stream::subscribe(&state.manager, &conn_id, channels, patterns, on_event).await
}

#[tauri::command]
pub async fn pubsub_unsubscribe(
    state: State<'_, AppState>,
    conn_id: String,
    channels: Vec<String>,
    patterns: Vec<String>,
) -> AppResult<()> {
    crate::redisx::stream::unsubscribe(&state.manager, &conn_id, channels, patterns).await
}

#[tauri::command]
pub async fn pubsub_publish(
    state: State<'_, AppState>,
    conn_id: String,
    channel: String,
    message: String,
) -> AppResult<u64> {
    crate::redisx::stream::publish(&state.manager, &conn_id, &channel, &message).await
}

#[tauri::command]
pub async fn start_monitor(
    state: State<'_, AppState>,
    conn_id: String,
    on_event: Channel<MonitorEvent>,
) -> AppResult<()> {
    crate::redisx::stream::start_monitor(&state.manager, &conn_id, on_event).await
}

#[tauri::command]
pub async fn stop_tasks(state: State<'_, AppState>, conn_id: String) -> AppResult<()> {
    let handle = state.manager.get(&conn_id)?;
    crate::redisx::stream::stop_all(&handle).await;
    Ok(())
}
