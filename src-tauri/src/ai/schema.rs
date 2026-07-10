use crate::ai::contracts::{
    FilterClause, LibraryChangeSetOutput, LibraryCleanupInput, LibraryOperation,
    NaturalLanguageFilterOutput, RecommendationInput, RecommendationOutput, ResourceFilterKind,
    SortClause,
};
use crate::ai::error::{AiError, AiErrorKind, AiResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub const MAX_STRUCTURED_OUTPUT_BYTES: usize = 262_144;
pub const MAX_LIBRARY_OPERATIONS: usize = 100;
pub const MAX_FILTER_CLAUSES: usize = 24;
pub const MAX_SORT_CLAUSES: usize = 4;
pub const MAX_RECOMMENDATIONS: usize = 50;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputSchemaDefinition {
    pub id: String,
    pub version: String,
    pub schema: Value,
}

pub fn library_cleanup_schema() -> OutputSchemaDefinition {
    OutputSchemaDefinition {
        id: "library_cleanup.change_set".to_string(),
        version: "1.0.0".to_string(),
        schema: json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["summary", "confidence", "operations"],
            "properties": {
                "summary": {"type": "string", "minLength": 1, "maxLength": 1000},
                "confidence": {"type": "number", "minimum": 0, "maximum": 1},
                "operations": {
                    "type": "array",
                    "maxItems": MAX_LIBRARY_OPERATIONS,
                    "items": {
                        "oneOf": [
                            {"type": "object", "required": ["type", "gameId", "field", "value", "reason"]},
                            {"type": "object", "required": ["type", "gameId", "tag", "reason"]},
                            {"type": "object", "required": ["type", "gameIds", "reason"]},
                            {"type": "object", "required": ["type", "gameId", "reason"]}
                        ]
                    }
                }
            }
        }),
    }
}

pub fn natural_language_filter_schema() -> OutputSchemaDefinition {
    OutputSchemaDefinition {
        id: "natural_language_filter.dsl".to_string(),
        version: "1.0.0".to_string(),
        schema: json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["kind", "filters", "sort", "explanation"],
            "properties": {
                "kind": {"enum": ["game", "anime", "comic"]},
                "filters": {"type": "array", "maxItems": MAX_FILTER_CLAUSES},
                "sort": {"type": "array", "maxItems": MAX_SORT_CLAUSES},
                "explanation": {"type": "string", "minLength": 1, "maxLength": 1000}
            }
        }),
    }
}

pub fn recommendation_schema() -> OutputSchemaDefinition {
    OutputSchemaDefinition {
        id: "recommendation.ranking".to_string(),
        version: "1.0.0".to_string(),
        schema: json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["summary", "recommendations"],
            "properties": {
                "summary": {"type": "string", "minLength": 1, "maxLength": 1000},
                "recommendations": {
                    "type": "array",
                    "maxItems": MAX_RECOMMENDATIONS,
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["resourceId", "rank", "reason", "confidence", "signals"],
                        "properties": {
                            "resourceId": {"type": "string", "minLength": 1, "maxLength": 200},
                            "rank": {"type": "integer", "minimum": 1, "maximum": MAX_RECOMMENDATIONS},
                            "reason": {"type": "string", "minLength": 1, "maxLength": 1000},
                            "confidence": {"type": "number", "minimum": 0, "maximum": 1},
                            "signals": {
                                "type": "array",
                                "maxItems": 16,
                                "items": {"type": "string", "minLength": 1, "maxLength": 120}
                            }
                        }
                    }
                }
            }
        }),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedLibraryChangeSet(pub(crate) LibraryChangeSetOutput);

impl ValidatedLibraryChangeSet {
    pub fn value(&self) -> &LibraryChangeSetOutput {
        &self.0
    }

