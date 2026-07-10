#![allow(dead_code, unused_imports)]

pub mod domain {
    pub use moeplay_lib::domain::*;
}

#[path = "../src/secret_store.rs"]
mod secret_store;

#[path = "../src/ai/mod.rs"]
mod ai;

use ai::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const OPENAI_LIBRARY_FIXTURE: &str = include_str!("fixtures/ai/openai_library_cleanup_valid.json");
const OLLAMA_FILTER_FIXTURE: &str = include_str!("fixtures/ai/ollama_filter_valid.json");
const INVALID_JSON_FIXTURE: &str = include_str!("fixtures/ai/invalid_json.txt");
const WRONG_ID_FIXTURE: &str = include_str!("fixtures/ai/library_wrong_id.json");
const SQL_FILTER_FIXTURE: &str = include_str!("fixtures/ai/filter_sql_injection.json");

fn capabilities(local: bool) -> AiCapabilities {
    AiCapabilities {
        structured_output: true,
        json_mode: true,
        streaming: false,
        vision: false,
        local,
        max_context_tokens: Some(32_768),
    }
}

fn openai_config(base_url: &str) -> AiProviderConfig {
    AiProviderConfig {
        id: "openai-fixture".to_string(),
        kind: AiProviderKind::OpenAiCompatible,
        display_name: "Fixture OpenAI-compatible".to_string(),
        base_url: base_url.to_string(),
        model: "fixture-model".to_string(),
        secret_configured: true,
        capabilities: capabilities(false),
        enabled: true,
    }
}

fn ollama_config(base_url: &str) -> AiProviderConfig {
    AiProviderConfig {
        id: "ollama-fixture".to_string(),
        kind: AiProviderKind::Ollama,
        display_name: "Fixture Ollama".to_string(),
        base_url: base_url.to_string(),
        model: "qwen2.5:7b".to_string(),
        secret_configured: false,
        capabilities: capabilities(true),
        enabled: true,
    }
}

fn request(model: &str, prompt_id: &str, schema_id: &str) -> StructuredRequest {
    StructuredRequest {
        task_id: "task-fixture".to_string(),
        prompt_id: prompt_id.to_string(),
        prompt_version: "1.0.0".to_string(),
        schema_id: schema_id.to_string(),
        model: model.to_string(),
        messages: vec![
            AiMessage {
                role: AiMessageRole::System,
                content: "Return JSON only.".to_string(),
            },
            AiMessage {
                role: AiMessageRole::User,
                content: "Fixture request".to_string(),
            },
        ],
        temperature: 0.1,
        max_output_tokens: 800,
    }
}

fn library_input() -> LibraryCleanupInput {
    LibraryCleanupInput {
        games: vec![
            LibraryGameContext {
                id: "game-1".to_string(),
                title: " fixture game ".to_string(),
                description: None,
                tags: vec!["relaxing".to_string()],
                metadata: BTreeMap::new(),
            },
            LibraryGameContext {
                id: "game-2".to_string(),
                title: "Fixture Game".to_string(),
                description: None,
                tags: vec![],
                metadata: BTreeMap::new(),
            },
        ],
    }
}

#[test]
fn endpoint_policy_requires_remote_https_and_forbids_url_credentials() {
    assert!(ValidatedEndpoint::parse(
        "https://api.example.test/v1",
        AiProviderKind::OpenAiCompatible
    )
    .is_ok());
    assert!(ValidatedEndpoint::parse(
        "http://api.example.test/v1",
        AiProviderKind::OpenAiCompatible
    )
    .is_err());
    assert!(ValidatedEndpoint::parse(
        "https://user:password@api.example.test/v1",
        AiProviderKind::OpenAiCompatible
    )
    .is_err());
    assert!(ValidatedEndpoint::parse(
        "https://api.example.test/v1?token=sentinel",
        AiProviderKind::OpenAiCompatible
    )
    .is_err());
}

#[test]
fn loopback_http_is_allowed_only_for_explicit_local_provider() {
    let local = ValidatedEndpoint::parse("http://127.0.0.1:11434", AiProviderKind::Ollama)
        .expect("loopback Ollama must be allowed");
    assert_eq!(local.scope, EndpointScope::Loopback);
    assert!(ValidatedEndpoint::parse(
        "http://127.0.0.1:11434/v1",
        AiProviderKind::OpenAiCompatible
    )
    .is_err());
    assert!(ValidatedEndpoint::parse("http://192.168.1.20:11434", AiProviderKind::Ollama).is_err());
}

#[test]
fn endpoint_origin_change_requires_explicit_secret_rebinding() {
    let original = ValidatedEndpoint::parse(
        "https://api.example.test/v1",
        AiProviderKind::OpenAiCompatible,
    )
    .unwrap();
    let changed = ValidatedEndpoint::parse(
        "https://other.example.test/v1",
        AiProviderKind::OpenAiCompatible,
    )
    .unwrap();
    let binding = EndpointBinding {
        provider_id: "openai-fixture".to_string(),
        bound_origin: original.origin.clone(),
    };
    binding.authorize(&original).unwrap();
    assert_eq!(
        binding.authorize(&changed).unwrap_err().kind,
        AiErrorKind::PolicyRejected
    );
}

