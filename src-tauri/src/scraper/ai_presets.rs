// 萌游 MoeGame · AI Provider 多预设系统（M3）
//
// 支持：
//   - 多 Provider：OpenAI / DeepSeek / Claude / Ollama / 自定义
//   - 多 Preset：中文描述增强 / 标签补全 / 翻译 / 背景词 / 分级推断
//   - 每个 preset 可指定 provider + model + 自定义 prompt
//   - 链式回退（provider A 失败→ provider B）

use serde::{Deserialize, Serialize};
use url::{Host, Url};

// ============================================================================
// Provider 定义
// ============================================================================

#[derive(Debug, Clone)]
pub struct AiProvider {
    /// 唯一标识
    pub id: String,
    /// 显示名
    pub name: String,
    /// API URL（chat completions 端点）
    pub api_url: String,
    /// API Key
    api_key: String,
    /// 默认模型
    pub default_model: String,
    /// 是否启用
    pub enabled: bool,
    /// 超时（秒）
    pub timeout_secs: u64,
    /// Provider 类型（用于特殊处理）
    pub provider_type: ProviderType,
    /// 当前后端是否实现了该 Provider 的正确请求协议。
    pub supported: bool,
    /// 不可用时提供给 UI/调用方的安全说明。
    pub disabled_reason: Option<String>,
}

impl AiProvider {
    pub(crate) fn api_key(&self) -> &str {
        &self.api_key
    }
}

/// 可安全返回给前端的 Provider 视图。
///
/// 该 DTO 从类型层面排除了 API Key，避免后续命令误把凭据序列化出去。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AiProviderDto {
    pub id: String,
    pub name: String,
    pub api_url: String,
    pub default_model: String,
    pub enabled: bool,
    pub timeout_secs: u64,
    pub provider_type: ProviderType,
    pub configured: bool,
    pub key_required: bool,
    pub supported: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ProviderType {
    #[default]
    OpenAI,
    DeepSeek,
    Anthropic,
    Ollama,
    Custom,
}

/// 返回预置 Provider 列表。
pub fn builtin_providers() -> Vec<AiProvider> {
    vec![
        AiProvider {
            id: "openai".into(),
            name: "OpenAI (GPT-4o-mini)".into(),
            api_url: "https://api.openai.com/v1/chat/completions".into(),
            api_key: String::new(),
            default_model: "gpt-4o-mini".into(),
            enabled: false,
            timeout_secs: 30,
            provider_type: ProviderType::OpenAI,
            supported: true,
            disabled_reason: None,
        },
        AiProvider {
            id: "deepseek".into(),
            name: "DeepSeek (V3)".into(),
            api_url: "https://api.deepseek.com/v1/chat/completions".into(),
            api_key: String::new(),
            default_model: "deepseek-chat".into(),
            enabled: false,
            timeout_secs: 30,
            provider_type: ProviderType::DeepSeek,
            supported: true,
            disabled_reason: None,
        },
        AiProvider {
            id: "ollama".into(),
            name: "Ollama (本地)".into(),
            api_url: "http://localhost:11434/v1/chat/completions".into(),
            api_key: String::new(),
            default_model: "qwen2.5:7b".into(),
            enabled: false,
            timeout_secs: 120,
            provider_type: ProviderType::Ollama,
            supported: true,
            disabled_reason: None,
        },
        AiProvider {
            id: "claude".into(),
            name: "Anthropic Claude".into(),
            api_url: "https://api.anthropic.com/v1/messages".into(),
            api_key: String::new(),
            default_model: "claude-3-5-haiku-20241022".into(),
            enabled: false,
            timeout_secs: 30,
            provider_type: ProviderType::Anthropic,
            supported: false,
            disabled_reason: Some(
                "Anthropic 请求协议尚未实现；为避免发送错误请求，当前已禁用".into(),
            ),
        },
    ]
}

/// 经过安全策略检查的 endpoint 信息。
#[derive(Debug, Clone)]
pub struct ValidatedEndpoint {
    url: Url,
    provider_type: ProviderType,
    local: bool,
}

impl ValidatedEndpoint {
    pub fn as_str(&self) -> &str {
        self.url.as_str()
    }

