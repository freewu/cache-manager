//! 单实例保证（Windows 端口锁方案，零外部依赖）
//!
//! 原理：启动时尝试绑定本机固定端口。
//! - 绑定成功 → 成为唯一实例，后台线程监听该端口；
//!   当有新进程尝试启动时会连上来，此时激活主窗口。
//! - 绑定失败 → 已有实例在运行，向其发送激活信号后本进程退出。

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use tauri::AppHandle;

const INSTANCE_PORT: u16 = 17453;

/// 尝试成为唯一实例。
/// 返回 `true` 表示本进程是主实例（已接管监听）；
/// 返回 `false` 表示已有实例在运行（已发送激活请求，调用方应立即退出）。
pub fn try_acquire(app: &AppHandle) -> bool {
    match TcpListener::bind(("127.0.0.1", INSTANCE_PORT)) {
        Ok(listener) => {
            // 后台线程：等待激活请求 → 显示并聚焦主窗口
            let handle = app.clone();
            std::thread::Builder::new()
                .name("single-instance-listener".into())
                .spawn(move || {
                    for stream in listener.incoming() {
                        let Ok(mut s) = stream else { continue };
                        let _ = s.read(&mut [0u8; 4]);
                        let _ = crate::tray::show_main_window(&handle);
                        let _ = s.write_all(b"ok");
                    }
                })
                .ok();
            true
        }
        Err(_) => {
            // 已有实例：通知其激活
            if let Ok(mut s) = TcpStream::connect(("127.0.0.1", INSTANCE_PORT)) {
                let _ = s.write_all(b"show");
                let mut buf = [0u8; 2];
                let _ = s.read(&mut buf);
            }
            false
        }
    }
}

/// 供测试：释放端口（进程退出时自动释放，一般无需调用）
pub fn _unused() -> u16 {
    INSTANCE_PORT
}
