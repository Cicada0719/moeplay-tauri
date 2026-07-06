use crate::locale::{self, LocaleEmulatorManager};
use crate::process_monitor::{ProcessMonitor, RunningGameInfo};
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub fn detect_game_engine(game_dir: String) -> Result<locale::EngineConfig, String> {
    let dir = PathBuf::from(&game_dir);
    if !dir.is_dir() {
        return Err("目录不存在".into());
    }
    locale::EngineLibrary::detect_engine(&dir).ok_or_else(|| "未检测到已知游戏引擎".into())
}

#[tauri::command]
pub fn get_locale_emulator_status(lem: State<'_, LocaleEmulatorManager>) -> serde_json::Value {
    serde_json::json!({
        "le_available": lem.is_le_available(),
        "ntleas_available": lem.is_ntleas_available(),
        "le_path": lem.le_path().map(|p| p.to_string_lossy().to_string()),
        "ntleas_path": lem.ntleas_path().map(|p| p.to_string_lossy().to_string()),
    })
}

#[tauri::command]
pub fn set_custom_le_path(
    lem: State<'_, LocaleEmulatorManager>,
    path: Option<String>,
) -> Result<(), String> {
    lem.set_custom_le_path(path.map(PathBuf::from));
    tracing::info!("Custom LE path updated");
    Ok(())
}

#[tauri::command]
pub fn get_running_games(monitor: State<'_, ProcessMonitor>) -> Vec<RunningGameInfo> {
    monitor.running_games()
}

#[tauri::command]
pub fn pick_directory() -> Result<String, String> {
    let dir = rfd::FileDialog::new().set_title("选择目录").pick_folder();

    match dir {
        Some(path) => Ok(path.to_string_lossy().to_string()),
        None => Err("已取消".to_string()),
    }
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| format!("打开 URL 失败: {}", e))
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("路径不存在: {}", path));
    }

    // 拒绝包含 .. 的路径
    for c in p.components() {
        if matches!(c, std::path::Component::ParentDir) {
            return Err("路径包含非法的 .. 片段".to_string());
        }
    }

    // 限制只能打开用户主目录、数据目录等已知安全位置
    let mut scope = crate::security::app_data_scope().unwrap_or_default();
    if let Some(home) = dirs::home_dir() {
        scope.allow(home);
    }
    if let Some(dl) = dirs::download_dir() {
        scope.allow(dl);
    }

    scope
        .resolve(&p)
        .map_err(|_| "路径不在允许范围内".to_string())?;

    open::that(&p).map_err(|e| format!("打开路径失败: {}", e))
}

#[tauri::command]
pub fn set_autostart(enabled: bool, startup_mode: String) -> Result<String, String> {
    crate::autostart::set_autostart(enabled, &startup_mode)?;
    Ok(if enabled {
        format!("已启用开机自启动（模式: {}）", startup_mode)
    } else {
        "已关闭开机自启动".to_string()
    })
}

#[tauri::command]
pub fn get_autostart_status() -> Result<crate::autostart::AutostartStatus, String> {
    crate::autostart::get_autostart_status()
}