    pub fn provider_type(&self) -> ProviderType {
        self.provider_type
    }

    pub fn is_local(&self) -> bool {
        self.local
    }
}

/// 验证 AI endpoint：远端仅允许 HTTPS，本地回环地址允许 HTTP。
pub fn validate_endpoint(api_url: &str) -> Result<ValidatedEndpoint, String> {
    let url = Url::parse(api_url.trim()).map_err(|_| "AI endpoint 不是有效 URL".to_string())?;

    if !url.username().is_empty() || url.password().is_some() {
        return Err("AI endpoint 不允许包含 URL credentials".to_string());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("AI endpoint 不允许包含 query 或 fragment".to_string());
    }

    let local = match url.host() {
        Some(Host::Domain(host)) => host.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(host)) => host.is_loopback() && host.octets() == [127, 0, 0, 1],
        Some(Host::Ipv6(host)) => host.is_loopback(),
        None => return Err("AI endpoint 缺少主机名".to_string()),
    };

    match url.scheme() {
        "https" => {}
        "http" if local => {}
        "http" => return Err("远端 AI endpoint 必须使用 HTTPS".to_string()),
        _ => return Err("AI endpoint 仅支持 http(s)".to_string()),
    }

    let provider_type = match url
        .host_str()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "api.openai.com" => ProviderType::OpenAI,
        "api.deepseek.com" => ProviderType::DeepSeek,
        "api.anthropic.com" => ProviderType::Anthropic,
        _ if local => ProviderType::Ollama,
        _ => ProviderType::Custom,
    };

    Ok(ValidatedEndpoint {
        url,
        provider_type,
        local,
    })
}

pub fn key_required(provider_type: ProviderType) -> bool {
    !matches!(provider_type, ProviderType::Ollama)
}

/// 验证内部 Provider 与 endpoint/凭据来源契约是否一致。
pub fn validate_provider_contract(provider: &AiProvider) -> Result<ValidatedEndpoint, String> {
    if !provider.enabled {
        return Err(format!("AI provider '{}' 未启用", provider.id));
    }
    if !provider.supported || matches!(provider.provider_type, ProviderType::Anthropic) {
        return Err(provider
            .disabled_reason
            .clone()
            .unwrap_or_else(|| format!("AI provider '{}' 当前不受支持", provider.id)));
    }

    let endpoint = validate_endpoint(&provider.api_url)?;
    let canonical_origin_matches = match provider.provider_type {
        ProviderType::OpenAI | ProviderType::DeepSeek | ProviderType::Anthropic => {
            endpoint.url.scheme() == "https" && endpoint.url.port_or_known_default() == Some(443)
        }
        ProviderType::Ollama => endpoint.is_local(),
        ProviderType::Custom => true,
    };
    if endpoint.provider_type() != provider.provider_type || !canonical_origin_matches {
        return Err(format!(
            "AI provider '{}' 与 endpoint 来源不匹配，已拒绝发送凭据",
            provider.id
        ));
    }
    if matches!(provider.provider_type, ProviderType::Custom) {
        return Err(
            "自定义远端 endpoint 无法与旧版 settings API Key 安全绑定，已拒绝请求".to_string(),
        );
    }
    if key_required(provider.provider_type) && provider.api_key.trim().is_empty() {
        return Err(format!("AI provider '{}' 需要 API Key", provider.id));
    }
    if provider.default_model.trim().is_empty() {
        return Err(format!("AI provider '{}' 未配置模型", provider.id));
    }

    Ok(endpoint)
}

