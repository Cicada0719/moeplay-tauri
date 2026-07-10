use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiProviderKind {
    OpenAiCompatible,
    Ollama,
    Mock,
}

impl AiProviderKind {
    pub fn is_local(self) -> bool {
        matches!(self, Self::Ollama | Self::Mock)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCapabilities {
    pub structured_output: bool,
    pub json_mode: bool,
    pub streaming: bool,
    pub vision: bool,
    pub local: bool,
    pub max_context_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderConfig {
    pub id: String,
    pub kind: AiProviderKind,
    pub display_name: String,
    pub base_url: String,
    pub model: String,
    pub secret_configured: bool,
    pub capabilities: AiCapabilities,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiMessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiMessage {
    pub role: AiMessageRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredRequest {
    pub task_id: String,
    pub prompt_id: String,
    pub prompt_version: String,
    pub schema_id: String,
    pub model: String,
    pub messages: Vec<AiMessage>,
    pub temperature: f32,
    pub max_output_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredResponse {
    pub provider_id: String,
    pub model: String,
    pub content: String,
    pub usage: TokenUsage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryCleanupInput {
    pub games: Vec<LibraryGameContext>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryGameContext {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

impl LibraryCleanupInput {
    pub fn allowed_game_ids(&self) -> BTreeSet<String> {
        self.games.iter().map(|game| game.id.clone()).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryChangeSetOutput {
    pub summary: String,
    pub confidence: f64,
    pub operations: Vec<LibraryOperation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum LibraryOperation {
    SetField {
        game_id: String,
        field: String,
        value: Value,
        reason: String,
    },
    AddTag {
        game_id: String,
        tag: String,
        reason: String,
    },
    PossibleDuplicate {
        game_ids: Vec<String>,
        reason: String,
    },
    NeedsReview {
        game_id: String,
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceFilterKind {
    Game,
    Anime,
    Comic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NaturalLanguageFilterOutput {
    pub kind: ResourceFilterKind,
    pub filters: Vec<FilterClause>,
    #[serde(default)]
    pub sort: Vec<SortClause>,
    pub explanation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FilterClause {
    pub field: String,
    pub op: String,
    #[serde(default)]
    pub value: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SortClause {
    pub field: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecommendationInput {
    pub candidates: Vec<RecommendationCandidateContext>,
    #[serde(default)]
    pub excluded_ids: BTreeSet<String>,
    pub limit: u32,
    #[serde(default)]
    pub request: Option<String>,
}

impl RecommendationInput {
    pub fn allowed_candidate_ids(&self) -> BTreeSet<String> {
        self.candidates
            .iter()
            .filter(|candidate| candidate.available && !self.excluded_ids.contains(&candidate.id))
            .map(|candidate| candidate.id.clone())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecommendationCandidateContext {
    pub id: String,
    pub title: String,
    pub kind: ResourceFilterKind,
    pub available: bool,
    #[serde(default)]
    pub estimated_minutes: Option<u32>,
    #[serde(default)]
    pub signals: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecommendationOutput {
    pub summary: String,
    pub recommendations: Vec<RecommendationItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecommendationItem {
    pub resource_id: String,
    pub rank: u32,
    pub reason: String,
    pub confidence: f64,
    #[serde(default)]
    pub signals: Vec<String>,
}
