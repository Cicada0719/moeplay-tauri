//! AI 元数据增强层
//!
//! 使用 OpenAI 兼容 API（GPT / 本地 LLM）对游戏元数据进行智能增强：
//! - 生成更丰富的中文游戏描述
//! - 推断更准确的标签分类
//! - 生成背景图搜索关键词

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::Duration;

use super::error::ScrapeError;

/// HTTP 请求超时
const REQUEST_TIMEOUT_SECS: u64 = 30;

// ========== 公共类型 ==========

/// AI 生成的元数据增强结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiEnhancedMeta {
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub background: Option<String>,
}

/// AI 刮削配置
#[derive(Debug, Clone)]
pub struct AiScrapeConfig {
    pub api_url: String,
    pub api_key: String,
    pub model: String,
}

impl Default for AiScrapeConfig {
    fn default() -> Self {
        Self {
            api_url: "https://api.openai.com/v1/chat/completions".to_string(),
            api_key: String::new(),
            model: "gpt-4o-mini".to_string(),
        }
    }
}

// ========== OpenAI 兼容请求/响应 ==========

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
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
    if config.api_key.is_empty() {
        return Err(ScrapeError::Config("AI API Key 未配置".into()));
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| ScrapeError::Network(e.to_string()))?;

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

    let req = ChatRequest {
        model: config.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_prompt,
            },
        ],
        max_tokens: 800,
        temperature: 0.7,
    };

    let resp = client
        .post(&config.api_url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&req)
        .send()
        .await
        .map_err(|e| ScrapeError::Network(e.to_string()))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(ScrapeError::Api {
            status: status.as_u16(),
            body,
        });
    }

    let chat_resp: ChatResponse = resp
        .json()
        .await
        .map_err(|e| ScrapeError::Parse(e.to_string()))?;

    let content = chat_resp
        .choices
        .first()
        .ok_or_else(|| ScrapeError::Parse("AI 未返回结果".into()))?
        .message
        .content
        .clone();

    // 从可能包含 markdown 代码块的响应中提取 JSON
    let json_str = extract_json(&content);

    let enhanced: AiEnhancedMeta = serde_json::from_str(json_str)
        .map_err(|e| ScrapeError::Parse(format!("AI 输出解析失败: {} | 原文: {}", e, content)))?;

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