/// 从旧版单 Provider settings 安全解析内部 Provider。
///
/// 旧 settings 没有保存独立的 provider/key 来源，因此只允许可由 canonical origin
/// 明确识别的 OpenAI、DeepSeek、Anthropic 和本地 Ollama。未知远端 origin 会被拒绝。
pub fn provider_from_legacy_settings(
    api_url: &str,
    api_key: &str,
    model: &str,
) -> Result<AiProvider, String> {
    let endpoint = validate_endpoint(api_url)?;
    let mut provider = builtin_providers()
        .into_iter()
        .find(|provider| provider.provider_type == endpoint.provider_type())
        .ok_or_else(|| {
            "自定义远端 endpoint 无法与旧版 settings API Key 安全绑定，已拒绝请求".to_string()
        })?;

    provider.api_url = endpoint.as_str().to_string();
    provider.default_model = model.trim().to_string();
    provider.enabled = true;
    // 本地 Provider 不需要 Key，也绝不应把旧远端 Key 转发到本地服务。
    provider.api_key = if endpoint.is_local() {
        String::new()
    } else {
        api_key.trim().to_string()
    };
    validate_provider_contract(&provider)?;
    Ok(provider)
}

/// 生成供 get_ai_providers 返回的安全视图。
pub fn provider_dtos_for_settings(
    ai_enabled: bool,
    api_url: &str,
    api_key: &str,
    model: &str,
) -> Vec<AiProviderDto> {
    let endpoint = validate_endpoint(api_url).ok();
    let resolved = provider_from_legacy_settings(api_url, api_key, model);
    let resolved_type = resolved
        .as_ref()
        .ok()
        .map(|provider| provider.provider_type);
    let resolution_error = resolved.as_ref().err().cloned();

    let mut providers: Vec<_> = builtin_providers()
        .into_iter()
        .map(|provider| {
            let is_selected = endpoint
                .as_ref()
                .is_some_and(|endpoint| endpoint.provider_type() == provider.provider_type);
            let configured = resolved_type == Some(provider.provider_type);
            let disabled_reason = if is_selected {
                resolution_error
                    .clone()
                    .or_else(|| provider.disabled_reason.clone())
            } else {
                provider.disabled_reason.clone()
            };

            AiProviderDto {
                id: provider.id,
                name: provider.name,
                api_url: if is_selected {
                    endpoint
                        .as_ref()
                        .map(|endpoint| endpoint.as_str().to_string())
                        .unwrap_or(provider.api_url)
                } else {
                    provider.api_url
                },
                default_model: if is_selected && !model.trim().is_empty() {
                    model.trim().to_string()
                } else {
                    provider.default_model
                },
                enabled: ai_enabled && configured,
                timeout_secs: provider.timeout_secs,
                provider_type: provider.provider_type,
                configured,
                key_required: key_required(provider.provider_type),
                supported: provider.supported,
                disabled_reason,
            }
        })
        .collect();

    if let Some(endpoint) =
        endpoint.filter(|endpoint| matches!(endpoint.provider_type(), ProviderType::Custom))
    {
        providers.push(AiProviderDto {
            id: "custom".to_string(),
            name: "Custom OpenAI-compatible".to_string(),
            api_url: endpoint.as_str().to_string(),
            default_model: model.trim().to_string(),
            enabled: false,
            timeout_secs: 30,
            provider_type: ProviderType::Custom,
            configured: false,
            key_required: true,
            supported: true,
            disabled_reason: resolution_error.or_else(|| {
                Some(
                    "旧版 settings 未记录 API Key 来源，无法安全绑定到自定义远端 origin"
                        .to_string(),
                )
            }),
        });
    }

    providers
}

// ============================================================================
// Preset 定义
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPreset {
    /// 唯一标识
    pub id: String,
    /// 显示名
    pub name: String,
    /// 用途说明
    pub description: String,
    /// 绑定的 provider id
    pub provider_id: String,
    /// 覆盖 provider 的默认模型
    pub model_override: Option<String>,
    /// System prompt
    pub system_prompt: String,
    /// Temperature (0.0-2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    /// 最大 token
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// 是否启用
    pub enabled: bool,
}

fn default_temperature() -> f64 {
    0.3
}
fn default_max_tokens() -> u32 {
    1024
}

