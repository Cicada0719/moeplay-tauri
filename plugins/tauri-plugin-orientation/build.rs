const COMMANDS: &[&str] = &[
    "set_orientation",
    "get_orientation",
    "register_listener",
    "remove_listener",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
