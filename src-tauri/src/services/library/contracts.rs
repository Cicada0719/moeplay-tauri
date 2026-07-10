use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformIdentity {
    pub source: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameIdentity {
    pub launch_path: Option<String>,
    pub platform_id: Option<PlatformIdentity>,
    pub title_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSourceRecord {
    pub source_record_id: String,
    pub title: String,
    pub launch_path: Option<String>,
    pub install_dir: Option<String>,
    pub platform_id: Option<PlatformIdentity>,
    pub launch_uri: Option<String>,
    #[serde(default)]
    pub fields: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportAction {
    Create,
    Update,
    Merge,
    Conflict,
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportReasonCode {
    NewIdentity,
    LaunchPathMatch,
    PlatformIdMatch,
    AmbiguousStrongIdentity,
    TitleRecallOnly,
    NoLaunchTarget,
    ExplicitMerge,
    ExplicitIgnore,
    StalePreview,
    InvalidDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReason {
    pub code: ImportReasonCode,
    pub message: String,
    #[serde(default)]
    pub recalled_game_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityMatchKind {
    PlatformId,
    LaunchPath,
    TitleRecall,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityMatch {
    pub game_id: String,
    pub game_title: String,
    pub kind: IdentityMatchKind,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldDiffDisposition {
    Unchanged,
    FillEmpty,
    ReplaceImported,
    PreserveUser,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldProvenance {
    pub game_id: String,
    pub field: String,
    pub source: String,
    pub source_record_id: String,
    pub imported_at: String,
    pub applied_value: Value,
    pub value_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDiff {
    pub field: String,
    pub current: Value,
    pub incoming: Value,
    pub disposition: FieldDiffDisposition,
    pub will_apply: bool,
    pub current_provenance: Option<FieldProvenance>,
    pub incoming_source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCandidate {
    pub id: String,
    pub source: String,
    pub identity: GameIdentity,
    pub action: ImportAction,
    pub reason: ImportReason,
    pub matches: Vec<IdentityMatch>,
    pub target_game_id: Option<String>,
    pub field_diff: Vec<FieldDiff>,
    pub record: ImportSourceRecord,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewImportRequest {
    pub source: String,
    pub records: Vec<ImportSourceRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub preview_id: String,
    pub source: String,
    pub candidates: Vec<ImportCandidate>,
    pub created_at: String,
    pub write_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDecision {
    pub candidate_id: String,
    pub action: ImportAction,
    pub target_game_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyImportRequest {
    pub preview: ImportPreview,
    #[serde(default)]
    pub decisions: Vec<ImportDecision>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyItemStatus {
    Created,
    Updated,
    Merged,
    NoChanges,
    Ignored,
    Conflict,
    Failed,
    AlreadyApplied,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldProvenanceChange {
    pub item_idempotency_key: String,
    pub game_id: String,
    pub field: String,
    pub before: Value,
    pub after: Value,
    pub previous: Option<FieldProvenance>,
    pub current: FieldProvenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyItemResult {
    pub candidate_id: String,
    pub item_idempotency_key: String,
    pub action: ImportAction,
    pub status: ApplyItemStatus,
    pub game_id: Option<String>,
    pub message: String,
    #[serde(default)]
    pub applied_fields: Vec<String>,
    #[serde(default)]
    pub preserved_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyImportResponse {
    pub job_id: String,
    pub idempotency_key: String,
    pub replayed: bool,
    pub results: Vec<ApplyItemResult>,
    #[serde(default)]
    pub provenance_changes: Vec<FieldProvenanceChange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchErrorKind {
    NotFound,
    PermissionDenied,
    InvalidDescriptor,
    UnsupportedScheme,
    SpawnFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LaunchDescriptor {
    Executable {
        path: String,
        args: Vec<String>,
        working_dir: Option<String>,
    },
    Uri {
        uri: String,
    },
    Unavailable {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LaunchOutcome {
    Started {
        descriptor: LaunchDescriptor,
        pid: Option<u32>,
    },
    Delegated {
        descriptor: LaunchDescriptor,
    },
    Failed {
        descriptor: LaunchDescriptor,
        error_kind: LaunchErrorKind,
        message: String,
        retryable: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryHealthState {
    Healthy,
    NeedsAttention,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryHealthIssue {
    pub code: String,
    pub severity: String,
    pub message: String,
    #[serde(default)]
    pub game_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryHealthSnapshot {
    pub state: LibraryHealthState,
    pub total_games: usize,
    pub missing_launch_targets: usize,
    pub duplicate_identity_groups: usize,
    pub title_recall_groups: usize,
    pub unresolved_import_conflicts: usize,
    pub provenance_coverage: f32,
    pub issues: Vec<LibraryHealthIssue>,
}
