//! 系统托盘：显示/隐藏主窗口、快速连接、退出

use tauri::{
    image::Image,
    menu::{IconMenuItemBuilder, Menu, MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Emitter, Manager, Wry,
};

use crate::commands::AppState;
use crate::model::ConnMode;

/// 连接类型图标：Redis → 红 Cube，Memcached → 青 Grid
fn conn_icon(mode: &ConnMode) -> Image<'static> {
    let bytes: &[u8] = match mode {
        ConnMode::Memcached => include_bytes!("../icons/memcached-tray.png"),
        _ => include_bytes!("../icons/redis-tray.png"),
    };
    Image::from_bytes(bytes).expect("内嵌托盘图标无效")
}

/// 构建托盘菜单：显示主窗口 + 设置 + 已保存连接（快速连接）+ 退出
pub fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let show = MenuItem::with_id(app, "tray-show", "显示主窗口", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "tray-settings", "设置…", true, None::<&str>)?;
    let mut builder = MenuBuilder::new(app).item(&show).separator().item(&settings);

    let connections = crate::store::load(app).unwrap_or_default();
    if !connections.is_empty() {
        builder = builder.separator();
        for cfg in &connections {
            let label = format!("{}  [{}]", cfg.name, cfg.display_url());
            let item = IconMenuItemBuilder::with_id(
                format!("tray-connect:{}", cfg.id),
                label,
            )
            .icon(conn_icon(&cfg.mode))
            .build(app)?;
            builder = builder.item(&item);
        }
    }

    let quit = MenuItem::with_id(app, "tray-quit", "退出", true, None::<&str>)?;
    builder.separator().item(&quit).build()
}

/// 显示并聚焦主窗口
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// 托盘菜单点击处理
fn on_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "tray-show" => show_main_window(app),
        "tray-settings" => {
            show_main_window(app);
            let _ = app.emit("tray:settings", ());
        }
        "tray-quit" => app.exit(0),
        id if id.starts_with("tray-connect:") => {
            let conn_id = id.trim_start_matches("tray-connect:").to_string();
            let manager = app.state::<AppState>().manager.clone();
            let app = app.clone();
            // 后台连接（异步），完成后通知前端跳转
            tauri::async_runtime::spawn(async move {
                let configs = crate::store::load(&app).unwrap_or_default();
                if let Some(cfg) = configs.into_iter().find(|c| c.id == conn_id) {
                    match manager.connect(cfg).await {
                        Ok(_) => {
                            let _ = app.emit("tray:connect", conn_id);
                        }
                        Err(e) => {
                            let _ = app.emit("tray:connect-error", format!("{}", e));
                        }
                    }
                } else {
                    let _ = app.emit("tray:connect-error", "连接配置不存在");
                }
                show_main_window(&app);
            });
        }
        _ => {}
    }
}

/// 重建托盘菜单（保存/删除连接后调用）
pub fn update_tray_menu(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        match build_menu(app) {
            Ok(menu) => {
                let _ = tray.set_menu(Some(menu));
            }
            Err(e) => eprintln!("更新托盘菜单失败: {e}"),
        }
    }
}

/// 在 setup 阶段创建托盘
pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_menu(app.handle())?;

    let builder = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Cache Manager - Redis 管理工具")
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(|tray, event| {
            // 左键单击显示窗口
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    let builder = match app.default_window_icon() {
        Some(icon) => builder.icon(icon.clone()),
        None => builder,
    };

    let tray = builder.build(app)?;
    // 持有 TrayIcon 防止被 drop 移除
    app.manage(tray);
    Ok(())
}
