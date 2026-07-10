use crate::ai::contracts::{
    AiCapabilities, AiMessage, AiMessageRole, AiProviderConfig, AiProviderKind,
    LibraryChangeSetOutput, LibraryCleanupInput, NaturalLanguageFilterOutput, RecommendationInput,
    RecommendationOutput, ResourceFilterKind, StructuredRequest, StructuredResponse, TokenUsage,
};
use crate::ai::endpoint::{EndpointBinding, EndpointScope, ValidatedEndpoint};
use crate::ai::error::{AiError, AiErrorKind, AiResult};
use crate::ai::governance::{
    authorize_provider_fallback, BudgetLedger, BudgetPolicy, BudgetSnapshot, FallbackAuthorization,
    FixedWindowRateLimiter, RateLimitPolicy,
};
use crate::ai::ollama::OllamaAdapter;
use crate::ai::openai_compatible::OpenAiCompatibleAdapter;
use crate::ai::prompts::{AiUseCase, PromptDefinition, PromptRegistry};
use crate::ai::provider::AiProviderAdapter;
use crate::ai::transport::{
    AiHttpTransport, CancellationProbe, CredentialSource, ReqwestAiTransport,
};
use crate::ai::CancellationToken;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const PROMPT_VERSION: &str = "1.0.0";
const MAX_TASK_INPUT_BYTES: usize = 65_536;
const MAX_PROVIDERS: usize = 4;

