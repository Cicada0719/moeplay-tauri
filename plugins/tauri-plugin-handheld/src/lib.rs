use serde::{Deserialize, Serialize};
use tauri::{plugin::TauriPlugin, Runtime};

#[cfg(target_os = "android")]
use tauri::plugin::PluginHandle;
#[cfg(target_os = "android")]
use tauri::Manager;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(target_os = "android")]
    #[error(transparent)]
    MobilePlugin(#[from] tauri::plugin::mobile::PluginInvokeError),
    #[error("handheld bridge is only available on Android")]
    Unsupported,
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// 已安装的模拟器应用（PackageManager 探测结果）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledEmulator {
    pub package_name: String,
    pub label: String,
    pub version_name: String,
    /// 该模拟器适合的平台 ID 列表（与内置 emulator.yaml 的 platform 对齐）。
    pub platforms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListEmulatorsResponse {
    pub emulators: Vec<InstalledEmulator>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchCandidate {
    /// retroarch = RetroArch 活动 + ROM/LIBRETRO/CONFIGFILE extras；
    /// view = ACTION_VIEW + FileProvider content URI；menu = 仅唤起主界面。
    pub mode: String,
    pub package: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub core: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchGameRequest {
    pub package_name: String,
    pub rom_path: String,
    /// 有序启动候选链（空 = 走 package_name 的旧式 view→launcher 两级策略）。
    #[serde(default)]
    pub candidates: Vec<LaunchCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchGameResponse {
    pub launched: bool,
    /// 实际生效的启动策略（view-intent / launcher-intent），便于诊断。
    pub strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllFilesAccessResponse {
    pub granted: bool,
}

/// 存储卷 ROM 目录候选（StorageManager 探测）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RomRootCandidate {
    pub path: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverRomRootsResponse {
    pub roots: Vec<RomRootCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemBarsRequest {
    pub immersive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemBarsResponse {
    pub immersive: bool,
}

#[cfg(target_os = "android")]
struct State<R: Runtime>(PluginHandle<R>);

/// 供应用内其他 Rust 命令（如 `launch_game`）调用插件能力的扩展 trait。
#[allow(unused)]
pub trait HandheldExt<R: Runtime> {
    fn handheld_list_emulators(&self) -> Result<ListEmulatorsResponse, Error>;
    fn handheld_launch_game(&self, request: LaunchGameRequest)
        -> Result<LaunchGameResponse, Error>;
    fn handheld_has_all_files_access(&self) -> Result<AllFilesAccessResponse, Error>;
    fn handheld_request_all_files_access(&self) -> Result<AllFilesAccessResponse, Error>;
    fn handheld_discover_rom_roots(&self) -> Result<DiscoverRomRootsResponse, Error>;
    fn handheld_set_system_bars(
        &self,
        request: SystemBarsRequest,
    ) -> Result<SystemBarsResponse, Error>;
}

#[cfg(target_os = "android")]
impl<R: Runtime> HandheldExt<R> for tauri::AppHandle<R> {
    fn handheld_list_emulators(&self) -> Result<ListEmulatorsResponse, Error> {
        self.state::<State<R>>()
            .0
            .run_mobile_plugin("listEmulators", ())
            .map_err(Into::into)
    }

    fn handheld_launch_game(
        &self,
        request: LaunchGameRequest,
    ) -> Result<LaunchGameResponse, Error> {
        self.state::<State<R>>()
            .0
            .run_mobile_plugin("launchGame", request)
            .map_err(Into::into)
    }

    fn handheld_has_all_files_access(&self) -> Result<AllFilesAccessResponse, Error> {
        self.state::<State<R>>()
            .0
            .run_mobile_plugin("hasAllFilesAccess", ())
            .map_err(Into::into)
    }

    fn handheld_request_all_files_access(&self) -> Result<AllFilesAccessResponse, Error> {
        self.state::<State<R>>()
            .0
            .run_mobile_plugin("requestAllFilesAccess", ())
            .map_err(Into::into)
    }

    fn handheld_discover_rom_roots(&self) -> Result<DiscoverRomRootsResponse, Error> {
        self.state::<State<R>>()
            .0
            .run_mobile_plugin("discoverRomRoots", ())
            .map_err(Into::into)
    }

    fn handheld_set_system_bars(
        &self,
        request: SystemBarsRequest,
    ) -> Result<SystemBarsResponse, Error> {
        self.state::<State<R>>()
            .0
            .run_mobile_plugin("setSystemBars", request)
            .map_err(Into::into)
    }
}

#[cfg(not(target_os = "android"))]
impl<R: Runtime> HandheldExt<R> for tauri::AppHandle<R> {
    fn handheld_list_emulators(&self) -> Result<ListEmulatorsResponse, Error> {
        Err(Error::Unsupported)
    }

    fn handheld_launch_game(
        &self,
        _request: LaunchGameRequest,
    ) -> Result<LaunchGameResponse, Error> {
        Err(Error::Unsupported)
    }

    fn handheld_has_all_files_access(&self) -> Result<AllFilesAccessResponse, Error> {
        Err(Error::Unsupported)
    }

    fn handheld_request_all_files_access(&self) -> Result<AllFilesAccessResponse, Error> {
        Err(Error::Unsupported)
    }

    fn handheld_discover_rom_roots(&self) -> Result<DiscoverRomRootsResponse, Error> {
        Err(Error::Unsupported)
    }

    fn handheld_set_system_bars(
        &self,
        _request: SystemBarsRequest,
    ) -> Result<SystemBarsResponse, Error> {
        Err(Error::Unsupported)
    }
}

// ── 前端可调用的 Tauri 命令（桌面端返回明确 Unsupported，保持契约一致）────

#[tauri::command]
async fn list_emulators<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<ListEmulatorsResponse, Error> {
    app.handheld_list_emulators()
}

#[tauri::command]
async fn launch_game<R: Runtime>(
    app: tauri::AppHandle<R>,
    request: LaunchGameRequest,
) -> Result<LaunchGameResponse, Error> {
    app.handheld_launch_game(request)
}

#[tauri::command]
async fn has_all_files_access<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<AllFilesAccessResponse, Error> {
    app.handheld_has_all_files_access()
}

#[tauri::command]
async fn request_all_files_access<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<AllFilesAccessResponse, Error> {
    app.handheld_request_all_files_access()
}

#[tauri::command]
async fn discover_rom_roots<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<DiscoverRomRootsResponse, Error> {
    app.handheld_discover_rom_roots()
}

#[tauri::command]
async fn set_system_bars<R: Runtime>(
    app: tauri::AppHandle<R>,
    request: SystemBarsRequest,
) -> Result<SystemBarsResponse, Error> {
    app.handheld_set_system_bars(request)
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("handheld")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle =
                    api.register_android_plugin("com.moeplay.handheld", "HandheldPlugin")?;
                app.manage(State(handle));
            }
            #[cfg(not(target_os = "android"))]
            {
                let _ = (app, api);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_emulators,
            launch_game,
            has_all_files_access,
            request_all_files_access,
            discover_rom_roots,
            set_system_bars
        ])
        .build()
}
