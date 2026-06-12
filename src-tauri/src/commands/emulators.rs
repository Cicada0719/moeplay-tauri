use crate::db::Database;
use crate::models::Game;
use tauri::State;

#[tauri::command]
pub fn search_emulators(search_paths: Vec<String>) -> Vec<crate::emulator::ScannedEmulator> {
    let defs = crate::emulator::builtin_emulator_definitions();
    crate::emulator::search_emulators(&search_paths, &defs)
}

#[tauri::command]
pub fn scan_roms(
    dir: String,
    extensions: Vec<String>,
    recursive: Option<bool>,
) -> Vec<crate::emulator::RomFile> {
    crate::emulator::scan_roms(&dir, &extensions, recursive.unwrap_or(true))
}

#[tauri::command]
pub fn import_rom_game(
    db: State<'_, Database>,
    name: String,
    rom_path: String,
    emulator_exe: String,
    startup_args: String,
    platform: String,
    cover_url: Option<String>,
) -> Result<Game, String> {
    let install_dir = std::path::Path::new(&emulator_exe)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let rom_dir = std::path::Path::new(&rom_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let args = startup_args
        .replace("{ImagePath}", &rom_path)
        .replace("{ImageDir}", &rom_dir);

    let mut game = Game::new(name.clone(), format!("{} {}", emulator_exe, args));
    game.install_dir = Some(install_dir);
    game.game_type = Some(platform.clone());
    game.metadata.engine = Some(format!("Emulator: {}", platform));

    if let Some(cv) = cover_url {
        game.cover = Some(cv.clone());
        game.metadata.cover = Some(cv);
    }

    db.add_game(game)
}
