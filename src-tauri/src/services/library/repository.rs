use super::contracts::{ApplyImportResponse, FieldProvenanceChange};
use super::importer::{ImportJobState, LibraryImportBackend};
use super::provenance::ProvenanceLedger;
use crate::db::Database;
use crate::db_sqlite::repositories::BackgroundJobRepository;
use crate::domain::{BackgroundJob, BackgroundJobStatus, ProviderError, ProviderErrorKind};
use crate::models::Game;
use chrono::Utc;
use serde_json::json;

pub struct DatabaseLibraryBackend<'a> {
    db: &'a Database,
}

impl<'a> DatabaseLibraryBackend<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    fn jobs(&self) -> BackgroundJobRepository<'_> {
        BackgroundJobRepository::new(self.db.sqlite())
    }
}

impl LibraryImportBackend for DatabaseLibraryBackend<'_> {
    fn list_games(&self) -> Result<Vec<Game>, String> {
        Ok(self.db.get_games())
    }

    fn add_game(&self, game: Game) -> Result<Game, String> {
        self.db.add_game(game)
    }

    fn update_game(&self, game: Game) -> Result<Game, String> {
        self.db.update_game(game)
    }

    fn load_provenance(&self) -> Result<ProvenanceLedger, String> {
        let jobs = self.jobs().list(&[], 500)?;
        let mut ledger = ProvenanceLedger::new();
        for job in jobs {
            if job.kind != "library_import_v2" {
                continue;
            }
            let Some(changes) = job.metadata.get("response").and_then(|response| {
                response.get("provenanceChanges").and_then(|value| {
                    serde_json::from_value::<Vec<FieldProvenanceChange>>(value.clone()).ok()
                })
            }) else {
                continue;
            };
            for change in changes {
                ledger
                    .entry((change.current.game_id.clone(), change.current.field.clone()))
                    .or_insert(change.current);
            }
        }
        Ok(ledger)
    }

    fn load_completed_apply(&self, job_id: &str) -> Result<Option<ApplyImportResponse>, String> {
        let Some(job) = self.jobs().get(job_id)? else {
            return Ok(None);
        };
        completed_response(&job)
    }

    fn claim_import_job(
        &self,
        job_id: &str,
        idempotency_key: &str,
    ) -> Result<Option<ApplyImportResponse>, String> {
        let repository = self.jobs();
        let now = Utc::now().to_rfc3339();
        let initial = make_job(
            job_id,
            idempotency_key,
            ImportJobState::Running,
            0.0,
            now.clone(),
            now,
            None,
            None,
        )?;
        match repository.insert(&initial) {
            Ok(()) => Ok(None),
            Err(insert_error) => {
                let Some(existing) = repository.get(job_id)? else {
                    return Err(insert_error);
                };
                if let Some(response) = completed_response(&existing)? {
                    return Ok(Some(response));
                }
                if matches!(
                    existing.status,
                    BackgroundJobStatus::Queued | BackgroundJobStatus::Running
                ) {
                    return Err(format!(
                        "import with this idempotency key is already running: {job_id}"
                    ));
                }
                let restarted_at = Utc::now().to_rfc3339();
                repository.upsert(&make_job(
                    job_id,
                    idempotency_key,
                    ImportJobState::Running,
                    0.0,
                    existing.created_at,
                    restarted_at,
                    None,
                    None,
                )?)?;
                Ok(None)
            }
        }
    }

    fn record_import_job(
        &self,
        job_id: &str,
        idempotency_key: &str,
        state: ImportJobState,
        progress: f32,
        response: Option<&ApplyImportResponse>,
        error: Option<&str>,
    ) -> Result<(), String> {
        let repository = self.jobs();
        let now = Utc::now().to_rfc3339();
        let created_at = repository
            .get(job_id)?
            .map(|job| job.created_at)
            .unwrap_or_else(|| now.clone());
        repository.upsert(&make_job(
            job_id,
            idempotency_key,
            state,
            progress,
            created_at,
            now,
            response,
            error,
        )?)
    }
}

#[allow(clippy::too_many_arguments)]
fn make_job(
    job_id: &str,
    idempotency_key: &str,
    state: ImportJobState,
    progress: f32,
    created_at: String,
    updated_at: String,
    response: Option<&ApplyImportResponse>,
    error: Option<&str>,
) -> Result<BackgroundJob, String> {
    let metadata = json!({
        "schemaVersion": 2,
        "idempotencyKey": idempotency_key,
        "response": response.map(serde_json::to_value).transpose().map_err(|error| error.to_string())?,
    });
    Ok(BackgroundJob {
        id: job_id.to_string(),
        kind: "library_import_v2".to_string(),
        title: "Game library import".to_string(),
        status: match state {
            ImportJobState::Running => BackgroundJobStatus::Running,
            ImportJobState::Succeeded => BackgroundJobStatus::Succeeded,
            ImportJobState::Failed => BackgroundJobStatus::Failed,
        },
        progress: progress.clamp(0.0, 1.0),
        created_at,
        updated_at,
        error: error.map(|message| ProviderError {
            kind: ProviderErrorKind::Unknown,
            message: message.to_string(),
            retryable: true,
            retry_after_ms: None,
            provider_id: Some("library_import_v2".to_string()),
            operation: Some("apply".to_string()),
        }),
        metadata,
    })
}

fn completed_response(job: &BackgroundJob) -> Result<Option<ApplyImportResponse>, String> {
    if job.status != BackgroundJobStatus::Succeeded {
        return Ok(None);
    }
    job.metadata
        .get("response")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| error.to_string())
}
