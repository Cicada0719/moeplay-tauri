use moeplay_lib::ai::*;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default)]
struct FakeCredentials {
    origins: BTreeSet<String>,
}
impl CredentialSource for FakeCredentials {
    fn bearer_configured(&self, origin: &str) -> AiResult<bool> {
        Ok(self.origins.contains(origin))
    }
    fn bearer_secret(&self, _origin: &str) -> AiResult<Option<String>> {
        Ok(Some("sentinel-never-in-dto".to_string()))
    }
}

#[derive(Default)]
struct ScriptedTransport {
    responses: Mutex<VecDeque<AiResult<AdapterHttpResponse>>>,
    requests: Arc<Mutex<Vec<AdapterHttpRequest>>>,
}
impl ScriptedTransport {
    fn new(responses: Vec<AiResult<AdapterHttpResponse>>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }
}
impl AiHttpTransport for ScriptedTransport {
    #[allow(clippy::too_many_arguments)]
    fn send<'a>(
        &'a self,
        _provider_id: &'a str,
        _endpoint: &'a ValidatedEndpoint,
        _binding: &'a EndpointBinding,
        request: AdapterHttpRequest,
        _credentials: &'a dyn CredentialSource,
        _cancellation: &'a dyn CancellationProbe,
        _timeout: Duration,
    ) -> TransportFuture<'a> {
        self.requests.lock().unwrap().push(request);
        let response = self.responses.lock().unwrap().pop_front().unwrap();
        Box::pin(async move { response })
    }
}

fn orchestrator(transport: ScriptedTransport) -> AiOrchestrator<ScriptedTransport> {
    AiOrchestrator::new(
        transport,
        BudgetPolicy {
            monthly_hard_limit_tokens: 500_000,
            soft_warning_tokens: 400_000,
            per_task_limit_tokens: 100_000,
        },
        RateLimitPolicy {
            max_requests: 20,
            window_ms: 60_000,
        },
        0,
    )
}
fn remote() -> AiProviderSpec {
    AiProviderSpec {
        id: "remote".into(),
        kind: AiProviderKind::OpenAiCompatible,
        display_name: "Remote".into(),
        base_url: "https://api.example.test/v1".into(),
        model: "fixture-model".into(),
        enabled: true,
        max_context_tokens: Some(32_768),
    }
}
fn local() -> AiProviderSpec {
    AiProviderSpec {
        id: "local".into(),
        kind: AiProviderKind::Ollama,
        display_name: "Local".into(),
        base_url: "http://127.0.0.1:11434".into(),
        model: "qwen2.5:7b".into(),
        enabled: true,
        max_context_tokens: Some(32_768),
    }
}
fn cleanup() -> AiStructuredTaskInput {
    AiStructuredTaskInput::LibraryCleanup {
        input: LibraryCleanupInput {
            games: vec![LibraryGameContext {
                id: "game-1".into(),
                title: "Fixture".into(),
                description: None,
                tags: vec![],
                metadata: BTreeMap::new(),
            }],
        },
    }
}
fn response(status: u16, body: String) -> AiResult<AdapterHttpResponse> {
    Ok(AdapterHttpResponse {
        status,
        body: body.into_bytes(),
    })
}
fn openai_valid() -> String {
    json!({"id":"fixture","model":"fixture-model","choices":[{"message":{"content":"{\"summary\":\"Safe\",\"confidence\":0.9,\"operations\":[]}"},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":5}}).to_string()
}

#[tokio::test]
async fn invalid_output_has_one_bounded_repair_and_never_puts_secret_in_adapter_dto() {
    let transport = ScriptedTransport::new(vec![
        response(
            200,
            r#"{"choices":[{"message":{"content":"not json"}}]}"#.to_string(),
        ),
        response(200, openai_valid()),
    ]);
    let requests = Arc::clone(&transport.requests);
    let engine = orchestrator(transport);
    let credentials = FakeCredentials {
        origins: BTreeSet::from(["https://api.example.test".to_string()]),
    };
    let spec = AiTaskStartSpec {
        providers: vec![remote()],
        primary_provider_id: "remote".into(),
        fallback_provider_id: None,
        fallback_authorization: FallbackAuthorization::Disabled,
        task: cleanup(),
    };
    let result = engine
        .execute("task-1", &spec, &credentials, &NeverCancelled)
        .await
        .unwrap();
    assert!(matches!(
        result.result,
        AiStructuredTaskResult::LibraryCleanup { .. }
    ));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests
        .iter()
        .all(|request| !serde_json::to_string(request)
            .unwrap()
            .contains("sentinel-never-in-dto")));
}

#[tokio::test]
async fn local_to_remote_fallback_is_rejected_without_explicit_cross_scope_opt_in() {
    let engine = orchestrator(ScriptedTransport::default());
    let credentials = FakeCredentials {
        origins: BTreeSet::from(["https://api.example.test".to_string()]),
    };
    let mut spec = AiTaskStartSpec {
        providers: vec![local(), remote()],
        primary_provider_id: "local".into(),
        fallback_provider_id: Some("remote".into()),
        fallback_authorization: FallbackAuthorization::SameScopeOnly,
        task: cleanup(),
    };
    assert_eq!(
        engine
            .validate_start_spec(&spec, &credentials)
            .unwrap_err()
            .kind,
        AiErrorKind::PolicyRejected
    );
    spec.fallback_authorization = FallbackAuthorization::ExplicitCrossScope;
    engine.validate_start_spec(&spec, &credentials).unwrap();
}

#[tokio::test]
async fn explicit_retryable_fallback_uses_the_second_provider() {
    let engine = orchestrator(ScriptedTransport::new(vec![
        response(503, "{}".into()),
        response(200, openai_valid()),
    ]));
    let credentials = FakeCredentials {
        origins: BTreeSet::from(["https://api.example.test".to_string()]),
    };
    let spec = AiTaskStartSpec {
        providers: vec![local(), remote()],
        primary_provider_id: "local".into(),
        fallback_provider_id: Some("remote".into()),
        fallback_authorization: FallbackAuthorization::ExplicitCrossScope,
        task: cleanup(),
    };
    let result = engine
        .execute("task-2", &spec, &credentials, &NeverCancelled)
        .await
        .unwrap();
    assert!(result.fallback_used);
    assert_eq!(result.provider_id, "remote");
}
