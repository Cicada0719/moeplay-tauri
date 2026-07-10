fn main() {
    tauri::Builder::default().invoke_handler(tauri::generate_handler![
        commands::foo_bar,
        commands::pick_image_file,
    ]);
}
