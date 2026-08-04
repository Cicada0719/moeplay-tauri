//! 规则清单 schema：加载、校验与序列化（kazumi 兼容）。
//!
//! 数据结构对应 PRD §3.1 / FR-03 的内置源字段约定，对外暴露
//! camelCase JSON（前端直接消费）。kazumi 风格的字段别名通过
//! `#[serde(alias = "...")]` 兼容，不改变对外输出的字段名。

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

/// 规则清单（kazumi 兼容，JSON/YAML 反序列化目标）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuleManifest {
    /// 规则名，唯一标识之一
    pub name: String,
    /// 语义化版本，如 "1.2.0"
    pub version: String,
    /// 内容类型：anime | manga | novel
    #[serde(alias = "type")]
    pub content_type: ContentType,
    /// 站点根地址
    pub base_url: String,
    /// 语言，默认 "zh-CN"
    #[serde(default)]
    pub language: String,
    /// NSFW 标记，默认 false
    #[serde(default)]
    pub nsfw: bool,
    #[serde(default)]
    pub author: Option<String>,
    /// 生命周期函数 JS 源码（kazumi 风格：函数体字符串）
    pub search: String,
    pub detail: String,
    pub chapter: String,
    pub parse: String,
}

/// 内容类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Anime,
    Manga,
    Novel,
}

/// 加载后的规则实体（含运行时状态）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedRule {
    /// 稳定 id：文件规则取文件名 stem、清单规则取内容 SHA-256（见 [`file_stem_id`]
    /// / [`stable_rule_id`]），非随机生成——重启后删除链路
    /// （`rules_remove_custom` → `custom_rules/{id}.json`）不断裂。
    pub id: String,
    pub manifest: RuleManifest,
    pub origin: RuleOrigin,
    pub status: RuleStatus,
    /// status=Invalid 时填充
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RuleLoadError>,
}

impl LoadedRule {
    /// 构造一条无效规则（用于加载/校验失败）
    pub fn invalid(id: String, manifest: RuleManifest, origin: RuleOrigin, error: RuleLoadError) -> Self {
        Self {
            id,
            manifest,
            origin,
            status: RuleStatus::Invalid,
            error: Some(error),
        }
    }
}

/// 规则来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleOrigin {
    Builtin,
    Custom,
}

/// 规则加载状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleStatus {
    Ready,
    Invalid,
    Loading,
}

/// 规则加载错误（结构化，供前端展示字段名/行号）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleLoadError {
    pub message: String,
    /// QuickJS 解析出的错误行号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// "schema" | "compile" | "timeout"
    pub phase: String,
}

impl RuleLoadError {
    pub fn schema(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            phase: "schema".into(),
        }
    }

    pub fn compile(message: impl Into<String>, line: Option<u32>) -> Self {
        Self {
            message: message.into(),
            line,
            phase: "compile".into(),
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            phase: "timeout".into(),
        }
    }
}

/// 规则文件格式（按扩展名判定）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleFileFormat {
    Json,
    Yaml,
}

impl RuleFileFormat {
    /// 根据文件扩展名推断格式；不支持则返回 `None`。
    pub fn from_path(path: &Path) -> Option<Self> {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref()
        {
            Some("json") => Some(RuleFileFormat::Json),
            Some("yaml") | Some("yml") => Some(RuleFileFormat::Yaml),
            _ => None,
        }
    }
}

impl RuleManifest {
    /// 从 JSON/YAML 文本解析规则清单。
    pub fn from_str(s: &str, format: RuleFileFormat) -> Result<Self, RuleLoadError> {
        match format {
            RuleFileFormat::Json => serde_json::from_str::<RuleManifest>(s)
                .map_err(|e| RuleLoadError::schema(format!("规则文件解析失败: {e}"))),
            RuleFileFormat::Yaml => serde_yaml::from_str::<RuleManifest>(s)
                .map_err(|e| RuleLoadError::schema(format!("规则文件解析失败: {e}"))),
        }
    }
}

/// 计算规则的稳定 id：对 manifest 的规范化 JSON 序列化取 SHA-256（hex）。
///
/// 用于无文件名可用的 [`RuleInput::Manifest`](crate::rules::engine::RuleInput::Manifest)
/// 输入（程序化/测试加载）：同一 manifest 值无论加载多少次都得到同一 id，
/// 重复加载天然 upsert（Kimi K3 复审第 2 项）。
///
/// 用「规范化序列化」而非原始文件字节：字段顺序固定（struct 声明序），同一
/// manifest 值恒等序列化，保证 id 与文件格式（JSON/YAML）、缩进无关。
pub fn stable_rule_id(manifest: &RuleManifest) -> String {
    let canonical = serde_json::to_vec(manifest).expect("RuleManifest 序列化不应失败");
    hex::encode(Sha256::digest(&canonical))
}

