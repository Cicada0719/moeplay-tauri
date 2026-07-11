use crate::db::Database;
use crate::db_sqlite::repositories::ProviderHealthRepository;
use crate::db_sqlite::{SourcePreferenceRepository, SourcePreferenceUpsert};
use crate::domain::{ProviderError, ProviderErrorKind, ProviderHealth};
use crate::providers::anime::AnimeProviderRegistry;
use crate::providers::comic::ComicProviderRegistry;
use crate::source_center::{
    health_summary_for, list_configured_sources, SourceDescriptor, SourceHealthSummary,
    SourceMediaType, SourceReference,
};
use crate::task_queue::{JobOperation, TaskCenterJob, TaskEventLevel, TaskQueue};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use tauri::State;

const SOURCE_CENTER_ERROR: &str = "source center operation failed";
const SOURCE_DISABLED_ERROR: &str = "source is disabled and cannot be verified";
const SOURCE_NOT_FOUND_ERROR: &str = "configured source was not found";
const BATCH_LIMIT: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSourcePreferenceRequest {
    pub provider_id: String,
    pub media_type: SourceMediaType,
    pub enabled: bool,
    pub priority: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceVerificationResult {
    pub source: SourceReference,
    pub task: TaskCenterJob,
    pub health: SourceHealthSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBatchVerificationItem {
    pub source: SourceReference,
    pub health: Option<SourceHealthSummary>,
    pub error: Option<SourceCenterCommandError>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBatchVerificationResult {
    pub task: TaskCenterJob,
    pub results: Vec<SourceBatchVerificationItem>,
    pub cancelled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceCenterCommandError {
    pub error_kind: ProviderErrorKind,
    pub message: String,
    pub retryable: bool,
    pub provider_id: Option<String>,
    pub operation: Option<String>,
}

#[tauri::command]
pub fn list_source_descriptors(
    anime: State<'_, AnimeProviderRegistry>,
    comic: State<'_, ComicProviderRegistry>,
    database: State<'_, Database>,
) -> Result<Vec<SourceDescriptor>, SourceCenterCommandError> {
    collect_descriptors(anime.inner(), comic.inner(), database.inner())
}

#[tauri::command]
pub fn update_source_preference(
    anime: State<'_, AnimeProviderRegistry>,
    comic: State<'_, ComicProviderRegistry>,
    database: State<'_, Database>,
    request: UpdateSourcePreferenceRequest,
) -> Result<SourceDescriptor, SourceCenterCommandError> {
    let source = SourceReference {
        provider_id: request.provider_id,
        media_type: request.media_type,
    }
    .normalized()
    .map_err(|_| {
        command_error(
            ProviderErrorKind::PolicyBlocked,
            SOURCE_CENTER_ERROR,
            false,
            None,
            "preference",
        )
    })?;
    let known = collect_descriptors(anime.inner(), comic.inner(), database.inner())?;
    if !known
        .iter()
        .any(|descriptor| descriptor.source_reference() == source)
    {
        return Err(command_error(
            ProviderErrorKind::Unsupported,
            SOURCE_NOT_FOUND_ERROR,
            false,
            Some(source.provider_id),
            "preference",
        ));
    }
    SourcePreferenceRepository::new(database.sqlite())
        .upsert(SourcePreferenceUpsert {
            provider_id: source.provider_id.clone(),
            media_type: source.media_type.as_str().to_string(),
            enabled: request.enabled,
            priority: request.priority,
        })
        .map_err(|_| {
            command_error(
                ProviderErrorKind::Unknown,
                SOURCE_CENTER_ERROR,
                true,
                None,
                "preference",
            )
        })?;
    collect_descriptors(anime.inner(), comic.inner(), database.inner())?
        .into_iter()
        .find(|descriptor| descriptor.source_reference() == source)
        .ok_or_else(|| {
            command_error(
                ProviderErrorKind::Unknown,
                SOURCE_CENTER_ERROR,
                true,
                None,
                "preference",
            )
        })
}

#[tauri::command]
pub async fn verify_source(
    anime: State<'_, AnimeProviderRegistry>,
    comic: State<'_, ComicProviderRegistry>,
    database: State<'_, Database>,
    queue: State<'_, TaskQueue>,
    source: SourceReference,
) -> Result<SourceVerificationResult, SourceCenterCommandError> {
    let source = source.normalized().map_err(|_| {
        command_error(
            ProviderErrorKind::PolicyBlocked,
            SOURCE_CENTER_ERROR,
            false,
            None,
            "verify",
        )
    })?;
    ensure_source_is_enabled(anime.inner(), comic.inner(), database.inner(), &source)?;
    let task = queue
        .enqueue_operation(
            "验证来源".to_string(),
            JobOperation::ProviderVerify {
                media_type: source.media_type.as_str().to_string(),
                provider_id: source.provider_id.clone(),
            },
            None,
        )
        .map_err(|_| {
            command_error(
                ProviderErrorKind::Unknown,
                SOURCE_CENTER_ERROR,
                true,
                Some(source.provider_id.clone()),
                "verify",
            )
        })?;
    queue
        .mark_running(&task.id, Some("正在验证来源".to_string()), Some(0.1))
        .map_err(|_| {
            command_error(
                ProviderErrorKind::Unknown,
                SOURCE_CENTER_ERROR,
                true,
                Some(source.provider_id.clone()),
                "verify",
            )
        })?;
    let cancellation = queue.register_operation(&task.id).map_err(|_| {
        command_error(
            ProviderErrorKind::Unknown,
            SOURCE_CENTER_ERROR,
            true,
            Some(source.provider_id.clone()),
            "verify",
        )
    })?;
    if cancellation.is_cancelled() {
        return Err(command_error(
            ProviderErrorKind::Cancelled,
            "source verification was cancelled",
            false,
            Some(source.provider_id),
            "verify",
        ));
    }

    match verify_one(anime.inner(), comic.inner(), database.inner(), &source).await {
        Ok(health) => {
            if cancellation.is_cancelled() {
                return Err(command_error(
                    ProviderErrorKind::Cancelled,
                    "source verification was cancelled",
                    false,
                    Some(source.provider_id),
                    "verify",
                ));
            }
            let task = queue
                .mark_succeeded(&task.id, Some("来源验证完成".to_string()))
                .map_err(|_| {
                    command_error(
                        ProviderErrorKind::Unknown,
                        SOURCE_CENTER_ERROR,
                        true,
                        Some(source.provider_id.clone()),
                        "verify",
                    )
                })?;
            Ok(SourceVerificationResult {
                source,
                task,
                health,
            })
        }
        Err(error) => {
            let safe = redact_provider_error(error, Some(source.provider_id.clone()), "verify");
            let _ = queue.mark_failed(
                &task.id,
                ProviderError {
                    kind: safe.error_kind,
                    message: safe.message.clone(),
                    retryable: safe.retryable,
                    retry_after_ms: None,
                    provider_id: safe.provider_id.clone(),
                    operation: safe.operation.clone(),
                },
            );
            Err(safe)
        }
    }
}

#[tauri::command]
pub async fn verify_sources_batch(
    anime: State<'_, AnimeProviderRegistry>,
    comic: State<'_, ComicProviderRegistry>,
    database: State<'_, Database>,
    queue: State<'_, TaskQueue>,
    sources: Vec<SourceReference>,
) -> Result<SourceBatchVerificationResult, SourceCenterCommandError> {
    let sources = normalize_batch_sources(sources)?;
    let task = queue
        .enqueue_operation(
            "批量验证来源".to_string(),
            JobOperation::ProviderVerify {
                media_type: "source_center_batch".to_string(),
                provider_id: "batch".to_string(),
            },
            None,
        )
        .map_err(|_| {
            command_error(
                ProviderErrorKind::Unknown,
                SOURCE_CENTER_ERROR,
                true,
                None,
                "verify_batch",
            )
        })?;
    queue
        .mark_running(&task.id, Some("正在批量验证来源".to_string()), Some(0.0))
        .map_err(|_| {
            command_error(
                ProviderErrorKind::Unknown,
                SOURCE_CENTER_ERROR,
                true,
                None,
                "verify_batch",
            )
        })?;
    let cancellation = queue.register_operation(&task.id).map_err(|_| {
        command_error(
            ProviderErrorKind::Unknown,
            SOURCE_CENTER_ERROR,
            true,
            None,
            "verify_batch",
        )
    })?;
    let total = sources.len() as f64;
    let mut results = Vec::with_capacity(sources.len());
    let mut cancelled = false;

    for (index, source) in sources.into_iter().enumerate() {
        if cancellation.is_cancelled() {
            cancelled = true;
            let _ = queue.append_event(
                &task.id,
                TaskEventLevel::Warn,
                "source_verify.cancelled".to_string(),
                "批量来源验证已取消".to_string(),
                Some(index as f64 / total),
            );
            break;
        }
        let progress = index as f64 / total;
        let _ = queue.append_event(
            &task.id,
            TaskEventLevel::Info,
            "source_verify.started".to_string(),
            "正在验证来源".to_string(),
            Some(progress),
        );
        match ensure_source_is_enabled(anime.inner(), comic.inner(), database.inner(), &source)
            .map(|_| ())
        {
            Ok(()) => match verify_one(anime.inner(), comic.inner(), database.inner(), &source)
                .await
            {
                Ok(health) => {
                    let _ = queue.append_event(
                        &task.id,
                        TaskEventLevel::Info,
                        "source_verify.succeeded".to_string(),
                        "来源验证通过".to_string(),
                        Some((index + 1) as f64 / total),
                    );
                    results.push(SourceBatchVerificationItem {
                        source,
                        health: Some(health),
                        error: None,
                    });
                }
                Err(error) => {
                    let safe =
                        redact_provider_error(error, Some(source.provider_id.clone()), "verify");
                    let _ = queue.append_event(
                        &task.id,
                        TaskEventLevel::Error,
                        "source_verify.failed".to_string(),
                        safe.message.clone(),
                        Some((index + 1) as f64 / total),
                    );
                    results.push(SourceBatchVerificationItem {
                        source,
                        health: None,
                        error: Some(safe),
                    });
                }
            },
            Err(error) => {
                let _ = queue.append_event(
                    &task.id,
                    TaskEventLevel::Error,
                    "source_verify.failed".to_string(),
                    error.message.clone(),
                    Some((index + 1) as f64 / total),
                );
                results.push(SourceBatchVerificationItem {
                    source,
                    health: None,
                    error: Some(error),
                });
            }
        }
    }
    let task = if cancelled {
        queue.get_task_center(&task.id).map_err(|_| {
            command_error(
                ProviderErrorKind::Unknown,
                SOURCE_CENTER_ERROR,
                true,
                None,
                "verify_batch",
            )
        })?
    } else {
        queue
            .mark_succeeded(
                &task.id,
                Some(format!("已完成 {} 个来源验证", results.len())),
            )
            .map_err(|_| {
                command_error(
                    ProviderErrorKind::Unknown,
                    SOURCE_CENTER_ERROR,
                    true,
                    None,
                    "verify_batch",
                )
            })?
    };
    Ok(SourceBatchVerificationResult {
        task,
        results,
        cancelled,
    })
}

#[tauri::command]
pub fn reset_source_health(
    anime: State<'_, AnimeProviderRegistry>,
    comic: State<'_, ComicProviderRegistry>,
    database: State<'_, Database>,
    source: SourceReference,
) -> Result<bool, SourceCenterCommandError> {
    let source = source.normalized().map_err(|_| {
        command_error(
            ProviderErrorKind::PolicyBlocked,
            SOURCE_CENTER_ERROR,
            false,
            None,
            "reset_health",
        )
    })?;
    let known = collect_descriptors(anime.inner(), comic.inner(), database.inner())?;
    if !known
        .iter()
        .any(|descriptor| descriptor.source_reference() == source)
    {
        return Err(command_error(
            ProviderErrorKind::Unsupported,
            SOURCE_NOT_FOUND_ERROR,
            false,
            Some(source.provider_id),
            "reset_health",
        ));
    }
    let mut removed = match source.media_type {
        SourceMediaType::Anime => anime.reset_health(&source.provider_id).map_err(|_| {
            command_error(
                ProviderErrorKind::Unknown,
                SOURCE_CENTER_ERROR,
                true,
                Some(source.provider_id.clone()),
                "reset_health",
            )
        })?,
        SourceMediaType::Comic | SourceMediaType::ExternalRuntime => false,
    };
    let repository = ProviderHealthRepository::new(database.sqlite());
    let records = repository.list_by_state(None).map_err(|_| {
        command_error(
            ProviderErrorKind::Unknown,
            SOURCE_CENTER_ERROR,
            true,
            None,
            "reset_health",
        )
    })?;
    for record in records
        .into_iter()
        .filter(|record| record.provider_id == source.provider_id)
    {
        removed |= repository
            .delete(&record.provider_id, &record.operation)
            .map_err(|_| {
                command_error(
                    ProviderErrorKind::Unknown,
                    SOURCE_CENTER_ERROR,
                    true,
                    None,
                    "reset_health",
                )
            })?;
    }
    Ok(removed)
}

fn collect_descriptors(
    anime: &AnimeProviderRegistry,
    comic: &ComicProviderRegistry,
    database: &Database,
) -> Result<Vec<SourceDescriptor>, SourceCenterCommandError> {
    let preferences = SourcePreferenceRepository::new(database.sqlite())
        .list()
        .map_err(|_| {
            command_error(
                ProviderErrorKind::Unknown,
                SOURCE_CENTER_ERROR,
                true,
                None,
                "list",
            )
        })?;
    let health = ProviderHealthRepository::new(database.sqlite())
        .list_by_state(None)
        .map_err(|_| {
            command_error(
                ProviderErrorKind::Unknown,
                SOURCE_CENTER_ERROR,
                true,
                None,
                "list",
            )
        })?;
    list_configured_sources(anime, comic, &preferences, &health).map_err(|_| {
        command_error(
            ProviderErrorKind::Unknown,
            SOURCE_CENTER_ERROR,
            true,
            None,
            "list",
        )
    })
}

fn ensure_source_is_enabled(
    anime: &AnimeProviderRegistry,
    comic: &ComicProviderRegistry,
    database: &Database,
    source: &SourceReference,
) -> Result<(), SourceCenterCommandError> {
    let descriptor = collect_descriptors(anime, comic, database)?
        .into_iter()
        .find(|descriptor| descriptor.source_reference() == *source)
        .ok_or_else(|| {
            command_error(
                ProviderErrorKind::Unsupported,
                SOURCE_NOT_FOUND_ERROR,
                false,
                Some(source.provider_id.clone()),
                "verify",
            )
        })?;
    if !descriptor.enabled {
        return Err(command_error(
            ProviderErrorKind::PolicyBlocked,
            SOURCE_DISABLED_ERROR,
            false,
            Some(source.provider_id.clone()),
            "verify",
        ));
    }
    Ok(())
}

async fn verify_one(
    anime: &AnimeProviderRegistry,
    comic: &ComicProviderRegistry,
    database: &Database,
    source: &SourceReference,
) -> Result<SourceHealthSummary, ProviderError> {
    match source.media_type {
        SourceMediaType::Anime => {
            // Current Anime adapters expose health snapshots but no safe generic
            // probe API. This records the current runtime health without sending
            // an invented search request or user content to the provider.
            let snapshots = anime.health()?;
            persist_health(database, &snapshots)?;
            Ok(health_summary_for(&source.provider_id, &snapshots))
        }
        SourceMediaType::Comic => {
            comic
                .probe(&source.provider_id)
                .await
                .map_err(|error| error.provider_error())?;
            let snapshot = comic
                .health(&source.provider_id, "probe")
                .map_err(|error| error.provider_error())?;
            persist_health(database, std::slice::from_ref(&snapshot))?;
            Ok(health_summary_for(&source.provider_id, &[snapshot]))
        }
        SourceMediaType::ExternalRuntime => Err(ProviderError {
            kind: ProviderErrorKind::Unsupported,
            message: SOURCE_NOT_FOUND_ERROR.to_string(),
            retryable: false,
            retry_after_ms: None,
            provider_id: Some(source.provider_id.clone()),
            operation: Some("verify".to_string()),
        }),
    }
}

fn persist_health(database: &Database, snapshots: &[ProviderHealth]) -> Result<(), ProviderError> {
    let repository = ProviderHealthRepository::new(database.sqlite());
    for snapshot in snapshots {
        repository.upsert(snapshot).map_err(|_| ProviderError {
            kind: ProviderErrorKind::Unknown,
            message: SOURCE_CENTER_ERROR.to_string(),
            retryable: true,
            retry_after_ms: None,
            provider_id: Some(snapshot.provider_id.clone()),
            operation: Some(snapshot.operation.clone()),
        })?;
    }
    Ok(())
}

fn normalize_batch_sources(
    sources: Vec<SourceReference>,
) -> Result<Vec<SourceReference>, SourceCenterCommandError> {
    if sources.is_empty() || sources.len() > BATCH_LIMIT {
        return Err(command_error(
            ProviderErrorKind::PolicyBlocked,
            SOURCE_CENTER_ERROR,
            false,
            None,
            "verify_batch",
        ));
    }
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::with_capacity(sources.len());
    for source in sources {
        let source = source.normalized().map_err(|_| {
            command_error(
                ProviderErrorKind::PolicyBlocked,
                SOURCE_CENTER_ERROR,
                false,
                None,
                "verify_batch",
            )
        })?;
        if seen.insert((
            source.media_type.as_str().to_string(),
            source.provider_id.clone(),
        )) {
            normalized.push(source);
        }
    }
    Ok(normalized)
}

fn redact_provider_error(
    error: ProviderError,
    provider_id: Option<String>,
    operation: &str,
) -> SourceCenterCommandError {
    let message = match error.kind {
        ProviderErrorKind::AuthRequired => "source authentication is required",
        ProviderErrorKind::Network
        | ProviderErrorKind::Timeout
        | ProviderErrorKind::RateLimited => "source network verification failed",
        ProviderErrorKind::PolicyBlocked => "source verification was blocked by policy",
        ProviderErrorKind::Cancelled => "source verification was cancelled",
        ProviderErrorKind::Unsupported => SOURCE_NOT_FOUND_ERROR,
        _ => SOURCE_CENTER_ERROR,
    };
    command_error(
        error.kind,
        message,
        error.retryable,
        provider_id.or(error.provider_id),
        operation,
    )
}

fn command_error(
    kind: ProviderErrorKind,
    message: &str,
    retryable: bool,
    provider_id: Option<String>,
    operation: &str,
) -> SourceCenterCommandError {
    SourceCenterCommandError {
        error_kind: kind,
        message: message.to_string(),
        retryable,
        provider_id,
        operation: Some(operation.to_string()),
    }
}