pub const DEFAULT_BUDGET_POLICY: BudgetPolicy = BudgetPolicy {
    monthly_hard_limit_tokens: 1_000_000,
    soft_warning_tokens: 800_000,
    per_task_limit_tokens: 100_000,
};
pub const DEFAULT_RATE_LIMIT_POLICY: RateLimitPolicy = RateLimitPolicy {
    max_requests: 8,
    window_ms: 60_000,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiProviderSpec {
    pub id: String,
    pub kind: AiProviderKind,
    pub display_name: String,
    pub base_url: String,
    pub model: String,
    pub enabled: bool,
    #[serde(default)]
    pub max_context_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderStatus {
    pub id: String,
    pub kind: AiProviderKind,
    pub display_name: String,
    pub model: String,
    pub endpoint_origin: String,
    pub endpoint_scope: EndpointScope,
    pub enabled: bool,
    pub credential_configured: bool,
    pub ready: bool,
    pub issue: Option<AiErrorKind>,
    pub capabilities: AiCapabilities,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum AiStructuredTaskInput {
    LibraryCleanup {
        input: LibraryCleanupInput,
    },
    NaturalLanguageFilter {
        query: String,
        kind: ResourceFilterKind,
    },
    Recommendation {
        input: RecommendationInput,
    },
}

impl AiStructuredTaskInput {
    pub fn use_case(&self) -> AiUseCase {
        match self {
            Self::LibraryCleanup { .. } => AiUseCase::LibraryCleanup,
            Self::NaturalLanguageFilter { .. } => AiUseCase::NaturalLanguageFilter,
            Self::Recommendation { .. } => AiUseCase::Recommendation,
        }
    }
    pub fn task_kind(&self) -> &'static str {
        match self {
            Self::LibraryCleanup { .. } => "ai_v2.library_cleanup",
            Self::NaturalLanguageFilter { .. } => "ai_v2.natural_language_filter",
            Self::Recommendation { .. } => "ai_v2.recommendation",
        }
    }
    pub fn task_title(&self) -> &'static str {
        match self {
            Self::LibraryCleanup { .. } => "AI library cleanup",
            Self::NaturalLanguageFilter { .. } => "AI natural-language filter",
            Self::Recommendation { .. } => "AI recommendation explanation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiTaskStartSpec {
    pub providers: Vec<AiProviderSpec>,
    pub primary_provider_id: String,
    #[serde(default)]
    pub fallback_provider_id: Option<String>,
    #[serde(default)]
    pub fallback_authorization: FallbackAuthorization,
    pub task: AiStructuredTaskInput,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum AiStructuredTaskResult {
    LibraryCleanup {
        change_set: LibraryChangeSetOutput,
    },
    NaturalLanguageFilter {
        filter: NaturalLanguageFilterOutput,
    },
    Recommendation {
        recommendation: RecommendationOutput,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiExecutionResult {
    pub task_id: String,
    pub provider_id: String,
    pub model: String,
    pub prompt_id: String,
    pub prompt_version: String,
    pub schema_id: String,
    pub fallback_used: bool,
    pub usage: TokenUsage,
    pub estimated_charged_tokens: u32,
    pub result: AiStructuredTaskResult,
}

pub struct AiOrchestrator<T = ReqwestAiTransport> {
    prompts: PromptRegistry,
    transport: T,
    budget_policy: BudgetPolicy,
    budget: BudgetLedger,
    rate_policy: RateLimitPolicy,
    rate_limiters: Mutex<HashMap<String, Arc<FixedWindowRateLimiter>>>,
    clock_started: Instant,
}

impl AiOrchestrator<ReqwestAiTransport> {
    pub fn production() -> AiResult<Self> {
        Ok(Self::new(
            ReqwestAiTransport::try_new()?,
            DEFAULT_BUDGET_POLICY,
            DEFAULT_RATE_LIMIT_POLICY,
            0,
        ))
    }
}

impl<T: AiHttpTransport> AiOrchestrator<T> {
    pub fn new(
        transport: T,
        budget_policy: BudgetPolicy,
        rate_policy: RateLimitPolicy,
        committed_tokens: u64,
    ) -> Self {
        Self {
            prompts: PromptRegistry,
            transport,
            budget_policy,
            budget: BudgetLedger::new(budget_policy, committed_tokens),
            rate_policy,
            rate_limiters: Mutex::new(HashMap::new()),
            clock_started: Instant::now(),
        }
    }
    pub fn budget_snapshot(&self) -> BudgetSnapshot {
        self.budget.snapshot()
    }
    pub fn provider_status(
        &self,
        provider: &AiProviderSpec,
        credentials: &dyn CredentialSource,
    ) -> AiResult<AiProviderStatus> {
        let prepared = prepare_provider(provider, credentials)?;
        let credential_configured = prepared.adapter.config().secret_configured;
        let ready = provider.enabled
            && (provider.kind != AiProviderKind::OpenAiCompatible || credential_configured);
        Ok(AiProviderStatus {
            id: provider.id.clone(),
            kind: provider.kind,
            display_name: provider.display_name.clone(),
            model: provider.model.clone(),
            endpoint_origin: prepared.endpoint.origin.clone(),
            endpoint_scope: prepared.endpoint.scope,
            enabled: provider.enabled,
            credential_configured,
            ready,
            issue: if ready {
                None
            } else {
                Some(AiErrorKind::NotConfigured)
            },
            capabilities: prepared.adapter.capabilities(),
        })
    }

    pub fn validate_start_spec(
        &self,
        spec: &AiTaskStartSpec,
        credentials: &dyn CredentialSource,
    ) -> AiResult<()> {
        let plan = self.prepare_plan(spec, credentials)?;
        let calls = u32::from(plan.prompt.max_repair_attempts)
            .saturating_add(1)
            .saturating_mul(if plan.fallback.is_some() { 2 } else { 1 });
        if plan.estimated_tokens.saturating_mul(calls) > self.budget_policy.per_task_limit_tokens {
            return Err(AiError::new(
                AiErrorKind::BudgetExceeded,
                "AI task token estimate exceeds the per-task limit",
                false,
            ));
        }
        Ok(())
    }

    pub async fn execute(
        &self,
        task_id: &str,
        spec: &AiTaskStartSpec,
        credentials: &dyn CredentialSource,
        cancellation: &dyn CancellationProbe,
    ) -> AiResult<AiExecutionResult> {
        ensure_active(cancellation)?;
        safe_text("task ID", task_id, 200)?;
        let plan = self.prepare_plan(spec, credentials)?;
        let calls = u32::from(plan.prompt.max_repair_attempts)
            .saturating_add(1)
            .saturating_mul(if plan.fallback.is_some() { 2 } else { 1 });
        let reservation = self
            .budget
            .reserve(plan.estimated_tokens.saturating_mul(calls))?;
        let primary = self
            .run_provider(
                task_id,
                &plan.primary,
                &plan.prompt,
                &plan.request,
                &spec.task,
                plan.estimated_tokens,
                credentials,
                cancellation,
            )
            .await;
        let (outcome, fallback_used, charged) = match primary {
            Ok(success) => {
                let charged = success.charged;
                (Ok(success), false, charged)
            }
            Err(first) if plan.fallback.is_some() && fallback_allowed_for_error(&first.error) => {
                let second = self
                    .run_provider(
                        task_id,
                        plan.fallback.as_ref().unwrap(),
                        &plan.prompt,
                        &plan.request,
                        &spec.task,
                        plan.estimated_tokens,
                        credentials,
                        cancellation,
                    )
                    .await;
                match second {
                    Ok(success) => {
                        let charged = first.charged.saturating_add(success.charged);
                        (Ok(success), true, charged)
                    }
                    Err(failure) => {
                        let charged = first.charged.saturating_add(failure.charged);
                        (Err(failure.error), true, charged)
                    }
                }
            }
            Err(first) => (Err(first.error), false, first.charged),
        };
        if charged == 0 {
            reservation.release();
        } else {
            reservation.commit(charged)?;
        }
        let success = outcome?;
        ensure_active(cancellation)?;
        Ok(AiExecutionResult {
            task_id: task_id.to_string(),
            provider_id: success.response.provider_id,
            model: success.response.model,
            prompt_id: plan.prompt.id.clone(),
            prompt_version: plan.prompt.version.clone(),
            schema_id: plan.prompt.output_schema.id.clone(),
            fallback_used,
            usage: success.response.usage,
            estimated_charged_tokens: charged,
            result: success.result,
        })
    }

    fn prepare_plan(
        &self,
        spec: &AiTaskStartSpec,
        credentials: &dyn CredentialSource,
    ) -> AiResult<ExecutionPlan> {
        validate_task_input(&spec.task)?;
        if spec.providers.is_empty() || spec.providers.len() > MAX_PROVIDERS {
            return Err(policy(
                "AI task must provide between one and four providers",
            ));
        }
        let mut ids = BTreeSet::new();
        for provider in &spec.providers {
            if !ids.insert(provider.id.as_str()) {
                return Err(policy("AI provider IDs must be unique"));
            }
        }
        let primary_spec = select_provider(&spec.providers, &spec.primary_provider_id)?;
        let primary = prepare_provider(primary_spec, credentials)?;
        ensure_selectable(&primary)?;
        let fallback = match spec.fallback_provider_id.as_deref() {
            Some(id) => {
                if id == spec.primary_provider_id {
                    return Err(policy(
                        "fallback provider must differ from primary provider",
                    ));
                }
                let prepared =
                    prepare_provider(select_provider(&spec.providers, id)?, credentials)?;
                ensure_selectable(&prepared)?;
                authorize_provider_fallback(
                    primary.endpoint.scope == EndpointScope::Loopback,
                    prepared.endpoint.scope == EndpointScope::Loopback,
                    spec.fallback_authorization,
                )?;
                Some(prepared)
            }
            None => {
                if spec.fallback_authorization != FallbackAuthorization::Disabled {
                    return Err(policy(
                        "fallback authorization requires a fallback provider",
                    ));
                }
                None
            }
        };
        let prompt = prompt_for_task(&self.prompts, &spec.task)?;
        let request = build_request_template(&prompt, &spec.task, primary_spec)?;
        let estimated_tokens = estimate_tokens(&request)?;
        for provider in std::iter::once(&primary).chain(fallback.iter()) {
            if provider
                .adapter
                .capabilities()
                .max_context_tokens
                .is_some_and(|limit| estimated_tokens > limit)
            {
                return Err(AiError::new(
                    AiErrorKind::BudgetExceeded,
                    "AI task estimate exceeds the provider context limit",
                    false,
                ));
            }
        }
        Ok(ExecutionPlan {
            primary,
            fallback,
            prompt,
            request,
            estimated_tokens,
        })
    }
    #[allow(clippy::too_many_arguments)]
    async fn run_provider(
        &self,
        task_id: &str,
        provider: &PreparedProvider,
        prompt: &PromptDefinition,
        template: &StructuredRequest,
        task: &AiStructuredTaskInput,
        estimate: u32,
        credentials: &dyn CredentialSource,
        cancellation: &dyn CancellationProbe,
    ) -> Result<ProviderSuccess, ProviderFailure> {
        let mut charged = 0_u32;
        let attempts = u32::from(prompt.max_repair_attempts).saturating_add(1);
        for index in 0..attempts {
            if let Err(error) = ensure_active(cancellation) {
                return Err(ProviderFailure { error, charged });
            }
            if let Err(error) = self.check_rate(provider) {
                return Err(ProviderFailure { error, charged });
            }
            let mut request = template.clone();
            request.task_id = task_id.to_string();
            request.model = provider.adapter.config().model.clone();
            let token = CancellationToken::new();
            let guard = token.guard();
            let adapter_request = provider
                .adapter
                .build_request(&request, &guard)
                .map_err(|error| ProviderFailure { error, charged })?;
            let raw = self
                .transport
                .send(
                    &provider.adapter.config().id,
                    &provider.endpoint,
                    &provider.binding,
                    adapter_request,
                    credentials,
                    cancellation,
                    Duration::from_millis(prompt.timeout_ms),
                )
                .await
                .map_err(|error| ProviderFailure { error, charged })?;
            charged = charged.saturating_add(estimate);
            if cancellation.is_cancelled() {
                token.cancel();
            }
            let response = match provider.adapter.parse_response(&raw, &guard) {
                Ok(value) => value,
                Err(error) if error.kind == AiErrorKind::InvalidOutput && index + 1 < attempts => {
                    continue
                }
                Err(error) => return Err(ProviderFailure { error, charged }),
            };
            match validate_result(&self.prompts, prompt, task, &response) {
                Ok(result) => {
                    return Ok(ProviderSuccess {
                        response,
                        result,
                        charged,
                    })
                }
                Err(error) if error.kind == AiErrorKind::InvalidOutput && index + 1 < attempts => {
                    continue
                }
                Err(error) => return Err(ProviderFailure { error, charged }),
            }
        }
        Err(ProviderFailure {
            error: AiError::new(
                AiErrorKind::InvalidOutput,
                "AI provider did not return a valid structured result",
                false,
            ),
            charged,
        })
    }

    fn check_rate(&self, provider: &PreparedProvider) -> AiResult<()> {
        let key = format!(
            "{}|{}",
            provider.adapter.config().id,
            provider.endpoint.origin
        );
        let limiter = {
            let mut map = self.rate_limiters.lock().map_err(|_| {
                AiError::new(
                    AiErrorKind::ProviderUnavailable,
                    "AI rate limiter is unavailable",
                    true,
                )
            })?;
            Arc::clone(
                map.entry(key)
                    .or_insert_with(|| Arc::new(FixedWindowRateLimiter::new(self.rate_policy))),
            )
        };
        let now = u64::try_from(self.clock_started.elapsed().as_millis()).unwrap_or(u64::MAX);
        limiter.check(now)
    }
}

struct ExecutionPlan {
    primary: PreparedProvider,
    fallback: Option<PreparedProvider>,
    prompt: PromptDefinition,
    request: StructuredRequest,
    estimated_tokens: u32,
}
struct PreparedProvider {
    adapter: Box<dyn AiProviderAdapter>,
    endpoint: ValidatedEndpoint,
    binding: EndpointBinding,
}
struct ProviderSuccess {
    response: StructuredResponse,
    result: AiStructuredTaskResult,
    charged: u32,
}
struct ProviderFailure {
    error: AiError,
    charged: u32,
}

fn prepare_provider(
    spec: &AiProviderSpec,
    credentials: &dyn CredentialSource,
) -> AiResult<PreparedProvider> {
    safe_text("provider ID", &spec.id, 80)?;
    safe_text("provider display name", &spec.display_name, 120)?;
    safe_text("provider model", &spec.model, 200)?;
    if spec.max_context_tokens == Some(0) {
        return Err(policy("provider context limit must be greater than zero"));
    }
    if spec.kind == AiProviderKind::Mock {
        return Err(policy(
            "mock providers are not accepted by production AI commands",
        ));
    }
    let endpoint = ValidatedEndpoint::parse(&spec.base_url, spec.kind)?;
    let secret_configured = spec.kind == AiProviderKind::OpenAiCompatible
        && credentials.bearer_configured(&endpoint.origin)?;
    let config = AiProviderConfig {
        id: spec.id.clone(),
        kind: spec.kind,
        display_name: spec.display_name.clone(),
        base_url: endpoint.url.clone(),
        model: spec.model.clone(),
        secret_configured,
        capabilities: AiCapabilities {
            structured_output: true,
            json_mode: true,
            streaming: false,
            vision: false,
            local: endpoint.scope == EndpointScope::Loopback,
            max_context_tokens: spec.max_context_tokens,
        },
        enabled: spec.enabled,
    };
    let adapter: Box<dyn AiProviderAdapter> = match spec.kind {
        AiProviderKind::OpenAiCompatible => Box::new(OpenAiCompatibleAdapter::try_new(config)?),
        AiProviderKind::Ollama => Box::new(OllamaAdapter::try_new(config)?),
        AiProviderKind::Mock => unreachable!(),
    };
    let binding = EndpointBinding {
        provider_id: spec.id.clone(),
        bound_origin: endpoint.origin.clone(),
    };
    binding.authorize_provider(&spec.id, &endpoint)?;
    Ok(PreparedProvider {
        adapter,
        endpoint,
        binding,
    })
}

fn ensure_selectable(provider: &PreparedProvider) -> AiResult<()> {
    if !provider.adapter.config().enabled {
        return Err(AiError::new(
            AiErrorKind::NotConfigured,
            "selected AI provider is disabled",
            false,
        ));
    }
    if !provider.adapter.capabilities().structured_output {
        return Err(policy(
            "selected AI provider does not support structured output",
        ));
    }
    Ok(())
}

fn select_provider<'a>(providers: &'a [AiProviderSpec], id: &str) -> AiResult<&'a AiProviderSpec> {
    safe_text("selected provider ID", id, 80)?;
    providers
        .iter()
        .find(|provider| provider.id == id)
        .ok_or_else(|| {
            AiError::new(
                AiErrorKind::NotConfigured,
                "selected AI provider was not supplied",
                false,
            )
        })
}
fn prompt_for_task(
    registry: &PromptRegistry,
    task: &AiStructuredTaskInput,
) -> AiResult<PromptDefinition> {
    let id = match task {
        AiStructuredTaskInput::LibraryCleanup { .. } => "library_cleanup",
        AiStructuredTaskInput::NaturalLanguageFilter { .. } => "natural_language_filter",
        AiStructuredTaskInput::Recommendation { .. } => "recommendation",
    };
    let prompt = registry.get(id, PROMPT_VERSION)?;
    if prompt.use_case != task.use_case()
        || prompt.output_schema.version != prompt.version
        || prompt.output_schema.id.trim().is_empty()
    {
        return Err(policy(
            "AI prompt registry entry failed integrity validation",
        ));
    }
    Ok(prompt)
}

fn build_request_template(
    prompt: &PromptDefinition,
    task: &AiStructuredTaskInput,
    provider: &AiProviderSpec,
) -> AiResult<StructuredRequest> {
    let context =
        serde_json::to_string(task).map_err(|_| policy("AI task input could not be serialized"))?;
    if context.len() > MAX_TASK_INPUT_BYTES {
        return Err(AiError::new(
            AiErrorKind::BudgetExceeded,
            "AI task input exceeded the context size limit",
            false,
        ));
    }
    Ok(StructuredRequest {
        task_id: "preflight".to_string(),
        prompt_id: prompt.id.clone(),
        prompt_version: prompt.version.clone(),
        schema_id: prompt.output_schema.id.clone(),
        model: provider.model.clone(),
        messages: vec![
            AiMessage {
                role: AiMessageRole::System,
                content: prompt.system_template.clone(),
            },
            AiMessage {
                role: AiMessageRole::User,
                content: format!(
                    "Use only this supplied JSON context and return only schema {}: {context}",
                    prompt.output_schema.id
                ),
            },
        ],
        temperature: 0.1,
        max_output_tokens: prompt.max_output_tokens,
    })
}

fn estimate_tokens(request: &StructuredRequest) -> AiResult<u32> {
    let bytes = request
        .messages
        .iter()
        .map(|message| message.content.len())
        .sum::<usize>();
    let input = u32::try_from(bytes.saturating_add(256).div_ceil(4)).map_err(|_| {
        AiError::new(
            AiErrorKind::BudgetExceeded,
            "AI task token estimate overflowed",
            false,
        )
    })?;
    Ok(input.saturating_add(request.max_output_tokens))
}

fn validate_result(
    registry: &PromptRegistry,
    prompt: &PromptDefinition,
    task: &AiStructuredTaskInput,
    response: &StructuredResponse,
) -> AiResult<AiStructuredTaskResult> {
    match task {
        AiStructuredTaskInput::LibraryCleanup { input } => registry
            .validate_library_cleanup_response(&prompt.version, &response.content, input)
            .map(|value| AiStructuredTaskResult::LibraryCleanup {
                change_set: value.into_inner(),
            }),
        AiStructuredTaskInput::NaturalLanguageFilter { kind, .. } => registry
            .validate_natural_language_filter_response(&prompt.version, &response.content, *kind)
            .map(|value| AiStructuredTaskResult::NaturalLanguageFilter {
                filter: value.into_inner(),
            }),
        AiStructuredTaskInput::Recommendation { input } => registry
            .validate_recommendation_response(&prompt.version, &response.content, input)
            .map(|value| AiStructuredTaskResult::Recommendation {
                recommendation: value.into_inner(),
            }),
    }
}

fn validate_task_input(task: &AiStructuredTaskInput) -> AiResult<()> {
    match task {
        AiStructuredTaskInput::LibraryCleanup { input } => {
            if input.games.is_empty() || input.games.len() > 100 {
                return Err(policy("library cleanup requires between one and 100 games"));
            }
            let mut ids = BTreeSet::new();
            for game in &input.games {
                safe_text("game ID", &game.id, 200)?;
                safe_text("game title", &game.title, 500)?;
                if !ids.insert(game.id.as_str()) {
                    return Err(policy("library cleanup game IDs must be unique"));
                }
                if game
                    .description
                    .as_deref()
                    .is_some_and(|value| value.chars().count() > 4_000)
                    || game.tags.len() > 100
                    || game.metadata.len() > 64
                {
                    return Err(policy("library cleanup context exceeded field limits"));
                }
            }
        }
        AiStructuredTaskInput::NaturalLanguageFilter { query, .. } => {
            safe_text("natural-language query", query, 2_000)?
        }
        AiStructuredTaskInput::Recommendation { input } => {
            if input.candidates.is_empty()
                || input.candidates.len() > 100
                || input.limit == 0
                || input.limit as usize > input.candidates.len().min(50)
            {
                return Err(policy(
                    "recommendation candidate or result limit is invalid",
                ));
            }
            if input
                .request
                .as_deref()
                .is_some_and(|value| value.chars().count() > 2_000)
            {
                return Err(policy("recommendation request exceeded the text limit"));
            }
            let mut ids = BTreeSet::new();
            for candidate in &input.candidates {
                safe_text("candidate ID", &candidate.id, 200)?;
                safe_text("candidate title", &candidate.title, 500)?;
                if !ids.insert(candidate.id.as_str()) {
                    return Err(policy("recommendation candidate IDs must be unique"));
                }
                if candidate.signals.len() > 32
                    || candidate
                        .signals
                        .iter()
                        .any(|signal| signal.trim().is_empty() || signal.chars().count() > 120)
                {
                    return Err(policy("recommendation candidate signals exceeded limits"));
                }
            }
        }
    }
    if serde_json::to_vec(task)
        .map_err(|_| policy("AI task input was invalid"))?
        .len()
        > MAX_TASK_INPUT_BYTES
    {
        return Err(AiError::new(
            AiErrorKind::BudgetExceeded,
            "AI task input exceeded the context size limit",
            false,
        ));
    }
    Ok(())
}

fn safe_text(field: &str, value: &str, max: usize) -> AiResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > max
        || trimmed
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\t'))
    {
        return Err(policy(format!("{field} contains invalid text")));
    }
    Ok(())
}
fn fallback_allowed_for_error(error: &AiError) -> bool {
    matches!(
        error.kind,
        AiErrorKind::NotConfigured
            | AiErrorKind::Auth
            | AiErrorKind::RateLimited
            | AiErrorKind::Timeout
            | AiErrorKind::InvalidOutput
            | AiErrorKind::ProviderUnavailable
    )
}
fn ensure_active(cancellation: &dyn CancellationProbe) -> AiResult<()> {
    if cancellation.is_cancelled() {
        Err(AiError::new(
            AiErrorKind::Cancelled,
            "AI task was cancelled",
            false,
        ))
    } else {
        Ok(())
    }
}
fn policy(message: impl Into<String>) -> AiError {
    AiError::new(AiErrorKind::PolicyRejected, message, false)
}
