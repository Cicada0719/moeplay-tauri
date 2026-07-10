use crate::downloader::{DownloadTask, Downloader};
use crate::task_queue::{TaskQueue, TaskStatus};
use serde_json::json;
use sha2::{Digest, Sha256};
use tauri::State;

// ============================================================================
// 下载管理
// ============================================================================

/// 启动一个下载任务。旧调用方可以省略 durable 参数；后端仍会为相同输入生成稳定键。
#[tauri::command]
pub async fn download_start(
    dm: State<'_, Downloader>,
    queue: State<'_, TaskQueue>,
    url: String,
    filename: String,
    auto_extract: Option<bool>,
    auto_import: Option<bool>,
    idempotency_key: Option<String>,
    quota_bytes: Option<u64>,
) -> Result<DownloadTask, String> {
    let auto_extract = auto_extract.unwrap_or(false);
    let auto_import = auto_import.unwrap_or(false);
    let filename = if filename.trim().is_empty() {
        "download.bin".to_string()
    } else {
        filename
    };
    let key = idempotency_key
        .filter(|key| !key.trim().is_empty())
        .unwrap_or_else(|| {
            default_download_key(&url, &filename, auto_extract, auto_import, quota_bytes)
        });
    let job = queue.enqueue_with_metadata(
        filename.clone(),
        "download".to_string(),
        Some(key),
        json!({
            "url": url,
            "filename": filename,
            "autoExtract": auto_extract,
            "autoImport": auto_import,
            "quotaBytes": quota_bytes,
            "resumable": true,
            "retryable": false
        }),
    )?;
    dm.enqueue_persistent(
        queue.inner().clone(),
        job,
        url,
        filename,
        auto_extract,
        auto_import,
        quota_bytes,
    )
    .await
}

/// 暂停指定下载。先停止内存中的写入，再持久化状态，避免“已暂停”后继续落盘。
#[tauri::command]
pub async fn download_pause(
    dm: State<'_, Downloader>,
    queue: State<'_, TaskQueue>,
    task_id: String,
) -> Result<(), String> {
    if queue.get(&task_id).is_err() {
        return dm.pause(&task_id).await;
    }
    dm.hydrate_persistent_jobs(queue.inner()).await?;
    dm.pause(&task_id).await?;
    if let Err(error) = queue.pause(
        &task_id,
        Some("已暂停；HTTP 连接已关闭，恢复时将使用 Range 续传".to_string()),
    ) {
        let _ = dm.resume_persistent(queue.inner().clone(), &task_id).await;
        return Err(error);
    }
    Ok(())
}

/// 恢复暂停或重启后恢复的下载。已有部分文件会通过 HTTP Range 校验后续传。
#[tauri::command]
pub async fn download_resume(
    dm: State<'_, Downloader>,
    queue: State<'_, TaskQueue>,
    task_id: String,
) -> Result<(), String> {
    if queue.get(&task_id).is_err() {
        return dm.resume(&task_id).await;
    }
    queue.resume(&task_id, Some("正在继续下载".to_string()))?;
    if let Err(error) = dm.resume_persistent(queue.inner().clone(), &task_id).await {
        let _ = queue.pause(&task_id, Some(format!("恢复失败: {error}")));
        return Err(error);
    }
    Ok(())
}

/// 取消指定下载。HTTP 连接会被主动丢弃且不再写入；归档解压只能按记录的限制延迟停止。
#[tauri::command]
pub async fn download_cancel(
    dm: State<'_, Downloader>,
    queue: State<'_, TaskQueue>,
    task_id: String,
) -> Result<(), String> {
    if queue.get(&task_id).is_err() {
        return dm.cancel(&task_id).await;
    }
    dm.hydrate_persistent_jobs(queue.inner()).await?;
    let archive_cancellation_deferred = dm.archive_cancellation_is_deferred(&task_id).await?;
    if archive_cancellation_deferred {
        queue.patch_metadata(
            &task_id,
            json!({
                "archiveCancellationDeferred": true,
                "archiveCancellationLimitation": crate::downloader::ARCHIVE_CANCELLATION_LIMITATION
            }),
        )?;
    }
    queue.cancel(&task_id)?;
    dm.cancel(&task_id).await?;
    Ok(())
}

