//! 元数据中文化与翻译
//!
//! 优先复用刮削结果里的中文标题/简介；缺失时可用应用 AI 设置调用
//! OpenAI 兼容 Chat Completions 接口翻译。中文结果不覆盖原文，调用方可以把
//! 标记块嵌入 review/notes 类字段。

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::models::ScrapeResult;

pub const MARKER_START: &str = "<!--moe:cn";
pub const SCRAPE_MARKER_START: &str = "<!--moe:scrape";
pub const MARKER_END: &str = "-->";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChineseMeta {
    pub name_cn: Option<String>,
    pub desc_cn: Option<String>,
}

impl ChineseMeta {
    pub fn has_any(&self) -> bool {
        self.name_cn
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
            || self
                .desc_cn
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationConfig {
    pub api_url: String,
    pub api_key: String,
    pub model: String,
    #[serde(default = "default_target_language")]
    pub target_language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScrapeMarker {
    pub scraped_at: Option<String>,
    pub source: Option<String>,
    pub metadata_hash: Option<String>,
    pub cover_image: bool,
    pub background_image: bool,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
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

fn default_target_language() -> String {
    "简体中文".to_string()
}

pub fn contains_cjk(value: &str) -> bool {
    value
        .chars()
        .any(|ch| ('\u{4E00}'..='\u{9FFF}').contains(&ch))
}

pub fn looks_chinese(value: &str) -> bool {
    contains_cjk(value)
        && !value.chars().any(|ch| {
            ('\u{3040}'..='\u{309F}').contains(&ch) || ('\u{30A0}'..='\u{30FF}').contains(&ch)
        })
}

pub async fn resolve_chinese_meta(
    result: &ScrapeResult,
    config: Option<&TranslationConfig>,
) -> ChineseMeta {
    let mut meta = ChineseMeta::default();

    if looks_chinese(&result.title) {
        meta.name_cn = Some(result.title.trim().to_string());
    } else if let Some(detail) = &result.detail {
        meta.name_cn = detail
            .aliases
            .iter()
            .find(|alias| looks_chinese(alias))
            .map(|alias| alias.trim().to_string());
    }

    if let Some(description) = result.description.as_deref() {
        if looks_chinese(description) {
            meta.desc_cn = Some(description.trim().to_string());
        }
    }

    if let Some(config) = config {
        if meta.name_cn.is_none() {
            if let Ok(translated) = translate_text(config, &result.title).await {
                if looks_chinese(&translated) && translated.trim() != result.title.trim() {
                    meta.name_cn = Some(translated);
                }
            }
        }

        if meta.desc_cn.is_none() {
            if let Some(description) = result.description.as_deref() {
                if let Ok(translated) = translate_text(config, description).await {
                    if looks_chinese(&translated) && translated.trim() != description.trim() {
                        meta.desc_cn = Some(translated);
                    }
                }
            }
        }
    }

    meta
}

pub async fn translate_text(config: &TranslationConfig, text: &str) -> Result<String, String> {
    if text.trim().is_empty() {
        return Ok(String::new());
    }
    if config.api_key.trim().is_empty() {
        return Err("未配置 AI API Key".to_string());
    }

    let request = ChatRequest {
        model: config.model.clone(),
        temperature: 0.1,
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: format!(
                    "Translate game metadata into {}. Preserve names, punctuation, and line breaks when appropriate. Return only the translated text.",
                    config.target_language
                ),
            },
            ChatMessage {
                role: "user".to_string(),
                content: text.to_string(),
            },
        ],
    };

    let client = Client::new();
    let response = client
        .post(&config.api_url)
        .bearer_auth(&config.api_key)
        .json(&request)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("翻译请求失败: HTTP {} {}", status, body));
    }

    let parsed: ChatResponse = response.json().await.map_err(|e| e.to_string())?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "翻译响应为空".to_string())
}

