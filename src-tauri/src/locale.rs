// Locale Emulator 集成 + 游戏引擎参数库

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// 支持的区域模拟方案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocaleMethod {
    LocaleEmulator,
    Ntleas,
    AppLocale,
    DirectLaunch,
}

/// 游戏引擎类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameEngine {
    Kirikiri,
    NScripter,
    RPGMaker,
    Unity,
    RenPy,
    CatSystem2,
    Artemis,
    Ethornell,
    Tyrano,
    WOLF,
    Malie,
    Luna,
    Other,
}

/// 引擎参数配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub engine: GameEngine,
    pub name: String,
    pub executables: Vec<String>,
    pub signature_files: Vec<String>,
    pub locale_methods: Vec<LocaleMethod>,
    pub launch_args: Vec<String>,
    pub notes: String,
}

/// 引擎参数库 (13种)
pub struct EngineLibrary;

impl EngineLibrary {
    pub fn get_all() -> Vec<EngineConfig> {
        vec![
            EngineConfig {
                engine: GameEngine::Kirikiri,
                name: "吉里吉里 (Kirikiri)".into(),
                executables: vec![
                    "krkr.exe".into(),
                    "kirikiri.exe".into(),
                    "kagamin.exe".into(),
                ],
                signature_files: vec!["data.xp3".into(), "plugin".into()],
                locale_methods: vec![LocaleMethod::LocaleEmulator, LocaleMethod::Ntleas],
                launch_args: vec!["-start".into()],
                notes: "日文区域启动，否则可能乱码".into(),
            },
            EngineConfig {
                engine: GameEngine::NScripter,
                name: "NScripter".into(),
                executables: vec!["nscr.exe".into(), "nscripter.exe".into()],
                signature_files: vec!["nscript.dat".into()],
                locale_methods: vec![
                    LocaleMethod::LocaleEmulator,
                    LocaleMethod::Ntleas,
                    LocaleMethod::AppLocale,
                ],
                launch_args: vec![],
                notes: "需要日文区域或转区工具".into(),
            },
            EngineConfig {
                engine: GameEngine::RPGMaker,
                name: "RPG Maker".into(),
                executables: vec!["Game.exe".into(), "game.exe".into()],
                signature_files: vec![
                    "Game.rgss3a".into(),
                    "Game.rgss2a".into(),
                    "Data/Scripts.rvdata2".into(),
                ],
                locale_methods: vec![LocaleMethod::DirectLaunch],
                launch_args: vec![],
                notes: "通常不需要转区".into(),
            },
            EngineConfig {
                engine: GameEngine::Unity,
                name: "Unity".into(),
                executables: vec![],
                signature_files: vec![
                    "UnityPlayer.dll".into(),
                    "MonoBleedingEdge".into(),
                    "GameAssembly.dll".into(),
                ],
                locale_methods: vec![LocaleMethod::DirectLaunch],
                launch_args: vec![
                    "-screen-fullscreen".into(),
                    "0".into(),
                    "-window-mode".into(),
                    "exclusive".into(),
                ],
                notes: "支持窗口/全屏切换".into(),
            },
            EngineConfig {
                engine: GameEngine::RenPy,
                name: "Ren'Py".into(),
                executables: vec![],
                signature_files: vec!["renpy".into(), "lib/python".into()],
                locale_methods: vec![LocaleMethod::DirectLaunch],
                launch_args: vec![],
                notes: "多平台支持".into(),
            },
            EngineConfig {
                engine: GameEngine::CatSystem2,
                name: "CatSystem2".into(),
                executables: vec!["cs2.exe".into(), "catsystem2.exe".into()],
                signature_files: vec!["cs2".into(), "cst".into()],
                locale_methods: vec![LocaleMethod::LocaleEmulator, LocaleMethod::Ntleas],
                launch_args: vec![],
                notes: "需要日文区域".into(),
            },
            EngineConfig {
                engine: GameEngine::Artemis,
                name: "Artemis".into(),
                executables: vec!["artemis.exe".into(), "arc.exe".into()],
                signature_files: vec!["arc".into()],
                locale_methods: vec![LocaleMethod::LocaleEmulator],
                launch_args: vec![],
                notes: "需要日文区域".into(),
            },
            EngineConfig {
                engine: GameEngine::Ethornell,
                name: "BGI/Ethonell".into(),
                executables: vec!["bgi.exe".into(), "ethornell.exe".into()],
                signature_files: vec!["arc.nsa".into(), "bgm".into()],
                locale_methods: vec![LocaleMethod::LocaleEmulator, LocaleMethod::Ntleas],
                launch_args: vec![],
                notes: "需要日文区域".into(),
            },
            EngineConfig {
                engine: GameEngine::Tyrano,
                name: "TyranoBuilder".into(),
                executables: vec!["tyrano.exe".into()],
                signature_files: vec!["tyrano".into(), "index.html".into()],
                locale_methods: vec![LocaleMethod::DirectLaunch],
                launch_args: vec![],
                notes: "HTML5 引擎，通常不需要转区".into(),
            },
            EngineConfig {
                engine: GameEngine::WOLF,
                name: "WOLF RPG Editor".into(),
                executables: vec!["WOLF.exe".into(), "wolf.exe".into()],
                signature_files: vec!["Data.wolf".into(), "BasicData".into()],
                locale_methods: vec![LocaleMethod::LocaleEmulator],
                launch_args: vec![],
                notes: "需要日文区域".into(),
            },
            EngineConfig {
                engine: GameEngine::Malie,
                name: "Malie".into(),
                executables: vec!["malie.exe".into()],
                signature_files: vec!["malie.dll".into()],
                locale_methods: vec![LocaleMethod::LocaleEmulator],
                launch_args: vec![],
                notes: "需要日文区域".into(),
            },
            EngineConfig {
                engine: GameEngine::Luna,
                name: "LUNA".into(),
                executables: vec!["luna.exe".into(), "lunascape.exe".into()],
                signature_files: vec!["luna.dll".into()],
                locale_methods: vec![LocaleMethod::LocaleEmulator],
                launch_args: vec![],
                notes: "需要日文区域".into(),
            },
            EngineConfig {
                engine: GameEngine::Other,
                name: "其他引擎".into(),
                executables: vec![],
                signature_files: vec![],
                locale_methods: vec![LocaleMethod::DirectLaunch, LocaleMethod::LocaleEmulator],
                launch_args: vec![],
                notes: "未知引擎，尝试直接启动或 LE".into(),
            },
        ]
    }

