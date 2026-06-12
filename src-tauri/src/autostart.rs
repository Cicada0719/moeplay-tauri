// 萌游 MoeGame · 开机自启管理
//
// Windows: 注册表 HKCU\Software\Microsoft\Windows\CurrentVersion\Run
// Linux/macOS: ~/.config/autostart/ 下 .desktop 文件
//
// 自启时通过命令行参数 --startup-mode 传递启动模式

use serde::{Deserialize, Serialize};
#[cfg(not(windows))]
use std::path::PathBuf;

/// 开机自启状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutostartStatus {
    pub enabled: bool,
    pub startup_mode: String,
    pub exe_path: String,
}

/// 获取当前可执行文件路径
fn current_exe_path() -> Result<String, String> {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("无法获取 exe 路径: {}", e))
}

/// 设置开机自启
/// `startup_mode`: "dashboard" | "big-picture"
pub fn set_autostart(enabled: bool, startup_mode: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        set_autostart_windows(enabled, startup_mode)
    }
    #[cfg(not(windows))]
    {
        set_autostart_unix(enabled, startup_mode)
    }
}

/// 获取当前开机自启状态
pub fn get_autostart_status() -> Result<AutostartStatus, String> {
    let exe = current_exe_path()?;

    #[cfg(windows)]
    {
        let enabled = is_autostart_enabled_windows();
        // Can't easily read the startup mode from registry key — just return default
        Ok(AutostartStatus {
            enabled,
            startup_mode: "dashboard".to_string(),
            exe_path: exe,
        })
    }
    #[cfg(not(windows))]
    {
        let enabled = is_autostart_enabled_unix();
        Ok(AutostartStatus {
            enabled,
            startup_mode: "dashboard".to_string(),
            exe_path: exe,
        })
    }
}

// ============================================================================
// Windows 实现
// ============================================================================

#[cfg(windows)]
fn set_autostart_windows(enabled: bool, startup_mode: &str) -> Result<(), String> {
    let exe = current_exe_path()?;
    let key_path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let value_name = "MoeGame";

    if enabled {
        // Add registry key
        let exe_with_args = format!("\"{}\" --startup-mode {}", exe, startup_mode);

        // Use powershell to set registry (zero dependency)
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile", "-Command",
                &format!(
                    r#"New-Item -Path "HKCU:\{}" -Force | Out-Null; Set-ItemProperty -Path "HKCU:\{}" -Name "{}" -Value "{}" -Force"#,
                    key_path, key_path, value_name, exe_with_args
                ),
            ])
            .output()
            .map_err(|e| format!("执行注册表写入失败: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("注册表写入失败: {}", stderr));
        }
        tracing::info!(exe = %exe_with_args, "Autostart enabled (Windows registry)");
    } else {
        // Remove registry key
        let _ = std::process::Command::new("powershell")
            .args([
                "-NoProfile", "-Command",
                &format!(
                    r#"Remove-ItemProperty -Path "HKCU:\{}" -Name "{}" -ErrorAction SilentlyContinue"#,
                    key_path, value_name
                ),
            ])
            .output()
            .map_err(|e| format!("执行注册表删除失败: {}", e))?;

        tracing::info!("Autostart disabled (Windows registry)");
    }
    Ok(())
}

#[cfg(windows)]
fn is_autostart_enabled_windows() -> bool {
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile", "-Command",
            r#"Get-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "MoeGame" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty MoeGame"#,
        ])
        .output()
        .ok();

    if let Some(out) = output {
        if out.status.success() {
            let val = String::from_utf8_lossy(&out.stdout).trim().to_string();
            return !val.is_empty();
        }
    }
    false
}

// ============================================================================
// Linux/macOS 实现
// ============================================================================

#[cfg(not(windows))]
fn set_autostart_unix(enabled: bool, startup_mode: &str) -> Result<(), String> {
    let exe = current_exe_path()?;
    let autostart_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("autostart");

    std::fs::create_dir_all(&autostart_dir)
        .map_err(|e| format!("无法创建 autostart 目录: {}", e))?;

    let desktop_file = autostart_dir.join("moeplay.desktop");

    if enabled {
        let content = format!(
            r#"[Desktop Entry]
Type=Application
Name=萌游 MoeGame
Exec={} --startup-mode {}
StartupNotify=false
Terminal=false
Categories=Game;
Comment=Galgame 游戏管理器
"#,
            exe, startup_mode
        );
        std::fs::write(&desktop_file, &content)
            .map_err(|e| format!("写入 .desktop 文件失败: {}", e))?;

        // Set executable permission on desktop file
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&desktop_file)
                .map_err(|e| format!("{}", e))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&desktop_file, perms).ok();
        }

        tracing::info!("Autostart enabled (Unix .desktop file)");
    } else {
        if desktop_file.exists() {
            std::fs::remove_file(&desktop_file)
                .map_err(|e| format!("删除 .desktop 文件失败: {}", e))?;
        }
        tracing::info!("Autostart disabled (Unix .desktop file removed)");
    }
    Ok(())
}

#[cfg(not(windows))]
fn is_autostart_enabled_unix() -> bool {
    let autostart_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("autostart");
    autostart_dir.join("moeplay.desktop").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_exe_path() {
        let path = current_exe_path();
        assert!(path.is_ok());
        assert!(path.unwrap().contains("moeplay"));
    }
}
