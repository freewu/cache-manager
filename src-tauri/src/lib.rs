pub mod commands;
pub mod error;
pub mod memcachedx;
pub mod model;
pub mod redisx;
pub mod single_instance;
pub mod store;
pub mod tray;

use std::sync::Arc;

use commands::AppState;
use memcachedx::MemcachedManager;
use redisx::RedisManager;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            manager: Arc::new(RedisManager::new()),
            memcached: Arc::new(MemcachedManager::new()),
        })
        // 单实例：已有实例时通知其激活并退出本进程
        .setup(|app| {
            if !crate::single_instance::try_acquire(app.handle()) {
                app.handle().exit(0);
                return Ok(());
            }
            crate::tray::setup(app)
        })
        // 点击关闭按钮 → 按设置决定最小化到托盘或直接退出
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let minimize = crate::store::load_settings(&window.app_handle()).minimize_to_tray;
                if minimize {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            // 连接管理
            commands::test_connection,
            commands::connect_connection,
            commands::disconnect_connection,
            commands::disconnect_all,
            commands::connection_status,
            commands::list_connections,
            commands::switch_database,
            commands::load_saved_connections,
            commands::save_connections,
            commands::export_connections,
            commands::import_connections,
            commands::update_tray_menu,
            commands::get_app_settings,
            commands::set_app_settings,
            // 键操作
            commands::scan_keys,
            commands::key_info,
            commands::get_value,
            commands::delete_keys,
            commands::rename_key,
            commands::create_key,
            commands::set_ttl,
            commands::flush_db,
            // 值编辑
            commands::set_string_value,
            commands::set_hash_field,
            commands::delete_hash_fields,
            commands::push_list,
            commands::remove_list,
            commands::add_set_members,
            commands::remove_set_members,
            commands::add_zset_members,
            commands::remove_zset_members,
            commands::xadd_stream,
            commands::xdel_stream,
            // 命令行
            commands::execute_command,
            // 服务器信息
            commands::get_server_info,
            commands::get_server_config,
            commands::set_server_config,
            commands::get_clients,
            commands::get_slowlog,
            commands::get_topology,
            commands::list_databases,
            // 实时监控
            commands::pubsub_subscribe,
            commands::pubsub_unsubscribe,
            commands::pubsub_publish,
            commands::start_monitor,
            commands::stop_tasks,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