    pub fn detect_engine(game_dir: &std::path::Path) -> Option<EngineConfig> {
        for config in Self::get_all() {
            if config.engine == GameEngine::Other {
                continue;
            }
            for sig_file in &config.signature_files {
                if game_dir.join(sig_file).exists() {
                    return Some(config.clone());
                }
            }
        }
        Self::get_all().last().cloned()
    }

    pub fn find_executable(game_dir: &std::path::Path, config: &EngineConfig) -> Option<PathBuf> {
        for exe_name in &config.executables {
            let p = game_dir.join(exe_name);
            if p.exists() {
                return Some(p);
            }
        }
        if let Ok(entries) = std::fs::read_dir(game_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "exe") {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase();
                    if !name.contains("unins")
                        && !name.contains("uninstall")
                        && !name.contains("setup")
                    {
                        return Some(path);
                    }
                }
            }
        }
        None
    }

    pub fn score_executable(
        exe_path: &std::path::Path,
        game_dir: &std::path::Path,
        config: &EngineConfig,
    ) -> u32 {
        let mut score = 0u32;
        let name = exe_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        for known in &config.executables {
            if name == known.to_lowercase() {
                score += 50;
                break;
            }
        }
        if let Some(dn) = game_dir.file_name() {
            let dn = dn.to_string_lossy().to_lowercase();
            if name.contains(&dn) || dn.contains(&name.replace(".exe", "")) {
                score += 30;
            }
        }
        if let Ok(meta) = std::fs::metadata(exe_path) {
            if meta.len() > 1_000_000 {
                score += 10;
            }
        }
        if exe_path.parent() == Some(game_dir) {
            score += 10;
        }
        score
    }
}

// ============================================================================
// Locale Emulator 管理器
// ============================================================================

/// Locale Emulator 管理器（线程安全：内部 Mutex 保护可写字段）。
pub struct LocaleEmulatorManager {
    le_path: Mutex<Option<PathBuf>>,
    ntleas_path: Mutex<Option<PathBuf>>,
}

impl Default for LocaleEmulatorManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LocaleEmulatorManager {
    pub fn new() -> Self {
        Self {
            le_path: Mutex::new(Self::discover_le()),
            ntleas_path: Mutex::new(Self::discover_ntleas()),
        }
    }

    fn discover_le() -> Option<PathBuf> {
        for path in &[
            r"C:\Program Files\Locale Emulator\LEProc.exe",
            r"C:\Program Files (x86)\Locale Emulator\LEProc.exe",
            r"C:\Locale Emulator\LEProc.exe",
        ] {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }
        None
    }

    fn discover_ntleas() -> Option<PathBuf> {
        for path in &[
            r"C:\Program Files\NTLEAS\ntleasWin.exe",
            r"C:\Program Files (x86)\NTLEAS\ntleasWin.exe",
        ] {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }
        None
    }

