//! KazumiRules 规则库同步（KazumiRules 同步轨）。
//!
//! 从 kazumi 官方规则仓库（Predidit/KazumiRules）拉取规则目录与规则文件，
//! 以**原始 kazumi 格式**落盘到 `custom_rules/kazumi/`——加载期由
//! [`crate::rules::schema::normalize_kazumi`] 自动包装为标准四函数脚本，
//! 因此 kazumi 更新源后，用户一键/自动同步即可生效，无需改代码。
//!
//! 数据源（与 kazumi 客户端一致）：
//! - 主源: `https://raw.githubusercontent.com/Predidit/KazumiRules/main/`
//! - 镜像: `https://raw.gitcode.com/gh_mirrors/ka/KazumiRules/raw/main/`
//!   主源失败时自动降级镜像（国内网络友好）。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::rules::schema::{normalize_kazumi, RuleFileFormat, RuleManifest};

/// 规则目录条目（`index.json` 结构）
#[derive(Debug, Clone, Deserialize)]
pub struct KazumiCatalogItem {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default, rename = "lastUpdate")]
    pub last_update: Option<i64>,
}

/// 同步结果（前端展示用）
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KazumiSyncResult {
    /// 目录条目数（index.json）
    pub catalog_total: usize,
    /// 新增
    pub added: usize,
    /// 已更新（版本变化重写）
    pub updated: usize,
    /// 未变化跳过
    pub unchanged: usize,
    /// 拉取/校验失败
    pub failed: usize,
    /// 无效规则（格式/字段缺失）
    pub invalid: usize,
    pub failures: Vec<String>,
}

const KAZUMI_BASE: &str = "https://raw.githubusercontent.com/Predidit/KazumiRules/main/";
const KAZUMI_MIRROR: &str = "https://raw.gitcode.com/gh_mirrors/ka/KazumiRules/raw/main/";
const HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// 同步目标目录：`{config}/moeplay/custom_rules/kazumi/`
pub fn kazumi_rules_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("moeplay")
        .join("custom_rules")
        .join("kazumi");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        tracing::warn!("创建 kazumi 规则目录失败: {e}");
    }
    dir
}

async fn get_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let resp = client
        .get(url)
        .timeout(HTTP_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("GET {url}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("GET {url}: HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("读取响应: {e}"))
}

/// 拉取目录 + 同步规则到本地。`force` 为 true 时忽略版本对比全量重写。
pub async fn sync_from_kazumi(force: bool) -> KazumiSyncResult {
    let mut result = KazumiSyncResult::default();
    let client = reqwest::Client::builder()
        .user_agent(concat!(
            "MoeGame/",
            env!("CARGO_PKG_VERSION"),
            " (KazumiRules sync)"
        ))
        .build()
        .unwrap_or_default();

    // 主源失败 → 镜像
    let index = match get_text(&client, &format!("{KAZUMI_BASE}index.json")).await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("KazumiRules 主源不可用 ({e})，尝试镜像…");
            match get_text(&client, &format!("{KAZUMI_MIRROR}index.json")).await {
                Ok(t) => t,
                Err(e2) => {
                    result.failed = 1;
                    result
                        .failures
                        .push(format!("主源与镜像均不可用: {e} / {e2}"));
                    return result;
                }
            }
        }
    };

    let catalog: Vec<KazumiCatalogItem> = match serde_json::from_str(&index) {
        Ok(c) => c,
        Err(e) => {
            result.failed = 1;
            result.failures.push(format!("index.json 解析失败: {e}"));
            return result;
        }
    };
    result.catalog_total = catalog.len();

    let out_dir = kazumi_rules_dir();
    for item in &catalog {
        let name = item.name.trim().to_string();
        if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
            result.invalid += 1;
            continue;
        }
        // 用 index.json 声明的版本做增量判断；本地无记录或版本不同才拉取
        let local_path = out_dir.join(format!("{name}.json"));
        let need_fetch =
            force || !local_path.exists() || !local_version_matches(&local_path, &item.version);
        if !need_fetch {
            result.unchanged += 1;
            continue;
        }

        let raw = match get_text(&client, &format!("{KAZUMI_BASE}{name}.json")).await {
            Ok(t) => t,
            Err(_) => match get_text(&client, &format!("{KAZUMI_MIRROR}{name}.json")).await {
                Ok(t) => t,
                Err(e) => {
                    result.failed += 1;
                    result.failures.push(format!("{name}: {e}"));
                    continue;
                }
            },
        };

        // 落盘前验证：可解析 + kazumi 字段齐全（normalize 可成功）
        match validate_kazumi_json(&raw, &name) {
            Ok(()) => {
                let existed = local_path.exists();
                if let Err(e) = std::fs::write(&local_path, raw) {
                    result.failed += 1;
                    result.failures.push(format!("{name}: 写入失败 {e}"));
                    continue;
                }
                if existed {
                    result.updated += 1;
                } else {
                    result.added += 1;
                }
                tracing::info!(
                    "KazumiRules 同步: {name} {} (v{})",
                    if existed { "更新" } else { "新增" },
                    item.version
                );
            }
            Err(e) => {
                result.invalid += 1;
                result.failures.push(format!("{name}: {e}"));
            }
        }
    }

    result
}

fn local_version_matches(path: &Path, version: &str) -> bool {
    if version.trim().is_empty() {
        return false;
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| {
            v.get("version")
                .and_then(|x| x.as_str())
                .map(str::to_string)
        })
        .map(|v| v == version)
        .unwrap_or(false)
}

/// 验证规则 JSON：可反序列化、kazumi 字段齐全（normalize_kazumi 成功）。
fn validate_kazumi_json(raw: &str, name: &str) -> Result<(), String> {
    let mut m: RuleManifest = RuleManifest::from_str(raw, RuleFileFormat::Json)
        .map_err(|e| format!("解析失败: {}", e.message))?;
    if m.name.trim() != name {
        // 容忍大小写/空格差异，但不允许空名
        if m.name.trim().is_empty() {
            return Err("name 为空".to_string());
        }
    }
    normalize_kazumi(&mut m).map_err(|e| e.message)
}