#[test]
fn openai_compatible_adapter_is_secret_free_and_validates_fixture_output() {
    assert!(OpenAiCompatibleAdapter::try_new(ollama_config("http://127.0.0.1:11434")).is_err());
    let adapter =
        OpenAiCompatibleAdapter::try_new(openai_config("https://api.example.test/v1")).unwrap();
    let token = CancellationToken::new();
    let guard = token.guard();
    let http_request = adapter
        .build_request(
            &request(
                "fixture-model",
                "library_cleanup",
                "library_cleanup.change_set",
            ),
            &guard,
        )
        .unwrap();

    assert_eq!(
        http_request.url,
        "https://api.example.test/v1/chat/completions"
    );
    assert_eq!(http_request.credential, CredentialRequirement::BearerSecret);
    assert!(!http_request
        .headers
        .keys()
        .any(|name| name.eq_ignore_ascii_case("authorization")));
    let serialized = serde_json::to_string(&http_request).unwrap();
    assert!(!serialized.contains("sentinel-secret"));
    assert!(!serialized.to_ascii_lowercase().contains("authorization"));

    let response = adapter
        .parse_response(
            &AdapterHttpResponse {
                status: 200,
                body: OPENAI_LIBRARY_FIXTURE.as_bytes().to_vec(),
            },
            &guard,
        )
        .unwrap();
    assert_eq!(response.usage.input_tokens, Some(120));

    let validated = PromptRegistry
        .validate_library_cleanup_response("1.0.0", &response.content, &library_input())
        .unwrap();
    let preview = build_library_change_set_preview("change-1", "task-fixture", validated);
    assert_eq!(preview.state, ChangeSetState::AwaitingConfirmation);
    assert_eq!(preview.operations.len(), 2);
    assert!(preview
        .operations
        .iter()
        .all(|operation| !operation.selected));
}

#[test]
fn ollama_adapter_needs_no_secret_and_validates_filter_fixture() {
    assert!(OllamaAdapter::try_new(openai_config("https://api.example.test/v1")).is_err());
    let adapter = OllamaAdapter::try_new(ollama_config("http://[::1]:11434")).unwrap();
    let token = CancellationToken::new();
    let guard = token.guard();
    let http_request = adapter
        .build_request(
            &request(
                "qwen2.5:7b",
                "natural_language_filter",
                "natural_language_filter.dsl",
            ),
            &guard,
        )
        .unwrap();
    assert_eq!(http_request.url, "http://[::1]:11434/api/chat");
    assert_eq!(http_request.credential, CredentialRequirement::None);

    let response = adapter
        .parse_response(
            &AdapterHttpResponse {
                status: 200,
                body: OLLAMA_FILTER_FIXTURE.as_bytes().to_vec(),
            },
            &guard,
        )
        .unwrap();
    let filter = PromptRegistry
        .validate_natural_language_filter_response(
            "1.0.0",
            &response.content,
            ResourceFilterKind::Game,
        )
        .unwrap();
    assert_eq!(filter.value().filters.len(), 4);
    assert_eq!(filter.value().sort[0].field, "userAffinity");
}

#[test]
fn invalid_json_and_wrong_ids_never_produce_applicable_change_sets() {
    let registry = PromptRegistry;
    let invalid_json = registry
        .validate_library_cleanup_response("1.0.0", INVALID_JSON_FIXTURE, &library_input())
        .unwrap_err();
    assert_eq!(invalid_json.kind, AiErrorKind::InvalidOutput);

    let wrong_id = registry
        .validate_library_cleanup_response("1.0.0", WRONG_ID_FIXTURE, &library_input())
        .unwrap_err();
    assert_eq!(wrong_id.kind, AiErrorKind::InvalidOutput);
    assert!(wrong_id.message.contains("outside the supplied context"));
}

#[test]
fn natural_language_filter_rejects_non_whitelisted_dsl() {
    let error = PromptRegistry
        .validate_natural_language_filter_response(
            "1.0.0",
            SQL_FILTER_FIXTURE,
            ResourceFilterKind::Game,
        )
        .unwrap_err();
    assert_eq!(error.kind, AiErrorKind::InvalidOutput);
}

