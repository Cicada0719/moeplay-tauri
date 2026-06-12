use crate::thumbnail;

#[tauri::command]
pub async fn cache_thumbnail(
    key: String,
    source: String,
) -> Result<thumbnail::ThumbnailInfo, String> {
    thumbnail::cache_thumbnail(&key, &source).await
}

#[tauri::command]
pub fn get_thumbnail(key: String) -> Option<thumbnail::ThumbnailInfo> {
    thumbnail::get_thumbnail(&key)
}

#[tauri::command]
pub fn clear_thumbnail_cache() -> Result<u32, String> {
    thumbnail::clear_thumbnail_cache()
}
