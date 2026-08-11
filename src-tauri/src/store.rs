use std::fs;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};
use crate::model::ConnConfig;

const FILE_NAME: &str = "connections.json";
const SETTINGS_FILE: &str = "settings.json";

fn config_path(app: &AppHandle) -> AppResult<std::path::PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::new(format!("无法获取配置目录: {}", e)))?;
    Ok(dir.join(FILE_NAME))
}

fn settings_path(app: &AppHandle) -> AppResult<std::path::PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::new(format!("无法获取配置目录: {}", e)))?;
    Ok(dir.join(SETTINGS_FILE))
}

/// 应用设置（持久化到 settings.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    /// 关闭主窗口时最小化到系统托盘（false 则直接退出）
    pub minimize_to_tray: bool,
    /// 连接导出默认目录（空则用系统下载目录）
    pub export_dir: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            minimize_to_tray: true,
            export_dir: None,
        }
    }
}

/// 读取应用设置
pub fn load_settings(app: &AppHandle) -> AppSettings {
    let path = match settings_path(app) {
        Ok(p) => p,
        Err(_) => return AppSettings::default(),
    };
    if !path.exists() {
        return AppSettings::default();
    }
    match fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

/// 保存应用设置
pub fn save_settings(app: &AppHandle, settings: &AppSettings) -> AppResult<()> {
    let path = settings_path(app)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let data = serde_json::to_string_pretty(settings)?;
    fs::write(&path, data)?;
    Ok(())
}

/// 读取已保存的连接配置
pub fn load(app: &AppHandle) -> AppResult<Vec<ConnConfig>> {
    let path = config_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&path)?;
    if data.trim().is_empty() {
        return Ok(Vec::new());
    }
    let configs: Vec<ConnConfig> = serde_json::from_str(&data)
        .map_err(|e| AppError::new(format!("连接配置文件解析失败: {}", e)))?;
    Ok(configs)
}

/// 保存连接配置
pub fn save(app: &AppHandle, configs: &[ConnConfig]) -> AppResult<()> {
    let path = config_path(app)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let data = serde_json::to_string_pretty(configs)?;
    fs::write(&path, data)?;
    Ok(())
}
