use serde::Serialize;

/// A stable frontend-facing description of the capabilities available on the
/// current runtime. UI code must use this contract instead of sprinkling
/// platform checks throughout individual components.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    pub platform: &'static str,
    pub orientation_control: bool,
    pub steam_integration: bool,
    pub game_launch: bool,
    pub local_game_scan: bool,
    pub emulator_import: bool,
    pub desktop_window_control: bool,
    pub tray: bool,
    pub autostart: bool,
    pub desktop_updater: bool,
    pub external_player: bool,
    pub file_system_watch: bool,
}

impl PlatformCapabilities {
    pub fn current() -> Self {
        #[cfg(target_os = "android")]
        {
            return Self {
                platform: "android",
                orientation_control: true,
                steam_integration: false,
                game_launch: false,
                local_game_scan: false,
                emulator_import: false,
                desktop_window_control: false,
                tray: false,
                autostart: false,
                desktop_updater: false,
                external_player: false,
                file_system_watch: false,
            };
        }

        #[cfg(not(target_os = "android"))]
        Self {
            platform: "windows",
            orientation_control: false,
            steam_integration: true,
            game_launch: true,
            local_game_scan: true,
            emulator_import: true,
            desktop_window_control: true,
            tray: true,
            autostart: true,
            desktop_updater: true,
            external_player: true,
            file_system_watch: true,
        }
    }
}

#[tauri::command]
pub fn get_platform_capabilities() -> PlatformCapabilities {
    PlatformCapabilities::current()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_capabilities_remain_enabled_on_desktop_builds() {
        let caps = PlatformCapabilities::current();
        #[cfg(not(target_os = "android"))]
        {
            assert_eq!(caps.platform, "windows");
            assert!(caps.game_launch);
            assert!(caps.desktop_window_control);
            assert!(!caps.orientation_control);
        }
    }
}
