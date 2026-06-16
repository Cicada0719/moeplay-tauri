//! 外部播放器支持 —— 启动 mpv / VLC / PotPlayer 等外部播放器
//!
//! 支持的播放器:
//! - mpv: 通过 PATH 或常见安装路径检测
//! - VLC: 通过 PATH 或常见安装路径检测
//! - PotPlayer: Windows 注册表 / 常见路径检测

use serde::{Deserialize, Serialize};
use std::process::Command;

/// 可用外部播放器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalPlayerInfo {
    pub name: String,
    pub display_name: String,
    pub available: bool,
}

/// 获取已安装的外部播放器列表
pub fn get_available_players() -> Vec<ExternalPlayerInfo> {
    vec![
        ExternalPlayerInfo {
            name: "mpv".into(),
            display_name: "mpv".into(),
            available: is_player_available("mpv"),
        },
        ExternalPlayerInfo {
            name: "vlc".into(),
            display_name: "VLC".into(),
            available: is_player_available("vlc"),
        },
        ExternalPlayerInfo {
            name: "potplayer".into(),
            display_name: "PotPlayer".into(),
            available: is_potplayer_available(),
        },
    ]
}

/// 检查播放器是否可用（检查 PATH）
fn is_player_available(name: &str) -> bool {
    let cmd = if cfg!(target_os = "windows") {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };
    // 尝试 which / where
    if cfg!(target_os = "windows") {
        Command::new("where")
            .arg(&cmd)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        Command::new("which")
            .arg(name)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

/// 检查 PotPlayer 是否可用（Windows 特有路径检测）
fn is_potplayer_available() -> bool {
    if !cfg!(target_os = "windows") {
        return false;
    }
    // 常见安装路径
    let paths = [
        r"C:\Program Files\DAUM\PotPlayer\PotPlayerMini64.exe",
        r"C:\Program Files (x86)\DAUM\PotPlayer\PotPlayerMini.exe",
        r"C:\Program Files\DAUM\PotPlayer\PotPlayerMini.exe",
    ];
    for p in &paths {
        if std::path::Path::new(p).exists() {
            return true;
        }
    }
    // 也检查 PATH 中是否有 PotPlayerMini64
    Command::new("where")
        .arg("PotPlayerMini64.exe")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// 启动外部播放器
pub fn launch_external_player(
    url: &str,
    player: &str,
    referer: Option<&str>,
) -> Result<String, String> {
    let player_lower = player.to_lowercase();
    match player_lower.as_str() {
        "mpv" => launch_mpv(url, referer),
        "vlc" => launch_vlc(url, referer),
        "potplayer" => launch_potplayer(url, referer),
        _ => Err(format!("不支持的播放器: {}", player)),
    }
}

fn launch_mpv(url: &str, referer: Option<&str>) -> Result<String, String> {
    let mut cmd = Command::new("mpv");
    if let Some(ref r) = referer {
        if !r.is_empty() {
            cmd.arg(format!("--referrer={}", r));
        }
    }
    cmd.arg(url);
    spawn_detached(&mut cmd, "mpv")
}

fn launch_vlc(url: &str, referer: Option<&str>) -> Result<String, String> {
    let vlc_cmd = if cfg!(target_os = "windows") {
        "vlc.exe"
    } else {
        "vlc"
    };
    let mut cmd = if is_player_available("vlc") {
        Command::new(vlc_cmd)
    } else {
        // 常见 VLC 安装路径
        let vlc_path = r"C:\Program Files\VideoLAN\VLC\vlc.exe";
        if std::path::Path::new(vlc_path).exists() {
            Command::new(vlc_path)
        } else {
            return Err("VLC 未找到".into());
        }
    };
    if let Some(ref r) = referer {
        if !r.is_empty() {
            cmd.arg(format!("--http-referrer={}", r));
        }
    }
    cmd.arg(url);
    spawn_detached(&mut cmd, "vlc")
}

fn launch_potplayer(url: &str, _referer: Option<&str>) -> Result<String, String> {
    let pot_path = find_potplayer_exe().ok_or_else(|| "PotPlayer 未找到".to_string())?;
    let mut cmd = Command::new(&pot_path);
    cmd.arg(url);
    spawn_detached(&mut cmd, "PotPlayer")
}

fn find_potplayer_exe() -> Option<String> {
    // 尝试 PATH
    if Command::new("where")
        .arg("PotPlayerMini64.exe")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return Some("PotPlayerMini64.exe".into());
    }
    // 常见路径
    let paths = [
        r"C:\Program Files\DAUM\PotPlayer\PotPlayerMini64.exe",
        r"C:\Program Files (x86)\DAUM\PotPlayer\PotPlayerMini.exe",
        r"C:\Program Files\DAUM\PotPlayer\PotPlayerMini.exe",
    ];
    for p in &paths {
        if std::path::Path::new(p).exists() {
            return Some(p.to_string());
        }
    }
    None
}

/// 以 detached 方式启动进程（不阻塞主进程）
fn spawn_detached(cmd: &mut Command, name: &str) -> Result<String, String> {
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x00000008); // DETACHED_PROCESS
    }

    cmd.spawn()
        .map(|_| format!("已启动 {} 播放器", name))
        .map_err(|e| format!("启动 {} 失败: {}", name, e))
}