pub fn embed_chinese_marker(existing: Option<&str>, meta: &ChineseMeta) -> String {
    let body = strip_chinese_marker(existing.unwrap_or_default());
    if !meta.has_any() {
        return body;
    }

    let mut marker = String::new();
    marker.push_str(MARKER_START);
    marker.push('\n');
    if let Some(name) = meta.name_cn.as_deref().filter(|s| !s.trim().is_empty()) {
        marker.push_str("name=");
        marker.push_str(&encode_line(name));
        marker.push('\n');
    }
    if let Some(desc) = meta.desc_cn.as_deref().filter(|s| !s.trim().is_empty()) {
        marker.push_str("desc=");
        marker.push_str(&encode_line(desc));
        marker.push('\n');
    }
    marker.push_str(MARKER_END);

    if body.trim().is_empty() {
        marker
    } else {
        format!("{}\n{}", marker, body)
    }
}

pub fn parse_chinese_marker(text: &str) -> ChineseMeta {
    let Some(body) = marker_body(text, MARKER_START) else {
        return ChineseMeta::default();
    };

    let mut meta = ChineseMeta::default();
    for line in body.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("name=") {
            meta.name_cn = Some(decode_line(value));
        } else if let Some(value) = line.strip_prefix("desc=") {
            meta.desc_cn = Some(decode_line(value));
        }
    }
    meta
}

pub fn strip_markers(text: &str) -> String {
    strip_scrape_marker(&strip_chinese_marker(text))
        .trim()
        .to_string()
}

pub fn embed_scrape_marker(
    existing: Option<&str>,
    source: Option<&str>,
    metadata_hash: Option<&str>,
    cover_image: bool,
    background_image: bool,
) -> String {
    let body = strip_scrape_marker(existing.unwrap_or_default());
    let mut marker = String::new();
    marker.push_str(SCRAPE_MARKER_START);
    marker.push('\n');
    marker.push_str("scrapedAt=");
    marker.push_str(&chrono::Utc::now().to_rfc3339());
    marker.push('\n');
    if let Some(source) = source {
        marker.push_str("source=");
        marker.push_str(&encode_line(source));
        marker.push('\n');
    }
    if let Some(hash) = metadata_hash {
        marker.push_str("metadataHash=");
        marker.push_str(&encode_line(hash));
        marker.push('\n');
    }
    marker.push_str(if cover_image {
        "cover=1\n"
    } else {
        "cover=0\n"
    });
    marker.push_str(if background_image {
        "background=1\n"
    } else {
        "background=0\n"
    });
    marker.push_str(MARKER_END);

    if body.trim().is_empty() {
        marker
    } else {
        format!("{}\n{}", marker, body)
    }
}

pub fn parse_scrape_marker(text: &str) -> ScrapeMarker {
    let Some(body) = marker_body(text, SCRAPE_MARKER_START) else {
        return ScrapeMarker::default();
    };

    let mut marker = ScrapeMarker::default();
    for line in body.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("scrapedAt=") {
            marker.scraped_at = Some(decode_line(value));
        } else if let Some(value) = line.strip_prefix("source=") {
            marker.source = Some(decode_line(value));
        } else if let Some(value) = line.strip_prefix("metadataHash=") {
            marker.metadata_hash = Some(decode_line(value));
        } else if let Some(value) = line.strip_prefix("cover=") {
            marker.cover_image = value.trim() == "1";
        } else if let Some(value) = line.strip_prefix("background=") {
            marker.background_image = value.trim() == "1";
        }
    }
    marker
}

fn strip_chinese_marker(text: &str) -> String {
    strip_marker(text, MARKER_START)
}

fn strip_scrape_marker(text: &str) -> String {
    strip_marker(text, SCRAPE_MARKER_START)
}

fn strip_marker(text: &str, start: &str) -> String {
    let Some(start_idx) = text.find(start) else {
        return text.to_string();
    };
    let Some(end_rel) = text[start_idx..].find(MARKER_END) else {
        return text.to_string();
    };
    let end_idx = start_idx + end_rel + MARKER_END.len();
    format!("{}{}", &text[..start_idx], &text[end_idx..])
        .trim()
        .to_string()
}

fn marker_body<'a>(text: &'a str, start: &str) -> Option<&'a str> {
    let start_idx = text.find(start)?;
    let body_start = start_idx + start.len();
    let end_rel = text[body_start..].find(MARKER_END)?;
    Some(&text[body_start..body_start + end_rel])
}

fn encode_line(value: &str) -> String {
    value.replace('\r', "").replace('\n', "\\n")
}

fn decode_line(value: &str) -> String {
    value.replace("\\n", "\n").trim().to_string()
}
