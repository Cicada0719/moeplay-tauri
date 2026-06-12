use crate::db::Database;
use crate::models::{Game, GameAlias, Tag};
use std::{fs, path::PathBuf};
use tauri::State;

// ===== Game query =====

#[tauri::command]
pub fn get_games(db: State<'_, Database>) -> Vec<Game> {
    db.get_games()
}

#[tauri::command]
pub fn get_game(db: State<'_, Database>, id: String) -> Result<Game, String> {
    db.get_game(&id)
}

#[tauri::command]
pub fn search_games(db: State<'_, Database>, query: String) -> Vec<Game> {
    db.search_games(&query)
}

// ===== Game CRUD =====

#[tauri::command]
pub fn add_game_by_path(db: State<'_, Database>, path: String) -> Result<Game, String> {
    let path = PathBuf::from(&path);
    if !path.exists() {
        return Err("文件不存在".to_string());
    }

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("未知游戏")
        .to_string();

    let game = Game::new(name, path.to_string_lossy().to_string());
    db.add_game(game)
}

#[tauri::command]
pub fn add_game_by_dialog(db: State<'_, Database>) -> Result<Game, String> {
    let file = rfd::FileDialog::new()
        .set_title("选择游戏可执行文件")
        .add_filter("可执行文件", &["exe", "bat", "cmd", "lnk", "msi"])
        .add_filter("所有文件", &["*"])
        .pick_file();

    match file {
        Some(path) => {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("未知游戏")
                .to_string();

            let game = Game::new(name, path.to_string_lossy().to_string());
            db.add_game(game)
        }
        None => Err("已取消".to_string()),
    }
}

#[tauri::command]
pub fn delete_game(db: State<'_, Database>, id: String) -> Result<(), String> {
    db.delete_game(&id)
}

#[tauri::command]
pub fn update_game(db: State<'_, Database>, game: Game) -> Result<Game, String> {
    db.update_game(game)
}

#[tauri::command]
pub fn import_games_from_dir(db: State<'_, Database>, dir: String) -> Result<Vec<Game>, String> {
    let dir_path = PathBuf::from(&dir);
    if !dir_path.is_dir() {
        return Err("目录不存在".to_string());
    }

    let mut imported = vec![];
    let extensions = ["exe", "bat", "cmd", "lnk"];

    if let Ok(entries) = fs::read_dir(&dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.contains(&ext.to_lowercase().as_str()) {
                        let name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("未知")
                            .to_string();

                        let game = Game::new(name, path.to_string_lossy().to_string());
                        if let Ok(g) = db.add_game(game.clone()) {
                            imported.push(g);
                        }
                    }
                }
            }
        }
    }

    Ok(imported)
}

// ===== Basic game fields =====

#[tauri::command]
pub fn update_game_name(db: State<'_, Database>, id: String, name: String) -> Result<Game, String> {
    db.update_game_name(&id, name)
}

#[tauri::command]
pub fn update_game_description(
    db: State<'_, Database>,
    id: String,
    description: Option<String>,
) -> Result<Game, String> {
    db.update_game_description(&id, description)
}

#[tauri::command]
pub fn update_game_cover(
    db: State<'_, Database>,
    id: String,
    cover: Option<String>,
) -> Result<Game, String> {
    db.update_game_cover(&id, cover)
}

#[tauri::command]
pub fn update_game_background(
    db: State<'_, Database>,
    id: String,
    background: Option<String>,
) -> Result<Game, String> {
    db.update_game_background(&id, background)
}

#[tauri::command]
pub fn update_game_icon(
    db: State<'_, Database>,
    id: String,
    icon: Option<String>,
) -> Result<Game, String> {
    db.update_game_icon(&id, icon)
}

#[tauri::command]
pub fn update_game_type(
    db: State<'_, Database>,
    id: String,
    game_type: Option<String>,
) -> Result<Game, String> {
    db.update_game_type(&id, game_type)
}

#[tauri::command]
pub fn update_install_dir(
    db: State<'_, Database>,
    id: String,
    install_dir: Option<String>,
) -> Result<Game, String> {
    db.update_install_dir(&id, install_dir)
}

#[tauri::command]
pub fn update_exe_path(
    db: State<'_, Database>,
    id: String,
    exe_path: String,
) -> Result<Game, String> {
    db.update_exe_path(&id, exe_path)
}

// ===== Quick toggles =====

#[tauri::command]
pub fn toggle_favorite(db: State<'_, Database>, id: String) -> Result<Game, String> {
    db.toggle_favorite(&id)
}

#[tauri::command]
pub fn toggle_hidden(db: State<'_, Database>, id: String) -> Result<Game, String> {
    db.toggle_hidden(&id)
}

// ===== Simple tags =====

#[tauri::command]
pub fn add_simple_tag(db: State<'_, Database>, id: String, tag: String) -> Result<Game, String> {
    db.add_simple_tag(&id, tag)
}

#[tauri::command]
pub fn remove_simple_tag(db: State<'_, Database>, id: String, tag: String) -> Result<Game, String> {
    db.remove_simple_tag(&id, &tag)
}

#[tauri::command]
pub fn set_simple_tags(
    db: State<'_, Database>,
    id: String,
    tags: Vec<String>,
) -> Result<Game, String> {
    db.set_simple_tags(&id, tags)
}

// ===== Rich tags =====

#[tauri::command]
pub fn add_tag_entry(db: State<'_, Database>, id: String, tag: Tag) -> Result<Game, String> {
    db.add_tag_entry(&id, tag)
}

#[tauri::command]
pub fn remove_tag_entry(
    db: State<'_, Database>,
    id: String,
    tag_name: String,
) -> Result<Game, String> {
    db.remove_tag_entry(&id, &tag_name)
}

#[tauri::command]
pub fn update_tag_entry(
    db: State<'_, Database>,
    id: String,
    tag_name: String,
    tag: Tag,
) -> Result<Game, String> {
    db.update_tag_entry(&id, &tag_name, tag)
}

#[tauri::command]
pub fn set_tag_entries(
    db: State<'_, Database>,
    id: String,
    tags: Vec<Tag>,
) -> Result<Game, String> {
    db.set_tag_entries(&id, tags)
}

// ===== Aliases =====

#[tauri::command]
pub fn add_game_alias(
    db: State<'_, Database>,
    id: String,
    alias: GameAlias,
) -> Result<Game, String> {
    db.add_alias(&id, alias)
}

#[tauri::command]
pub fn remove_game_alias(
    db: State<'_, Database>,
    id: String,
    alias_name: String,
) -> Result<Game, String> {
    db.remove_alias(&id, &alias_name)
}

#[tauri::command]
pub fn set_primary_alias(
    db: State<'_, Database>,
    id: String,
    alias_name: String,
) -> Result<Game, String> {
    db.set_primary_alias(&id, &alias_name)
}

#[tauri::command]
pub fn set_game_aliases(
    db: State<'_, Database>,
    id: String,
    aliases: Vec<GameAlias>,
) -> Result<Game, String> {
    db.set_aliases(&id, aliases)
}
