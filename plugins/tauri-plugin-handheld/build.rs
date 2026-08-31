const COMMANDS: &[&str] = &[
    "list_emulators",
    "launch_game",
    "has_all_files_access",
    "request_all_files_access",
    "discover_rom_roots",
    "set_system_bars",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