/// 返回预置 Preset 列表。
pub fn builtin_presets() -> Vec<AiPreset> {
    vec![
        AiPreset {
            id: "enhance_description".into(),
            name: "描述增强".into(),
            description: "用中文生成 200 字内游戏简介".into(),
            provider_id: "openai".into(),
            model_override: None,
            system_prompt: "你是一位资深 galgame 评论家。请为以下游戏写一段中文简介（不超过200字），涵盖剧情、氛围和亮点。不要包含评分。".into(),
            temperature: 0.5,
            max_tokens: 512,
            enabled: true,
        },
        AiPreset {
            id: "classify_tags".into(),
            name: "标签补全".into(),
            description: "根据游戏名和已有标签，补充 5-8 个分类标签（类型/主题/氛围/特性）".into(),
            provider_id: "openai".into(),
            model_override: None,
            system_prompt: "你是 galgame 标签分类专家。根据游戏名称和已有描述，推荐5-8个标签，每个标签注明分类（genre/theme/mood/feature/content）。返回 JSON 格式：{\"tags\":[{\"name\":\"...\",\"category\":\"...\"}]}".into(),
            temperature: 0.3,
            max_tokens: 512,
            enabled: true,
        },
        AiPreset {
            id: "translate_to_zh".into(),
            name: "翻译为中文".into(),
            description: "将日英标题/简介翻译为中文".into(),
            provider_id: "openai".into(),
            model_override: None,
            system_prompt: "你是日→中翻译专家。将以下游戏标题和简介翻译为自然流畅的中文。若无日文则保持英文标题并给出中文意译。返回 JSON：{\"name_cn\":\"...\",\"desc_cn\":\"...\"}".into(),
            temperature: 0.1,
            max_tokens: 512,
            enabled: true,
        },
        AiPreset {
            id: "background_keyword".into(),
            name: "背景图关键词".into(),
            description: "生成适合搜索背景图的英文关键词".into(),
            provider_id: "openai".into(),
            model_override: None,
            system_prompt: "根据游戏名称和简介，生成1-3个适合在图片网站搜索高清背景/壁纸的英文关键词。返回 JSON：{\"keywords\":[\"...\"]}".into(),
            temperature: 0.3,
            max_tokens: 256,
            enabled: false,
        },
        AiPreset {
            id: "infer_age_rating".into(),
            name: "分级推断".into(),
            description: "根据标签/描述推断年龄分级（全年龄/软/R18）".into(),
            provider_id: "openai".into(),
            model_override: None,
            system_prompt: "根据游戏标签和描述判断年龄分级：all_ages（全年龄）、soft_r18（轻微成人内容）、r18（成人内容）。返回 JSON：{\"level\":\"...\",\"confidence\":0.0-1.0}".into(),
            temperature: 0.1,
            max_tokens: 128,
            enabled: false,
        },
        AiPreset {
            id: "recognize_title".into(),
            name: "标题识别".into(),
            description: "从文件夹名/文件名识别规范游戏标题".into(),
            provider_id: "openai".into(),
            model_override: None,
            system_prompt: "你是游戏标题识别专家。给定文件夹名或文件名，识别出游戏的规范标题（去版本号、去社名、去汉化组信息）。返回 JSON：{\"title\":\"...\",\"confidence\":0.0-1.0}".into(),
            temperature: 0.1,
            max_tokens: 256,
            enabled: true,
        },
    ]
}

// ============================================================================
// LLM 调用（统一接口）
// ============================================================================