    pub fn get_best_method(&self, config: &EngineConfig) -> LocaleMethod {
        let le_ok = self.le_path.lock().unwrap().is_some();
        let nt_ok = self.ntleas_path.lock().unwrap().is_some();
        for method in &config.locale_methods {
            match method {
                LocaleMethod::LocaleEmulator if le_ok => return LocaleMethod::LocaleEmulator,
                LocaleMethod::Ntleas if nt_ok => return LocaleMethod::Ntleas,
                LocaleMethod::DirectLaunch => return LocaleMethod::DirectLaunch,
                _ => {}
            }
        }
        LocaleMethod::DirectLaunch
    }

    pub fn launch_with_locale(
        &self,
        exe_path: &std::path::Path,
        method: &LocaleMethod,
        _locale: &str,
    ) -> Result<std::process::Child, String> {
        let wd = exe_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        match method {
            LocaleMethod::LocaleEmulator => {
                let le = self.le_path.lock().unwrap();
                let le = le.as_ref().ok_or("Locale Emulator 未安装")?;
                std::process::Command::new(le)
                    .args(["-runas", &exe_path.to_string_lossy()])
                    .current_dir(wd)
                    .spawn()
                    .map_err(|e| format!("LE 启动失败: {}", e))
            }
            LocaleMethod::Ntleas => {
                let nt = self.ntleas_path.lock().unwrap();
                let ntleas = nt.as_ref().ok_or("NTLEAS 未安装")?;
                std::process::Command::new(ntleas)
                    .args([&exe_path.to_string_lossy(), "C"])
                    .current_dir(wd)
                    .spawn()
                    .map_err(|e| format!("NTLEAS 启动失败: {}", e))
            }
            _ => std::process::Command::new(exe_path)
                .current_dir(wd)
                .spawn()
                .map_err(|e| format!("直接启动失败: {}", e)),
        }
    }

    pub fn is_le_available(&self) -> bool {
        self.le_path.lock().unwrap().is_some()
    }
    pub fn is_ntleas_available(&self) -> bool {
        self.ntleas_path.lock().unwrap().is_some()
    }
    pub fn le_path(&self) -> Option<PathBuf> {
        self.le_path.lock().unwrap().clone()
    }
    pub fn ntleas_path(&self) -> Option<PathBuf> {
        self.ntleas_path.lock().unwrap().clone()
    }
    pub fn set_custom_le_path(&self, p: Option<PathBuf>) {
        *self.le_path.lock().unwrap() = p;
    }
    pub fn set_custom_ntleas_path(&self, p: Option<PathBuf>) {
        *self.ntleas_path.lock().unwrap() = p;
    }
}

// ============================================================================
// 启动结果
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchResult {
    pub session_id: String,
    pub engine: Option<String>,
    pub engine_name: Option<String>,
    pub locale_method: String,
    pub pid: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_kirikiri_engine() {
        let tmp = std::env::temp_dir().join("moegame_test_kirikiri");
        std::fs::create_dir_all(&tmp).unwrap();
        // 创建特征文件
        std::fs::write(tmp.join("data.xp3"), b"fake xp3").unwrap();
        let result = EngineLibrary::detect_engine(&tmp);
        assert!(result.is_some());
        assert_eq!(result.unwrap().engine, GameEngine::Kirikiri);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_detect_nscripter_engine() {
        let tmp = std::env::temp_dir().join("moegame_test_nscr");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("nscript.dat"), b"fake nscript").unwrap();
        let result = EngineLibrary::detect_engine(&tmp);
        assert!(result.is_some());
        assert_eq!(result.unwrap().engine, GameEngine::NScripter);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_detect_unknown_engine() {
        let tmp = std::env::temp_dir().join("moegame_test_unknown");
        std::fs::create_dir_all(&tmp).unwrap();
        let result = EngineLibrary::detect_engine(&tmp);
        assert!(result.is_some());
        assert_eq!(result.unwrap().engine, GameEngine::Other);
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_score_exe_higher_for_known_name() {
        let tmp = std::env::temp_dir().join("moegame_test_score");
        std::fs::create_dir_all(&tmp).unwrap();
        let exe = tmp.join("krkr.exe");
        std::fs::write(&exe, vec![0u8; 2_000_000]).unwrap();
        let config = EngineLibrary::get_all()
            .into_iter()
            .find(|c| c.engine == GameEngine::Kirikiri)
            .unwrap();
        let score = EngineLibrary::score_executable(&exe, &tmp, &config);
        assert!(
            score >= 50,
            "known exe name should score >= 50, got {}",
            score
        );
        std::fs::remove_dir_all(&tmp).ok();
    }
}