    pub fn into_inner(self) -> LibraryChangeSetOutput {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedNaturalLanguageFilter(pub(crate) NaturalLanguageFilterOutput);

impl ValidatedNaturalLanguageFilter {
    pub fn value(&self) -> &NaturalLanguageFilterOutput {
        &self.0
    }

    pub fn into_inner(self) -> NaturalLanguageFilterOutput {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedRecommendation(pub(crate) RecommendationOutput);

impl ValidatedRecommendation {
    pub fn value(&self) -> &RecommendationOutput {
        &self.0
    }

    pub fn into_inner(self) -> RecommendationOutput {
        self.0
    }
}

pub fn parse_json_document(content: &str) -> AiResult<Value> {
    if content.len() > MAX_STRUCTURED_OUTPUT_BYTES {
        return Err(invalid("structured AI output exceeded the size limit"));
    }
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(invalid("structured AI output was empty"));
    }

    let document = if trimmed.starts_with("```") {
        extract_single_fenced_document(trimmed)?
    } else {
        trimmed
    };
    serde_json::from_str(document)
        .map_err(|_| invalid("structured AI output was not a valid JSON document"))
}

fn extract_single_fenced_document(content: &str) -> AiResult<&str> {
    let first_newline = content
        .find('\n')
        .ok_or_else(|| invalid("structured AI output contained an invalid fenced block"))?;
    let marker = content[..first_newline].trim();
    if marker != "```" && !marker.eq_ignore_ascii_case("```json") {
        return Err(invalid(
            "structured AI output used an unsupported fenced block",
        ));
    }
    if !content.ends_with("```") {
        return Err(invalid(
            "structured AI output contained an unterminated fenced block",
        ));
    }
    let body = content[first_newline + 1..content.len() - 3].trim();
    if body.contains("```") {
        return Err(invalid(
            "structured AI output contained multiple fenced blocks",
        ));
    }
    Ok(body)
}

pub fn validate_library_cleanup(
    value: Value,
    input: &LibraryCleanupInput,
) -> AiResult<ValidatedLibraryChangeSet> {
    let output: LibraryChangeSetOutput = serde_json::from_value(value)
        .map_err(|_| invalid("library cleanup output did not match schema version 1.0.0"))?;

    validate_nonempty_text("summary", &output.summary, 1000)?;
    if !output.confidence.is_finite() || !(0.0..=1.0).contains(&output.confidence) {
        return Err(invalid(
            "library cleanup confidence must be between 0 and 1",
        ));
    }
    if output.operations.len() > MAX_LIBRARY_OPERATIONS {
        return Err(invalid("library cleanup returned too many operations"));
    }

    let allowed_ids = input.allowed_game_ids();
    if allowed_ids.is_empty() && !output.operations.is_empty() {
        return Err(invalid("library cleanup cannot target an empty input set"));
    }
    for operation in &output.operations {
        validate_library_operation(operation, &allowed_ids)?;
    }

    Ok(ValidatedLibraryChangeSet(output))
}

fn validate_library_operation(
    operation: &LibraryOperation,
    allowed_ids: &BTreeSet<String>,
) -> AiResult<()> {
    match operation {
        LibraryOperation::SetField {
            game_id,
            field,
            value,
            reason,
        } => {
            require_allowed_id(game_id, allowed_ids)?;
            validate_nonempty_text("reason", reason, 1000)?;
            validate_set_field(field, value)?;
        }
        LibraryOperation::AddTag {
            game_id,
            tag,
            reason,
        } => {
            require_allowed_id(game_id, allowed_ids)?;
            validate_nonempty_text("tag", tag, 80)?;
            validate_nonempty_text("reason", reason, 1000)?;
        }
        LibraryOperation::PossibleDuplicate { game_ids, reason } => {
            validate_nonempty_text("reason", reason, 1000)?;
            if game_ids.len() < 2 || game_ids.len() > 10 {
                return Err(invalid(
                    "possible_duplicate must contain between 2 and 10 IDs",
                ));
            }
            let mut unique = BTreeSet::new();
            for game_id in game_ids {
                require_allowed_id(game_id, allowed_ids)?;
                if !unique.insert(game_id) {
                    return Err(invalid("possible_duplicate IDs must be unique"));
                }
            }
        }
        LibraryOperation::NeedsReview { game_id, reason } => {
            require_allowed_id(game_id, allowed_ids)?;
            validate_nonempty_text("reason", reason, 1000)?;
        }
    }
    Ok(())
}

fn validate_set_field(field: &str, value: &Value) -> AiResult<()> {
    match field {
        "title" | "developer" | "publisher" => {
            let text = value
                .as_str()
                .ok_or_else(|| invalid("library text field value must be a string"))?;
            validate_nonempty_text(field, text, 200)
        }
        "description" => {
            let text = value
                .as_str()
                .ok_or_else(|| invalid("library description value must be a string"))?;
            validate_nonempty_text(field, text, 5000)
        }
        "contentRating" => {
            let rating = value
                .as_str()
                .ok_or_else(|| invalid("contentRating must be a string"))?;
            if !matches!(rating, "all_ages" | "teen" | "mature" | "adult" | "unknown") {
                return Err(invalid("contentRating is not an allowed value"));
            }
            Ok(())
        }
        "estimatedHours" => {
            let hours = value
                .as_f64()
                .filter(|value| value.is_finite() && (0.0..=10_000.0).contains(value))
                .ok_or_else(|| invalid("estimatedHours must be a number between 0 and 10000"))?;
            let _ = hours;
            Ok(())
        }
        _ => Err(invalid(
            "library cleanup attempted to change a non-whitelisted field",
        )),
    }
}

pub fn validate_recommendation(
    value: Value,
    input: &RecommendationInput,
) -> AiResult<ValidatedRecommendation> {
    let output: RecommendationOutput = serde_json::from_value(value)
        .map_err(|_| invalid("recommendation output did not match the registered schema"))?;
    validate_nonempty_text("recommendation summary", &output.summary, 1000)?;

    let limit = usize::try_from(input.limit)
        .unwrap_or(usize::MAX)
        .min(MAX_RECOMMENDATIONS);
    if output.recommendations.len() > limit {
        return Err(invalid(
            "recommendation output exceeded the requested limit",
        ));
    }

    let allowed_ids = input.allowed_candidate_ids();
    let candidate_signals = input
        .candidates
        .iter()
        .map(|candidate| {
            (
                candidate.id.as_str(),
                candidate
                    .signals
                    .iter()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut seen_ids = BTreeSet::new();
    let mut seen_ranks = BTreeSet::new();

    for recommendation in &output.recommendations {
        if !allowed_ids.contains(&recommendation.resource_id) {
            return Err(invalid(
                "recommendation referenced an unavailable, excluded, or unknown resource ID",
            ));
        }
        if !seen_ids.insert(recommendation.resource_id.as_str()) {
            return Err(invalid(
                "recommendation output contained a duplicate resource ID",
            ));
        }
        if recommendation.rank == 0
            || usize::try_from(recommendation.rank).unwrap_or(usize::MAX) > limit
            || !seen_ranks.insert(recommendation.rank)
        {
            return Err(invalid(
                "recommendation ranks must be unique and within the requested limit",
            ));
        }
        validate_nonempty_text("recommendation reason", &recommendation.reason, 1000)?;
        if !recommendation.confidence.is_finite()
            || !(0.0..=1.0).contains(&recommendation.confidence)
        {
            return Err(invalid(
                "recommendation confidence must be between zero and one",
            ));
        }
        if recommendation.signals.len() > 16 {
            return Err(invalid("recommendation output contained too many signals"));
        }
        let allowed_signals = candidate_signals
            .get(recommendation.resource_id.as_str())
            .ok_or_else(|| invalid("recommendation candidate context was missing"))?;
        let mut seen_signals = BTreeSet::new();
        for signal in &recommendation.signals {
            validate_nonempty_text("recommendation signal", signal, 120)?;
            if !allowed_signals.contains(signal.as_str()) {
                return Err(invalid(
                    "recommendation referenced a signal outside the supplied candidate context",
                ));
            }
            if !seen_signals.insert(signal.as_str()) {
                return Err(invalid(
                    "recommendation output contained a duplicate signal",
                ));
            }
        }
    }

    for expected_rank in 1..=output.recommendations.len() as u32 {
        if !seen_ranks.contains(&expected_rank) {
            return Err(invalid(
                "recommendation ranks must be contiguous starting at one",
            ));
        }
    }

    Ok(ValidatedRecommendation(output))
}

pub fn validate_natural_language_filter(
    value: Value,
    expected_kind: ResourceFilterKind,
) -> AiResult<ValidatedNaturalLanguageFilter> {
    let output: NaturalLanguageFilterOutput = serde_json::from_value(value).map_err(|_| {
        invalid("natural-language filter output did not match schema version 1.0.0")
    })?;
    if output.kind != expected_kind {
        return Err(invalid(
            "filter resource kind did not match the requested kind",
        ));
    }
    if output.filters.len() > MAX_FILTER_CLAUSES {
        return Err(invalid("natural-language filter returned too many clauses"));
    }
    if output.sort.len() > MAX_SORT_CLAUSES {
        return Err(invalid(
            "natural-language filter returned too many sort clauses",
        ));
    }
    validate_nonempty_text("explanation", &output.explanation, 1000)?;
    for filter in &output.filters {
        validate_filter_clause(output.kind, filter)?;
    }
    for sort in &output.sort {
        validate_sort_clause(output.kind, sort)?;
    }
    Ok(ValidatedNaturalLanguageFilter(output))
}

fn validate_filter_clause(kind: ResourceFilterKind, filter: &FilterClause) -> AiResult<()> {
    if filter.field.to_ascii_lowercase().contains("sql")
        || filter.op.to_ascii_lowercase().contains("sql")
    {
        return Err(invalid("filter DSL cannot contain SQL"));
    }

    let rule = filter_rule(kind, &filter.field, &filter.op)
        .ok_or_else(|| invalid("filter field/operator combination is not whitelisted"))?;
    match rule {
        FilterValueRule::None => {
            if filter.value.is_some() {
                return Err(invalid("filter operator does not accept a value"));
            }
        }
        FilterValueRule::String => {
            let text = filter
                .value
                .as_ref()
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("filter value must be a string"))?;
            validate_nonempty_text("filter value", text, 200)?;
        }
        FilterValueRule::StringArray => {
            let values = filter
                .value
                .as_ref()
                .and_then(Value::as_array)
                .ok_or_else(|| invalid("filter value must be a string array"))?;
            if values.is_empty() || values.len() > 20 {
                return Err(invalid(
                    "filter string array must contain between 1 and 20 values",
                ));
            }
            for value in values {
                let text = value
                    .as_str()
                    .ok_or_else(|| invalid("filter array values must be strings"))?;
                validate_nonempty_text("filter value", text, 80)?;
            }
        }
        FilterValueRule::Number => {
            let number = filter
                .value
                .as_ref()
                .and_then(Value::as_f64)
                .filter(|value| value.is_finite() && (-1_000_000.0..=1_000_000.0).contains(value))
                .ok_or_else(|| invalid("filter value must be a bounded number"))?;
            let _ = number;
        }
        FilterValueRule::Boolean => {
            if !filter.value.as_ref().is_some_and(Value::is_boolean) {
                return Err(invalid("filter value must be a boolean"));
            }
        }
        FilterValueRule::ContentRating => {
            let rating = filter
                .value
                .as_ref()
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("content rating filter must be a string"))?;
            if !matches!(rating, "all_ages" | "teen" | "mature" | "adult" | "unknown") {
                return Err(invalid("content rating filter value is not whitelisted"));
            }
        }
    }
    Ok(())
}

fn validate_sort_clause(kind: ResourceFilterKind, sort: &SortClause) -> AiResult<()> {
    let allowed = match kind {
        ResourceFilterKind::Game => matches!(
            sort.field.as_str(),
            "title" | "lastPlayedAt" | "estimatedHours" | "userAffinity" | "addedAt"
        ),
        ResourceFilterKind::Anime => matches!(
            sort.field.as_str(),
            "title" | "lastWatchedAt" | "episodeCount" | "userAffinity" | "addedAt"
        ),
        ResourceFilterKind::Comic => matches!(
            sort.field.as_str(),
            "title" | "lastReadAt" | "chapterCount" | "userAffinity" | "addedAt"
        ),
    };
    if !allowed {
        return Err(invalid("sort field is not whitelisted"));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum FilterValueRule {
    None,
    String,
    StringArray,
    Number,
    Boolean,
    ContentRating,
}

fn filter_rule(kind: ResourceFilterKind, field: &str, op: &str) -> Option<FilterValueRule> {
    use FilterValueRule::*;
    match (kind, field, op) {
        (ResourceFilterKind::Game, "lastPlayedAt", "is_null")
        | (ResourceFilterKind::Anime, "lastWatchedAt", "is_null")
        | (ResourceFilterKind::Comic, "lastReadAt", "is_null") => Some(None),
        (_, "title", "contains") => Some(String),
        (_, "tags", "contains_any" | "contains_all") => Some(StringArray),
        (ResourceFilterKind::Anime, "genres", "contains_any" | "contains_all")
        | (ResourceFilterKind::Comic, "genres", "contains_any" | "contains_all") => {
            Some(StringArray)
        }
        (ResourceFilterKind::Game, "contentRating", "eq") => Some(ContentRating),
        (ResourceFilterKind::Game, "estimatedHours", "lte" | "gte")
        | (ResourceFilterKind::Game, "userAffinity", "lte" | "gte")
        | (ResourceFilterKind::Anime, "episodeCount", "lte" | "gte")
        | (ResourceFilterKind::Anime, "userAffinity", "lte" | "gte")
        | (ResourceFilterKind::Comic, "chapterCount", "lte" | "gte")
        | (ResourceFilterKind::Comic, "userAffinity", "lte" | "gte") => Some(Number),
        (_, "favorite", "eq") | (_, "completed", "eq") => Some(Boolean),
        _ => Option::None,
    }
}

fn require_allowed_id(game_id: &str, allowed_ids: &BTreeSet<String>) -> AiResult<()> {
    if !allowed_ids.contains(game_id) {
        return Err(invalid(
            "library cleanup referenced an ID outside the supplied context",
        ));
    }
    Ok(())
}

fn validate_nonempty_text(field: &str, value: &str, max_chars: usize) -> AiResult<()> {
    let chars = value.trim().chars().count();
    if chars == 0 || chars > max_chars {
        return Err(invalid(format!(
            "{field} must contain between 1 and {max_chars} characters"
        )));
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> AiError {
    AiError::new(AiErrorKind::InvalidOutput, message, false)
}