#[test]
fn budget_rate_limit_and_cancellation_are_deterministic() {
    let ledger = BudgetLedger::new(
        BudgetPolicy {
            monthly_hard_limit_tokens: 1_000,
            soft_warning_tokens: 800,
            per_task_limit_tokens: 600,
        },
        300,
    );
    let reservation = ledger.reserve(500).unwrap();
    assert!(ledger.snapshot().soft_warning_reached);
    reservation.commit(450).unwrap();
    assert_eq!(ledger.snapshot().committed_tokens, 750);
    assert_eq!(
        ledger.reserve(300).unwrap_err().kind,
        AiErrorKind::BudgetExceeded
    );

    let limiter = FixedWindowRateLimiter::new(RateLimitPolicy {
        max_requests: 2,
        window_ms: 1_000,
    });
    limiter.check(10_000).unwrap();
    limiter.check(10_001).unwrap();
    let limited = limiter.check(10_100).unwrap_err();
    assert_eq!(limited.kind, AiErrorKind::RateLimited);
    assert_eq!(limited.retry_after_ms, Some(900));
    limiter.check(11_000).unwrap();

    let token = CancellationToken::new();
    let guard = token.guard();
    guard.ensure_active().unwrap();
    token.cancel();
    assert_eq!(
        guard.ensure_active().unwrap_err().kind,
        AiErrorKind::Cancelled
    );
}

#[test]
fn redaction_and_fallback_policy_never_silently_promote_local_to_remote() {
    let sentinel = "sentinel-secret-123";
    let headers = BTreeMap::from([
        ("Authorization".to_string(), format!("Bearer {sentinel}")),
        ("x-api-key".to_string(), sentinel.to_string()),
        ("content-type".to_string(), "application/json".to_string()),
    ]);
    let redacted = redact_headers(&headers);
    let serialized = serde_json::to_string(&redacted).unwrap();
    assert!(!serialized.contains(sentinel));
    assert_eq!(
        redact_url("https://user:pass@example.test/v1?token=abc"),
        "https://example.test/v1"
    );
    assert!(!redact_text(&format!("failure {sentinel}"), &[sentinel]).contains(sentinel));
    assert!(
        !redact_text("request failed Bearer bearer-token-sentinel", &[])
            .contains("bearer-token-sentinel")
    );

    assert_eq!(
        authorize_provider_fallback(true, false, FallbackAuthorization::Disabled)
            .unwrap_err()
            .kind,
        AiErrorKind::PolicyRejected
    );
    assert_eq!(
        authorize_provider_fallback(true, false, FallbackAuthorization::SameScopeOnly)
            .unwrap_err()
            .kind,
        AiErrorKind::PolicyRejected
    );
    authorize_provider_fallback(true, false, FallbackAuthorization::ExplicitCrossScope).unwrap();
}

#[test]
fn ai_errors_map_to_existing_domain_provider_error_concepts() {
    let mapped = AiError::new(
        AiErrorKind::InvalidOutput,
        "invalid structured output",
        false,
    )
    .to_provider_error("openai-fixture", "generate_structured");
    assert_eq!(mapped.kind, domain::ProviderErrorKind::ParseChanged);
    assert_eq!(mapped.provider_id.as_deref(), Some("openai-fixture"));
}

#[test]
fn recommendation_registry_rejects_excluded_or_hallucinated_ids() {
    const VALID: &str = include_str!("fixtures/ai/recommendation_valid.json");
    const EXCLUDED: &str = include_str!("fixtures/ai/recommendation_excluded.json");
    let input = RecommendationInput {
        candidates: vec![
            RecommendationCandidateContext {
                id: "game-1".to_string(),
                title: "Fixture Game".to_string(),
                kind: ResourceFilterKind::Game,
                available: true,
                estimated_minutes: Some(45),
                signals: vec!["short_session".to_string(), "relaxing".to_string()],
            },
            RecommendationCandidateContext {
                id: "excluded-game".to_string(),
                title: "Excluded".to_string(),
                kind: ResourceFilterKind::Game,
                available: true,
                estimated_minutes: None,
                signals: vec!["relaxing".to_string()],
            },
        ],
        excluded_ids: std::collections::BTreeSet::from(["excluded-game".to_string()]),
        limit: 2,
        request: Some("something relaxing".to_string()),
    };
    let registry = PromptRegistry;
    let valid = registry
        .validate_recommendation_response("1.0.0", VALID, &input)
        .unwrap();
    assert_eq!(valid.value().recommendations[0].resource_id, "game-1");
    assert_eq!(
        registry
            .validate_recommendation_response("1.0.0", EXCLUDED, &input)
            .unwrap_err()
            .kind,
        AiErrorKind::InvalidOutput
    );
}

#[test]
fn request_url_must_stay_on_bound_origin() {
    let endpoint = ValidatedEndpoint::parse(
        "https://api.example.test/v1",
        AiProviderKind::OpenAiCompatible,
    )
    .unwrap();
    endpoint
        .authorize_request_url("https://api.example.test/v1/chat/completions")
        .unwrap();
    assert_eq!(
        endpoint
            .authorize_request_url("https://other.example.test/v1/chat/completions")
            .unwrap_err()
            .kind,
        AiErrorKind::PolicyRejected
    );
}