/// 从规则文件路径取稳定 id：文件 stem（不含扩展名）。
///
/// 自定义规则的 id 采用文件名 stem（Kimi K3 复审第 1 项，`rules_import` 落盘为
/// `custom_rules/{stem}.json`，`rules_load_all` 以同一文件重载时 id 恒等）：
/// - 幂等：无论加载多少次、重启多少次，id 都与磁盘文件名一一对应；
/// - 删除链路不断裂：`rules_remove_custom` 按 `{rule_id}.json` 定位文件删除，
///   即使规则文件被就地编辑（内容变了、内容 hash 变了），stem 仍不变，
///   文件仍能被定位删除——若 id 每次随机生成或随内容变化，重启后 id 与文件名
///   对不上，`file.exists()` 静默跳过，文件残留，规则下轮启动「复活」。
pub fn file_stem_id(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| "rule".to_string())
}

/// Schema 校验：必需字段非空、baseUrl 为合法 http(s) URL、脚本含 function 关键字。
pub fn validate_manifest(m: &RuleManifest) -> Result<(), RuleLoadError> {
    let mut missing: Vec<&str> = Vec::new();
    if m.name.trim().is_empty() {
        missing.push("name");
    }
    if m.version.trim().is_empty() {
        missing.push("version");
    }
    if m.base_url.trim().is_empty() {
        missing.push("baseUrl");
    }
    if m.search.trim().is_empty() {
        missing.push("search");
    }
    if m.detail.trim().is_empty() {
        missing.push("detail");
    }
    if m.chapter.trim().is_empty() {
        missing.push("chapter");
    }
    if m.parse.trim().is_empty() {
        missing.push("parse");
    }
    if !missing.is_empty() {
        return Err(RuleLoadError::schema(format!(
            "规则缺少必需字段: {}",
            missing.join(", ")
        )));
    }

    // baseUrl 必须是合法 http(s) URL
    let parsed = url::Url::parse(m.base_url.trim())
        .map_err(|e| RuleLoadError::schema(format!("baseUrl 不是合法 URL: {e}")))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(RuleLoadError::schema("baseUrl 必须是 http(s) 地址"));
    }

    // 四个脚本字段必须包含 function 关键字（kazumi 风格函数体）
    for (field, script) in [
        ("search", m.search.as_str()),
        ("detail", m.detail.as_str()),
        ("chapter", m.chapter.as_str()),
        ("parse", m.parse.as_str()),
    ] {
        if !script.contains("function") {
            return Err(RuleLoadError::schema(format!(
                "脚本字段 {field} 缺少 function 关键字"
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_JSON: &str = r#"{
        "name": "测试源",
        "version": "1.0.0",
        "contentType": "anime",
        "baseUrl": "https://example.com",
        "search": "function search(keyword, page) { return []; }",
        "detail": "function detail(url) { return {}; }",
        "chapter": "function chapter(detailUrl) { return []; }",
        "parse": "function parse(chapterUrl) { return { urls: [], kind: 'video' }; }"
    }"#;

    const VALID_YAML: &str = r#"
name: 测试源
version: 1.0.0
contentType: anime
baseUrl: https://example.com
search: "function search(keyword, page) { return []; }"
detail: "function detail(url) { return {}; }"
chapter: "function chapter(detailUrl) { return []; }"
parse: "function parse(chapterUrl) { return { urls: [], kind: 'video' }; }"
"#;

    fn valid_manifest() -> RuleManifest {
        RuleManifest::from_str(VALID_JSON, RuleFileFormat::Json).unwrap()
    }

    #[test]
    fn schema_validate_ok() {
        assert!(validate_manifest(&valid_manifest()).is_ok());
    }

    #[test]
    fn schema_missing_field_rejected() {
        let mut m = valid_manifest();
        m.parse = String::new();
        let err = validate_manifest(&m).unwrap_err();
        assert!(err.message.contains("parse"), "message: {}", err.message);
        assert_eq!(err.phase, "schema");
    }

    #[test]
    fn schema_yaml_and_json_both_parse() {
        let json = RuleManifest::from_str(VALID_JSON, RuleFileFormat::Json).unwrap();
        let yaml = RuleManifest::from_str(VALID_YAML, RuleFileFormat::Yaml).unwrap();
        assert_eq!(json, yaml);
        assert_eq!(json.content_type, ContentType::Anime);
    }

    #[test]
    fn schema_rejects_bad_base_url() {
        let mut m = valid_manifest();
        m.base_url = "not-a-url".into();
        assert!(validate_manifest(&m).is_err());
    }

    #[test]
    fn schema_rejects_missing_function_keyword() {
        let mut m = valid_manifest();
        m.search = "return [];".into();
        let err = validate_manifest(&m).unwrap_err();
        assert!(err.message.contains("search"), "message: {}", err.message);
    }

    #[test]
    fn schema_accepts_kazumi_type_alias() {
        let kazumi = r#"{
            "name": "Kazumi 源",
            "version": "0.1.0",
            "type": "anime",
            "baseUrl": "https://example.com",
            "search": "function search(keyword, page) { return []; }",
            "detail": "function detail(url) { return {}; }",
            "chapter": "function chapter(detailUrl) { return []; }",
            "parse": "function parse(chapterUrl) { return { urls: [], kind: 'video' }; }"
        }"#;
        let m = RuleManifest::from_str(kazumi, RuleFileFormat::Json).unwrap();
        assert_eq!(m.content_type, ContentType::Anime);
        assert!(validate_manifest(&m).is_ok());
    }
}
