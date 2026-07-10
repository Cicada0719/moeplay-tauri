use crate::ai::contracts::{
    AiCapabilities, AiMessageRole, AiProviderConfig, StructuredRequest, StructuredResponse,
    TokenUsage,
};
use crate::ai::endpoint::ValidatedEndpoint;
use crate::ai::error::{AiError, AiErrorKind, AiResult};
use crate::ai::governance::CancellationGuard;
use crate::ai::provider::{
    AdapterHttpRequest, AdapterHttpResponse, AiProviderAdapter, CredentialRequirement,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use url::Url;

const MAX_RESPONSE_BYTES: usize = 1_048_576;

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleAdapter {
    config: AiProviderConfig,
    endpoint: ValidatedEndpoint,
}

impl OpenAiCompatibleAdapter {
    pub fn try_new(config: AiProviderConfig) -> AiResult<Self> {
        if config.kind != crate::ai::contracts::AiProviderKind::OpenAiCompatible {
            return Err(AiError::new(
                AiErrorKind::PolicyRejected,
                "OpenAI-compatible adapter requires an openai_compatible provider config",
                false,
            ));
        }
        let endpoint = ValidatedEndpoint::parse(&config.base_url, config.kind)?;
        Ok(Self { config, endpoint })
    }
}

impl AiProviderAdapter for OpenAiCompatibleAdapter {
    fn config(&self) -> &AiProviderConfig {
        &self.config
    }

    fn capabilities(&self) -> AiCapabilities {
        self.config.capabilities.clone()
    }

    fn endpoint(&self) -> &ValidatedEndpoint {
        &self.endpoint
    }

    fn build_request(
        &self,
        request: &StructuredRequest,
        cancellation: &CancellationGuard,
    ) -> AiResult<AdapterHttpRequest> {
        cancellation.ensure_active()?;
        let messages: Vec<Value> = request
            .messages
            .iter()
            .map(|message| {
                let role = match message.role {
                    AiMessageRole::System => "system",
                    AiMessageRole::User => "user",
                    AiMessageRole::Assistant => "assistant",
                };
                json!({"role": role, "content": message.content})
            })
            .collect();

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "temperature": request.temperature,
            "max_tokens": request.max_output_tokens
        });
        if self.config.capabilities.json_mode {
            body["response_format"] = json!({"type": "json_object"});
        }

        Ok(AdapterHttpRequest {
            method: "POST".to_string(),
            url: completion_url(&self.endpoint.url)?,
            headers: BTreeMap::from([
                ("accept".to_string(), "application/json".to_string()),
                ("content-type".to_string(), "application/json".to_string()),
            ]),
            body,
            credential: CredentialRequirement::BearerSecret,
        })
    }

    fn parse_response(
        &self,
        response: &AdapterHttpResponse,
        cancellation: &CancellationGuard,
    ) -> AiResult<StructuredResponse> {
        cancellation.ensure_active()?;
        classify_status(response.status)?;
        if response.body.len() > MAX_RESPONSE_BYTES {
            return Err(AiError::new(
                AiErrorKind::InvalidOutput,
                "AI provider response exceeded the configured size limit",
                false,
            ));
        }
        let value: Value = serde_json::from_slice(&response.body).map_err(|_| {
            AiError::new(
                AiErrorKind::InvalidOutput,
                "AI provider returned invalid JSON",
                false,
            )
        })?;
        let choice = value
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .ok_or_else(|| {
                AiError::new(
                    AiErrorKind::InvalidOutput,
                    "AI provider response did not include a choice",
                    false,
                )
            })?;
        let content = choice
            .pointer("/message/content")
            .and_then(Value::as_str)
            .filter(|content| !content.trim().is_empty())
            .ok_or_else(|| {
                AiError::new(
                    AiErrorKind::InvalidOutput,
                    "AI provider response did not include text content",
                    false,
                )
            })?;

        Ok(StructuredResponse {
            provider_id: self.config.id.clone(),
            model: value
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or(&self.config.model)
                .to_string(),
            content: content.to_string(),
            usage: TokenUsage {
                input_tokens: value
                    .pointer("/usage/prompt_tokens")
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok()),
                output_tokens: value
                    .pointer("/usage/completion_tokens")
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok()),
            },
            finish_reason: choice
                .get("finish_reason")
                .and_then(Value::as_str)
                .map(str::to_string),
        })
    }
}

fn completion_url(base: &str) -> AiResult<String> {
    let mut url = Url::parse(base).map_err(|_| {
        AiError::new(
            AiErrorKind::PolicyRejected,
            "validated OpenAI-compatible endpoint became invalid",
            false,
        )
    })?;
    let path = url.path().trim_end_matches('/');
    let target = if path.ends_with("/chat/completions") {
        path.to_string()
    } else if path.ends_with("/v1") {
        format!("{path}/chat/completions")
    } else if path.is_empty() {
        "/v1/chat/completions".to_string()
    } else {
        format!("{path}/v1/chat/completions")
    };
    url.set_path(&target);
    Ok(url.to_string())
}

pub(crate) fn classify_status(status: u16) -> AiResult<()> {
    match status {
        200..=299 => Ok(()),
        401 | 403 => Err(AiError::new(
            AiErrorKind::Auth,
            "AI provider authentication failed",
            false,
        )),
        408 | 504 => Err(AiError::new(
            AiErrorKind::Timeout,
            "AI provider request timed out",
            true,
        )),
        429 => Err(AiError::new(
            AiErrorKind::RateLimited,
            "AI provider rate limited the request",
            true,
        )),
        500..=599 => Err(AiError::new(
            AiErrorKind::ProviderUnavailable,
            "AI provider is temporarily unavailable",
            true,
        )),
        _ => Err(AiError::new(
            AiErrorKind::ProviderUnavailable,
            format!("AI provider rejected the request with status {status}"),
            false,
        )),
    }
}