/// 调用 LLM（OpenAI-compatible Chat Completions API）。
///
/// Anthropic 当前明确禁用，直到实现正确的 Messages API 请求体。
pub async fn call_llm(
    provider: &AiProvider,
    model: &str,
    system_prompt: &str,
    user_message: &str,
    temperature: f64,
    max_tokens: u32,
) -> Result<String, String> {
    validate_provider_contract(provider)?;
    if model.trim().is_empty() {
        return Err("AI model 未配置".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(provider.timeout_secs))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "无法创建 AI HTTP 客户端".to_string())?;

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_message}
        ],
        "temperature": temperature,
        "max_tokens": max_tokens
    });

    let mut req = client
        .post(&provider.api_url)
        .header("Content-Type", "application/json")
        .header("User-Agent", crate::http_client::app_user_agent());

    if key_required(provider.provider_type) {
        req = req.header("Authorization", format!("Bearer {}", provider.api_key()));
    }

    let resp = req
        .json(&body)
        .send()
        .await
        .map_err(|_| "AI 请求失败".to_string())?;

    if !resp.status().is_success() {
        return Err(format!("AI 请求返回 HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|_| "AI 响应 JSON 解析失败".to_string())?;

    if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
        return Ok(content.to_string());
    }

    Err("AI 响应缺少 choices[0].message.content".to_string())
}

/// 提取 JSON（处理 markdown 代码块包装）。
pub fn extract_json(text: &str) -> Option<serde_json::Value> {
    let text = text.trim();
    // 尝试直接解析
    if let Ok(v) = serde_json::from_str(text) {
        return Some(v);
    }
    // 尝试 ```json ... ``` 包装
    if let Some(inner) = text
        .strip_prefix("```json")
        .and_then(|s| s.strip_suffix("```"))
        .or_else(|| text.strip_prefix("```").and_then(|s| s.strip_suffix("```")))
    {
        if let Ok(v) = serde_json::from_str(inner.trim()) {
            return Some(v);
        }
    }
    // 尝试找到 { 到 } 区间
    if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
        let json_str = &text[start..=end];
        if let Ok(v) = serde_json::from_str(json_str) {
            return Some(v);
        }
    }
    None
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_providers() {
        let providers = builtin_providers();
        assert_eq!(providers.len(), 4);
        assert_eq!(providers[0].id, "openai");
        assert_eq!(providers[2].id, "ollama");
    }

    #[test]
    fn test_builtin_presets() {
        let presets = builtin_presets();
        assert!(!presets.is_empty());
        assert!(presets.iter().any(|p| p.id == "enhance_description"));
        assert!(presets.iter().any(|p| p.id == "recognize_title"));
    }

    #[test]
    fn test_extract_json_bare() {
        let json = extract_json(r#"{"name":"test","value":42}"#);
        assert!(json.is_some());
        assert_eq!(json.unwrap()["name"], "test");
    }

    #[test]
    fn test_extract_json_markdown() {
        let json = extract_json("```json\n{\"name\":\"test\"}\n```");
        assert!(json.is_some());
        assert_eq!(json.unwrap()["name"], "test");
    }

    #[test]
    fn test_extract_json_with_text() {
        let json = extract_json("Here is the result: {\"name\":\"test\"} done.");
        assert!(json.is_some());
        assert_eq!(json.unwrap()["name"], "test");
    }

    #[test]
    fn test_extract_json_invalid() {
        assert!(extract_json("no json here").is_none());
    }

    #[test]
    fn provider_dto_never_serializes_api_key() {
        let secret = "sk-contract-secret";
        let providers = provider_dtos_for_settings(
            true,
            "https://api.openai.com/v1/chat/completions",
            secret,
            "gpt-4o-mini",
        );
        let json = serde_json::to_string(&providers).unwrap();

        assert!(!json.contains(secret));
        assert!(!json.contains("api_key"));
        let openai = providers
            .iter()
            .find(|provider| provider.id == "openai")
            .unwrap();
        assert!(openai.configured);
        assert!(openai.key_required);
        assert!(openai.enabled);
    }

    #[test]
    fn ollama_is_configured_without_api_key() {
        let providers = provider_dtos_for_settings(
            true,
            "http://localhost:11434/v1/chat/completions",
            "",
            "qwen2.5:7b",
        );
        let ollama = providers
            .iter()
            .find(|provider| provider.id == "ollama")
            .unwrap();

        assert!(ollama.configured);
        assert!(!ollama.key_required);
        assert!(ollama.enabled);

        let provider = provider_from_legacy_settings(
            "http://127.0.0.1:11434/v1/chat/completions",
            "stale-remote-key-must-not-be-forwarded",
            "qwen2.5:7b",
        )
        .unwrap();
        assert!(provider.api_key().is_empty());
        assert!(validate_provider_contract(&provider).is_ok());
    }

    #[test]
    fn endpoint_policy_accepts_only_https_remote_or_loopback_http() {
        for url in [
            "https://api.openai.com/v1/chat/completions",
            "http://localhost:11434/v1/chat/completions",
            "http://127.0.0.1:11434/v1/chat/completions",
            "http://[::1]:11434/v1/chat/completions",
        ] {
            assert!(validate_endpoint(url).is_ok(), "expected allowed: {url}");
        }

        for url in [
            "http://api.openai.com/v1/chat/completions",
            "http://192.168.1.20:11434/v1/chat/completions",
            "ftp://localhost/model",
            "https://user:password@api.openai.com/v1/chat/completions",
            "https://api.openai.com/v1/chat/completions?api_key=secret",
        ] {
            assert!(validate_endpoint(url).is_err(), "expected rejected: {url}");
        }
    }

    #[test]
    fn provider_origin_mismatch_rejects_key_reuse() {
        let secret = "sk-must-not-leak";
        let mut provider = builtin_providers()
            .into_iter()
            .find(|provider| provider.id == "openai")
            .unwrap();
        provider.enabled = true;
        provider.api_key = secret.to_string();
        provider.api_url = "https://api.deepseek.com/v1/chat/completions".to_string();

        let error = validate_provider_contract(&provider).unwrap_err();
        assert!(error.contains("来源不匹配"));
        assert!(!error.contains(secret));

        let error = provider_from_legacy_settings(
            "https://api.openai.com:444/v1/chat/completions",
            secret,
            "gpt-4o-mini",
        )
        .unwrap_err();
        assert!(error.contains("来源不匹配"));
        assert!(!error.contains(secret));
    }

    #[test]
    fn legacy_settings_support_known_openai_compatible_providers_only() {
        let deepseek = provider_from_legacy_settings(
            "https://api.deepseek.com/v1/chat/completions",
            "deepseek-secret",
            "deepseek-chat",
        )
        .unwrap();
        assert_eq!(deepseek.provider_type, ProviderType::DeepSeek);

        let error = provider_from_legacy_settings(
            "https://llm-proxy.example.com/v1/chat/completions",
            "legacy-secret",
            "some-model",
        )
        .unwrap_err();
        assert!(error.contains("无法与旧版 settings API Key 安全绑定"));
        assert!(!error.contains("legacy-secret"));

        let custom = provider_dtos_for_settings(
            true,
            "https://llm-proxy.example.com/v1/chat/completions",
            "legacy-secret",
            "some-model",
        );
        let custom = custom
            .iter()
            .find(|provider| provider.id == "custom")
            .unwrap();
        assert!(!custom.configured);
        assert!(!custom.enabled);
        assert!(custom.disabled_reason.is_some());
    }

    #[test]
    fn anthropic_is_explicitly_unsupported() {
        let providers = provider_dtos_for_settings(
            true,
            "https://api.anthropic.com/v1/messages",
            "anthropic-secret",
            "claude-3-5-haiku-20241022",
        );
        let claude = providers
            .iter()
            .find(|provider| provider.id == "claude")
            .unwrap();
        assert!(!claude.supported);
        assert!(!claude.configured);
        assert!(!claude.enabled);

        let error = provider_from_legacy_settings(
            "https://api.anthropic.com/v1/messages",
            "anthropic-secret",
            "claude-3-5-haiku-20241022",
        )
        .unwrap_err();
        assert!(error.contains("请求协议尚未实现"));
        assert!(!error.contains("anthropic-secret"));
    }

    #[tokio::test]
    async fn http_error_does_not_expose_body_or_forward_key_to_local_provider() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0_u8; 16 * 1024];
            let read = socket.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..read]).to_string();
            let body = "server-secret-response-body";
            let response = format!(
                "HTTP/1.1 401 Unauthorized\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            request
        });

        let endpoint = format!("http://{address}/v1/chat/completions");
        let provider = provider_from_legacy_settings(
            &endpoint,
            "stale-key-must-not-be-forwarded",
            "local-model",
        )
        .unwrap();
        let error = call_llm(&provider, "local-model", "system", "user", 0.1, 16)
            .await
            .unwrap_err();
        let request = server.await.unwrap();

        assert!(error.contains("HTTP 401"));
        assert!(!error.contains("server-secret-response-body"));
        assert!(!error.contains("stale-key-must-not-be-forwarded"));
        assert!(!request.to_ascii_lowercase().contains("authorization:"));
        assert!(!request.contains("stale-key-must-not-be-forwarded"));
    }
}
