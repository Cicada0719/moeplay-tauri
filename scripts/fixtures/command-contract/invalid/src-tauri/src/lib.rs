fn main() {
    tauri::Builder::default().invoke_handler(tauri::generate_handler![
        commands::foo_bar,
        commands::baz_qux,
    ]);
}
