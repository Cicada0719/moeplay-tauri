use crate::ai::contracts::{LibraryCleanupInput, RecommendationInput, ResourceFilterKind};
use crate::ai::error::{AiError, AiErrorKind, AiResult};
use crate::ai::schema::{
    library_cleanup_schema, natural_language_filter_schema, parse_json_document,
    recommendation_schema, validate_library_cleanup, validate_natural_language_filter,
    validate_recommendation, OutputSchemaDefinition, ValidatedLibraryChangeSet,
    ValidatedNaturalLanguageFilter, ValidatedRecommendation,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiUseCase {
    LibraryCleanup,
    NaturalLanguageFilter,
    Recommendation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptDefinition {
    pub id: String,
    pub version: String,
    pub use_case: AiUseCase,
    pub system_template: String,
    pub output_schema: OutputSchemaDefinition,
    pub max_output_tokens: u32,
    pub timeout_ms: u64,
    pub max_repair_attempts: u8,
    pub privacy_fields: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PromptRegistry;

impl PromptRegistry {
    pub fn list(&self) -> Vec<PromptDefinition> {
        vec![
            self.library_cleanup(),
            self.natural_language_filter(),
            self.recommendation(),
        ]
    }

    pub fn get(&self, id: &str, version: &str) -> AiResult<PromptDefinition> {
        self.list()
            .into_iter()
            .find(|prompt| prompt.id == id && prompt.version == version)
            .ok_or_else(|| {
                AiError::new(
                    AiErrorKind::NotConfigured,
                    "requested AI prompt/schema version is not registered",
                    false,
                )
            })
    }

    pub fn validate_library_cleanup_response(
        &self,
        version: &str,
        content: &str,
        input: &LibraryCleanupInput,
    ) -> AiResult<ValidatedLibraryChangeSet> {
        let prompt = self.get("library_cleanup", version)?;
        if prompt.use_case != AiUseCase::LibraryCleanup {
            return Err(AiError::new(
                AiErrorKind::PolicyRejected,
                "prompt use case mismatch",
                false,
            ));
        }
        validate_library_cleanup(parse_json_document(content)?, input)
    }

    pub fn validate_natural_language_filter_response(
        &self,
        version: &str,
        content: &str,
        expected_kind: ResourceFilterKind,
    ) -> AiResult<ValidatedNaturalLanguageFilter> {
        let prompt = self.get("natural_language_filter", version)?;
        if prompt.use_case != AiUseCase::NaturalLanguageFilter {
            return Err(AiError::new(
                AiErrorKind::PolicyRejected,
                "prompt use case mismatch",
                false,
            ));
        }
        validate_natural_language_filter(parse_json_document(content)?, expected_kind)
    }

    pub fn validate_recommendation_response(
        &self,
        version: &str,
        content: &str,
        input: &RecommendationInput,
    ) -> AiResult<ValidatedRecommendation> {
        let prompt = self.get("recommendation", version)?;
        if prompt.use_case != AiUseCase::Recommendation {
            return Err(AiError::new(
                AiErrorKind::PolicyRejected,
                "prompt use case mismatch",
                false,
            ));
        }
        validate_recommendation(parse_json_document(content)?, input)
    }

    fn library_cleanup(&self) -> PromptDefinition {
        PromptDefinition {
            id: "library_cleanup".to_string(),
            version: "1.0.0".to_string(),
            use_case: AiUseCase::LibraryCleanup,
            system_template: "Return only the registered library cleanup JSON change-set. Use only supplied game IDs and whitelisted fields. Never claim a write was applied.".to_string(),
            output_schema: library_cleanup_schema(),
            max_output_tokens: 2400,
            timeout_ms: 45_000,
            max_repair_attempts: 1,
            privacy_fields: vec![
                "title".to_string(),
                "description".to_string(),
                "tags".to_string(),
                "selectedPublicMetadata".to_string(),
            ],
        }
    }

    fn natural_language_filter(&self) -> PromptDefinition {
        PromptDefinition {
            id: "natural_language_filter".to_string(),
            version: "1.0.0".to_string(),
            use_case: AiUseCase::NaturalLanguageFilter,
            system_template: "Compile the request into the registered whitelist-only filter DSL. Return JSON only. Never emit SQL, scripts, paths, or commands.".to_string(),
            output_schema: natural_language_filter_schema(),
            max_output_tokens: 1000,
            timeout_ms: 20_000,
            max_repair_attempts: 1,
            privacy_fields: vec!["naturalLanguageQuery".to_string()],
        }
    }

    fn recommendation(&self) -> PromptDefinition {
        PromptDefinition {
            id: "recommendation".to_string(),
            version: "1.0.0".to_string(),
            use_case: AiUseCase::Recommendation,
            system_template: "Rank and explain only the supplied available candidate IDs. Respect excluded IDs. Cite only supplied local signals and never introduce a resource outside the candidate set.".to_string(),
            output_schema: recommendation_schema(),
            max_output_tokens: 1600,
            timeout_ms: 30_000,
            max_repair_attempts: 1,
            privacy_fields: vec![
                "candidateTitles".to_string(),
                "localSignals".to_string(),
                "naturalLanguageRequest".to_string(),
            ],
        }
    }
}
