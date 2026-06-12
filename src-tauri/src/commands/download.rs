use crate::downloader::{DownloadTask, Downloader};
use tauri::State;

// ============================================================================
// 下载管理
// ============================================================================

/// 启动一个下载任务
#[tauri::command]
pub async fn download_start(
    dm: State<'_, Downloader>,
    url: String,
    filename: String,
    auto_extract: Option<bool>,
    auto_import: Option<bool>,
) -> Result<DownloadTask, String> {
    Ok(dm
        .enqueue(
            url,
            filename,
            auto_extract.unwrap_or(false),
            auto_import.unwrap_or(false),
        )
        .await)
}

/// 暂停指定下载
#[tauri::command]
pub async fn download_pause(dm: State<'_, Downloader>, task_id: String) -> Result<(), String> {
    dm.pause(&task_id).await
}

/// 恢复暂停的下载
#[tauri::command]
pub async fn download_resume(dm: State<'_, Downloader>, task_id: String) -> Result<(), String> {
    dm.resume(&task_id).await
}

/// 取消指定下载
#[tauri::command]
pub async fn download_cancel(dm: State<'_, Downloader>, task_id: String) -> Result<(), String> {
    dm.cancel(&task_id).await
}

/// 取消所有活跃下载
#[tauri::command]
pub async fn download_cancel_all(dm: State<'_, Downloader>) -> Result<(), String> {
    dm.cancel_all().await
}

/// 重试失败的下载
#[tauri::command]
pub async fn download_retry(dm: State<'_, Downloader>, task_id: String) -> Result<(), String> {
    dm.retry(&task_id).await
}

/// 从列表中移除任务
#[tauri::command]
pub async fn download_remove(dm: State<'_, Downloader>, task_id: String) -> Result<(), String> {
    dm.remove(&task_id).await
}

/// 清除已完成/已取消/失败的任务
#[tauri::command]
pub async fn download_clear_finished(dm: State<'_, Downloader>) -> Result<(), String> {
    dm.clear_finished().await
}

/// 获取所有下载任务状态（前端轮询用，建议 200ms 间隔）
#[tauri::command]
pub async fn get_downloads(dm: State<'_, Downloader>) -> Result<Vec<DownloadTask>, String> {
    Ok(dm.get_all().await)
}

/// 设置全局下载速度限制（字节/秒，0 = 不限制）
#[tauri::command]
pub async fn set_download_speed_limit(
    dm: State<'_, Downloader>,
    bytes_per_sec: u64,
) -> Result<(), String> {
    dm.set_speed_limit(if bytes_per_sec > 0 {
        Some(bytes_per_sec)
    } else {
        None
    })
    .await;
    Ok(())
}

/// 获取当前速度限制
#[tauri::command]
pub async fn get_download_speed_limit(dm: State<'_, Downloader>) -> Result<u64, String> {
    Ok(dm.get_speed_limit().await)
}

/// 设置最大并发下载数
#[tauri::command]
pub async fn set_download_max_concurrent(
    dm: State<'_, Downloader>,
    max: u32,
) -> Result<(), String> {
    dm.set_max_concurrent(max).await;
    Ok(())
}

/// 获取最大并发数
#[tauri::command]
pub async fn get_download_max_concurrent(dm: State<'_, Downloader>) -> Result<u32, String> {
    Ok(dm.get_max_concurrent().await)
}