/// 取消所有活跃下载。
#[tauri::command]
pub async fn download_cancel_all(
    dm: State<'_, Downloader>,
    queue: State<'_, TaskQueue>,
) -> Result<(), String> {
    for job in queue.list_result()?.into_iter().filter(|job| {
        job.kind == "download"
            && matches!(
                job.status,
                TaskStatus::Queued | TaskStatus::Running | TaskStatus::Paused
            )
    }) {
        let _ = queue.cancel(&job.id);
    }
    dm.cancel_all().await
}

/// 重试失败的下载。
#[tauri::command]
pub async fn download_retry(
    dm: State<'_, Downloader>,
    queue: State<'_, TaskQueue>,
    task_id: String,
) -> Result<(), String> {
    if queue.get(&task_id).is_err() {
        return dm.retry(&task_id).await;
    }
    queue.retry(&task_id, Some("已排队重试".to_string()))?;
    dm.retry_persistent(queue.inner().clone(), &task_id).await
}

/// 从列表中移除任务。
#[tauri::command]
pub async fn download_remove(
    dm: State<'_, Downloader>,
    queue: State<'_, TaskQueue>,
    task_id: String,
) -> Result<(), String> {
    if queue.get(&task_id).is_ok() {
        queue.remove(&task_id)?;
    }
    let _ = dm.remove(&task_id).await;
    Ok(())
}

/// 清除已完成/已取消/失败的任务。
#[tauri::command]
pub async fn download_clear_finished(
    dm: State<'_, Downloader>,
    queue: State<'_, TaskQueue>,
) -> Result<(), String> {
    dm.clear_finished().await?;
    queue.clear_finished_for_kind("download")
}

/// 获取所有下载任务状态。持久任务会先恢复为可继续的本地投影，但不会自动启动传输。
#[tauri::command]
pub async fn get_downloads(
    dm: State<'_, Downloader>,
    queue: State<'_, TaskQueue>,
) -> Result<Vec<DownloadTask>, String> {
    dm.hydrate_persistent_jobs(queue.inner()).await?;
    Ok(dm.get_all().await)
}

/// 设置全局下载速度限制（字节/秒，0 = 不限制）。
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

/// 获取当前速度限制。
#[tauri::command]
pub async fn get_download_speed_limit(dm: State<'_, Downloader>) -> Result<u64, String> {
    Ok(dm.get_speed_limit().await)
}

/// 设置最大并发下载数。
#[tauri::command]
pub async fn set_download_max_concurrent(
    dm: State<'_, Downloader>,
    max: u32,
) -> Result<(), String> {
    dm.set_max_concurrent(max).await;
    Ok(())
}

/// 获取最大并发数。
#[tauri::command]
pub async fn get_download_max_concurrent(dm: State<'_, Downloader>) -> Result<u32, String> {
    Ok(dm.get_max_concurrent().await)
}

fn default_download_key(
    url: &str,
    filename: &str,
    auto_extract: bool,
    auto_import: bool,
    quota_bytes: Option<u64>,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"download:v1\0");
    digest.update(url.trim().as_bytes());
    digest.update(b"\0");
    digest.update(filename.trim().as_bytes());
    digest.update([auto_extract as u8, auto_import as u8]);
    digest.update(quota_bytes.unwrap_or(0).to_le_bytes());
    format!("download:v1:{}", hex::encode(digest.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_key_is_stable_and_input_sensitive() {
        let first = default_download_key("https://example.test/a", "a.zip", true, false, Some(10));
        let same = default_download_key("https://example.test/a", "a.zip", true, false, Some(10));
        let different =
            default_download_key("https://example.test/a", "a.zip", false, false, Some(10));
        assert_eq!(first, same);
        assert_ne!(first, different);
    }
}
