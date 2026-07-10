use crate::ai::contracts::{
    AiCapabilities, AiMessageRole, AiProviderConfig, StructuredRequest, StructuredResponse,
    TokenUsage,
};
use crate::ai::endpoint::ValidatedEndpoint;
use crate::ai::error::{AiError, AiErrorKind, AiResult};
use crate::ai::governance::CancellationGuard;
use crate::ai::openai_compatible::classify_status;
use crate::ai::provider::{
    AdapterHttpRequest, AdapterHttpResponse, AiProviderAdapter, CredentialRequirement,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use url::Url;

const MAX_RESPONSE_BYTES: usize = 1_048_576;

#[derive(Debug, Clone)]
pub struct OllamaAdapter {
    config: AiProviderConfig,
    endpoint: ValidatedEndpoint,
}

impl OllamaAdapter {
    pub fn try_new(config: AiProviderConfig) -> AiResult<Self> {
        if config.kind != crate::ai::contracts::AiProviderKind::Ollama {
            return Err(AiError::new(
                AiErrorKind::PolicyRejected,
                "Ollama adapter requires an ollama provider config",
                false,
            ));
        }
        let endpoint = ValidatedEndpoint::parse(&config.base_url, config.kind)?;
        Ok(Self { config, endpoint })
    }
}

impl AiProviderAdapter for OllamaAdapter {
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

        Ok(AdapterHttpRequest {
            method: "POST".to_string(),
            url: chat_url(&self.endpoint.url)?,
            headers: BTreeMap::from([
                ("accept".to_string(), "application/json".to_string()),
                ("content-type".to_string(), "application/json".to_string()),
            ]),
            body: json!({
                "model": request.model,
                "messages": messages,
                "stream": false,
                "format": "json",
                "options": {
                    "temperature": request.temperature,
                    "num_predict": request.max_output_tokens
                }
            }),
            credential: CredentialRequirement::None,
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
                "Ollama response exceeded the configured size limit",
                false,
            ));
        }
        let value: Value = serde_json::from_slice(&response.body).map_err(|_| {
            AiError::new(
                AiErrorKind::InvalidOutput,
                "Ollama returned invalid JSON",
                false,
            )
        })?;
        let content = value
            .pointer("/message/content")
            .and_then(Value::as_str)
            .filter(|content| !content.trim().is_empty())
            .ok_or_else(|| {
                AiError::new(
                    AiErrorKind::InvalidOutput,
                    "Ollama response did not include text content",
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
                    .get("prompt_eval_count")
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok()),
                output_tokens: value
                    .get("eval_count")
                    .and_then(Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok()),
            },
            finish_reason: value
                .get("done_reason")
                .and_then(Value::as_str)
                .map(str::to_string),
        })
    }
}

fn chat_url(base: &str) -> AiResult<String> {
    let mut url = Url::parse(base).map_err(|_| {
        AiError::new(
            AiErrorKind::PolicyRejected,
            "validated Ollama endpoint became invalid",
            false,
        )
    })?;
    let path = url.path().trim_end_matches('/');
    let target = if path.ends_with("/api/chat") {
        path.to_string()
    } else if path.is_empty() {
        "/api/chat".to_string()
    } else {
        format!("{path}/api/chat")
    };
    url.set_path(&target);
    Ok(url.to_string())
}
