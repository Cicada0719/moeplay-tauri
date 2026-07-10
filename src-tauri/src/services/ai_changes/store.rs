use super::error::{AiChangesError, AiChangesResult};
use super::AiChangeProvenance;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const AI_UNDO_RECORD_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiUndoMutation {
    pub game_id: String,
    pub field: String,
    pub before: Value,
    pub after: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiUndoRecordBody {
    pub version: u32,
    pub undo_id: String,
    pub change_set_id: String,
    pub task_id: String,
    pub change_set_hash: String,
    pub before_hash: String,
    pub after_hash: String,
    pub selected_operation_indices: Vec<usize>,
    pub mutations: Vec<AiUndoMutation>,
    pub provenance: AiChangeProvenance,
    pub applied_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiUndoRecord {
    #[serde(flatten)]
    pub body: AiUndoRecordBody,
    pub record_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AiUndoMarker {
    version: u32,
    undo_id: String,
    change_set_id: String,
    record_hash: String,
    undone_at: String,
}

pub(crate) struct UndoStore {
    root: PathBuf,
}

impl UndoStore {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub(crate) fn write_record(&self, body: AiUndoRecordBody) -> AiChangesResult<AiUndoRecord> {
        fs::create_dir_all(&self.root).map_err(|_| AiChangesError::storage())?;
        let record_hash = hash_json(&body)?;
        let record = AiUndoRecord { body, record_hash };
        write_new_json(&self.record_path(&record.body.undo_id), &record)?;
        Ok(record)
    }

    pub(crate) fn remove_record(&self, undo_id: &str) {
        let _ = fs::remove_file(self.record_path(undo_id));
    }

    pub(crate) fn read_record(&self, undo_id: &str) -> AiChangesResult<AiUndoRecord> {
        let path = self.record_path(undo_id);
        let mut file = OpenOptions::new().read(true).open(path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                AiChangesError::undo_not_found()
            } else {
                AiChangesError::storage()
            }
        })?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|_| AiChangesError::storage())?;
        let record: AiUndoRecord =
            serde_json::from_slice(&bytes).map_err(|_| AiChangesError::storage())?;
        if record.body.version != AI_UNDO_RECORD_VERSION
            || record.record_hash != hash_json(&record.body)?
        {
            return Err(AiChangesError::storage());
        }
        Ok(record)
    }

    pub(crate) fn marker_exists(&self, undo_id: &str) -> bool {
        self.marker_path(undo_id).is_file()
    }

    pub(crate) fn write_marker(
        &self,
        record: &AiUndoRecord,
        undone_at: String,
    ) -> AiChangesResult<()> {
        let marker = AiUndoMarker {
            version: AI_UNDO_RECORD_VERSION,
            undo_id: record.body.undo_id.clone(),
            change_set_id: record.body.change_set_id.clone(),
            record_hash: record.record_hash.clone(),
            undone_at,
        };
        match write_new_json(&self.marker_path(&record.body.undo_id), &marker) {
            Ok(()) => Ok(()),
            Err(_error) if self.marker_exists(&record.body.undo_id) => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn record_path(&self, undo_id: &str) -> PathBuf {
        self.root.join(format!("{undo_id}.json"))
    }

    fn marker_path(&self, undo_id: &str) -> PathBuf {
        self.root.join(format!("{undo_id}.undone.json"))
    }
}

pub(crate) fn hash_json<T: Serialize>(value: &T) -> AiChangesResult<String> {
    let bytes = serde_json::to_vec(value).map_err(|_| AiChangesError::storage())?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(hex::encode(hasher.finalize()))
}

fn write_new_json<T: Serialize>(path: &Path, value: &T) -> AiChangesResult<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| AiChangesError::storage())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| AiChangesError::storage())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| AiChangesError::storage())
}
