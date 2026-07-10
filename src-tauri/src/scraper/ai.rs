//! AI 元数据增强层
//!
//! 使用 OpenAI 兼容 API（GPT / 本地 LLM）对游戏元数据进行智能增强：
//! - 生成更丰富的中文游戏描述
//! - 推断更准确的标签分类
//! - 生成背景图搜索关键词

use super::ai_presets::{self, AiProvider};
use super::error::ScrapeError;
use serde::{Deserialize, Serialize};

// ========== 公共类型 ==========

/// AI 生成的元数据增强结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiEnhancedMeta {
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub background: Option<String>,
}

/// AI 刮削配置。
///
/// Provider 与 API Key 保持在同一份已校验对象中，调用方不能再单独替换 endpoint
/// 后继续复用原 Key。
#[derive(Debug, Clone)]
pub struct AiScrapeConfig {
    provider: AiProvider,
}

impl AiScrapeConfig {
    pub fn from_legacy_settings(
        api_url: &str,
        api_key: &str,
        model: &str,
    ) -> Result<Self, ScrapeError> {
        let provider = ai_presets::provider_from_legacy_settings(api_url, api_key, model)
            .map_err(ScrapeError::Config)?;
        Ok(Self { provider })
    }

    #[cfg(test)]
    fn provider(&self) -> &AiProvider {
        &self.provider
    }
}

impl Default for AiScrapeConfig {
    fn default() -> Self {
        let provider = ai_presets::builtin_providers()
            .into_iter()
            .next()
            .expect("builtin OpenAI provider must exist");
        Self { provider }
    }
}

// ========== 公共 API ==========

/// 调用 LLM 对游戏元数据进行智能增强
///
/// 给定游戏名称和已有元数据，AI 会：
/// 1. 生成更丰富的中文描述
/// 2. 推断更准确的标签分类
/// 3. 生成背景图搜索关键词（用于后续壁纸抓取）
pub async fn enhance(
    config: &AiScrapeConfig,
    game_name: &str,
    existing_desc: Option<&str>,
    existing_tags: &[String],
) -> Result<AiEnhancedMeta, ScrapeError> {
    let tag_str = if existing_tags.is_empty() {
        "无".to_string()
    } else {
        existing_tags.join(", ")
    };

    let desc_hint = existing_desc.unwrap_or("无");

    let system_prompt = r#"你是一个游戏元数据专家。根据用户提供的游戏名称和已有信息，输出增强后的元数据。

严格按以下 JSON 格式输出，不要输出其他内容：
{
  "description": "200字以内的中文游戏介绍，突出玩法特色和故事背景",
  "tags": ["标签1", "标签2", "标签3", "标签4", "标签5"],
  "background": "适合该游戏氛围的壁纸搜索关键词（英文）"
}

规则：
- description 必须是中文，生动有趣，适合展示在游戏库中
- tags 优先使用已有的，补充缺失的类型/玩法标签，控制在 5-8 个
- background 用英文关键词，用于搜索高质量游戏壁纸/背景图"#;

    let user_prompt = format!(
        "游戏名称：{}\n已有描述：{}\n已有标签：{}",
        game_name, desc_hint, tag_str
    );

    let content = ai_presets::call_llm(
        &config.provider,
        &config.provider.default_model,
        system_prompt,
        &user_prompt,
        0.7,
        800,
    )
    .await
    .map_err(ScrapeError::Network)?;

    // 从可能包含 markdown 代码块的响应中提取 JSON
    let json_str = extract_json(&content);

    let enhanced: AiEnhancedMeta = serde_json::from_str(json_str)
        .map_err(|_| ScrapeError::Parse("AI 输出不是有效的元数据 JSON".to_string()))?;

    Ok(enhanced)
}

// ========== 内部工具 ==========

/// 从 AI 响应中提取 JSON（兼容 markdown 代码块包裹）
fn extract_json(content: &str) -> &str {
    let trimmed = content.trim();

    // 尝试找 ```json ... ``` 代码块
    if let Some(start) = trimmed.find("```json") {
        let json_start = start + 7;
        if let Some(end) = trimmed[json_start..].find("```") {
            return trimmed[json_start..json_start + end].trim();
        }
    }

    // 尝试找 ``` ... ``` 代码块
    if let Some(start) = trimmed.find("```") {
        let json_start = start + 3;
        let after_mark = &trimmed[json_start..];
        let actual_start = if let Some(newline) = after_mark.find('\n') {
            json_start + newline + 1
        } else {
            json_start
        };
        if let Some(end) = trimmed[actual_start..].find("```") {
            return trimmed[actual_start..actual_start + end].trim();
        }
    }

    // 直接尝试找 { ... }
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if end > start {
                return &trimmed[start..=end];
            }
        }
    }

    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scraper::ai_presets::ProviderType;

    #[test]
    fn scrape_config_accepts_local_provider_without_key() {
        let config = AiScrapeConfig::from_legacy_settings(
            "http://[::1]:11434/v1/chat/completions",
            "",
            "qwen2.5:7b",
        )
        .unwrap();

        assert_eq!(config.provider().provider_type, ProviderType::Ollama);
        assert!(config.provider().api_key().is_empty());
    }

    #[test]
    fn scrape_config_rejects_remote_plaintext_and_unknown_key_origin() {
        let plaintext = AiScrapeConfig::from_legacy_settings(
            "http://api.openai.com/v1/chat/completions",
            "sk-secret",
            "gpt-4o-mini",
        )
        .unwrap_err()
        .to_string();
        assert!(plaintext.contains("HTTPS"));
        assert!(!plaintext.contains("sk-secret"));

        let unknown = AiScrapeConfig::from_legacy_settings(
            "https://proxy.example.com/v1/chat/completions",
            "sk-secret",
            "gpt-4o-mini",
        )
        .unwrap_err()
        .to_string();
        assert!(unknown.contains("安全绑定"));
        assert!(!unknown.contains("sk-secret"));
    }

    #[test]
    fn json_parse_errors_do_not_include_full_model_output() {
        let model_output = "sensitive model output that is not json";
        let json = extract_json(model_output);
        let error = serde_json::from_str::<AiEnhancedMeta>(json)
            .map_err(|_| ScrapeError::Parse("AI 输出不是有效的元数据 JSON".to_string()))
            .unwrap_err()
            .to_string();

        assert!(!error.contains(model_output));
    }
}
