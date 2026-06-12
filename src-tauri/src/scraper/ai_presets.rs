// 萌游 MoeGame · AI Provider 多预设系统（M3）
//
// 支持：
//   - 多 Provider：OpenAI / DeepSeek / Claude / Ollama / 自定义
//   - 多 Preset：中文描述增强 / 标签补全 / 翻译 / 背景词 / 分级推断
//   - 每个 preset 可指定 provider + model + 自定义 prompt
//   - 链式回退（provider A 失败→ provider B）

use serde::{Deserialize, Serialize};

// ============================================================================
// Provider 定义
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProvider {
    /// 唯一标识
    pub id: String,
    /// 显示名
    pub name: String,
    /// API URL（chat completions 端点）
    pub api_url: String,
    /// API Key
    pub api_key: String,
    /// 默认模型
    pub default_model: String,
    /// 是否启用
    pub enabled: bool,
    /// 超时（秒）
    pub timeout_secs: u64,
    /// Provider 类型（用于特殊处理）
    #[serde(default)]
    pub provider_type: ProviderType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
        },
    ]
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
/// 对 Anthropic 做特殊处理（beta header）。
pub async fn call_llm(
    provider: &AiProvider,
    model: &str,
    system_prompt: &str,
    user_message: &str,
    temperature: f64,
    max_tokens: u32,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(provider.timeout_secs))
        .build()
        .map_err(|e| e.to_string())?;

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
        .header("User-Agent", "MoeGame/0.1");

    // API Key 认证（OpenAI/Bearer 风格）
    if !provider.api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", provider.api_key));
    }
    // Anthropic 特殊 header
    if matches!(provider.provider_type, ProviderType::Anthropic) {
        req = req.header("anthropic-version", "2023-06-01");
        if !provider.api_key.is_empty() {
            req = req.header("x-api-key", &provider.api_key);
        }
    }

    let resp = req
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, text));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    // OpenAI-compatible 路径
    if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
        return Ok(content.to_string());
    }
    // Anthropic 路径
    if let Some(content) = json["content"][0]["text"].as_str() {
        return Ok(content.to_string());
    }

    Err(format!("无法解析响应: {}", json))
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
}
