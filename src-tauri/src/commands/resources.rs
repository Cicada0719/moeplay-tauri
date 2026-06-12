use crate::db::Database;
use crate::image_scanner;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub fn scan_images_dir(dir: String) -> Result<Vec<image_scanner::ImageCandidate>, String> {
    image_scanner::scan_images(&PathBuf::from(dir))
}

#[tauri::command]
pub fn scan_game_images(
    db: State<'_, Database>,
    game_id: String,
) -> Result<Vec<image_scanner::ImageCandidate>, String> {
    let game = db.get_game(&game_id)?;
    let dir = super::resolve_game_dir(&game)?;
    image_scanner::scan_images(&dir)
}

#[tauri::command]
pub async fn fetch_game_resources(
    source: String,
    source_id: String,
) -> Result<Vec<crate::resource_fetcher::ResourceLink>, String> {
    if source == "kungal" || source == "touchgal" {
        crate::resource_fetcher::fetch_kungal_resources(&source_id).await
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
pub async fn search_game_downloads(
    name: String,
    kungal_id: Option<String>,
    patch_id: Option<String>,
) -> Result<crate::gal_download::DownloadSearchResult, String> {
    let req = crate::gal_download::DownloadSearchRequest {
        name,
        kungal_id,
        patch_id,
    };
    crate::gal_download::search_downloads(&req).await
}

#[tauri::command]
pub async fn search_downloads_direct(
    candidates: Vec<String>,
) -> Result<crate::gal_download::DownloadSearchResult, String> {
    crate::gal_download::search_downloads_direct(&candidates).await
}
