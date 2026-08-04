//! 构建期规则清单生成（spec task-02 Step 2）。
//!
//! 遍历 `resources/rules/{anime,manga,novel}/*.json`，对每条规则做 schema 静态校验
//! （§3.1 必填字段缺失则 **build 失败**，防止残缺规则进包），计算每个文件 SHA-256，
//! 输出 `resources/rules/manifest.json`（覆盖写）。
//!
//! 该模块独立于 crate 之外，`build.rs` 用 `#[path = "src/rules/manifest_gen.rs"]` 方式包含编译，
//! 因此**禁止引用 crate 内部符号**（不得 `use crate::...`），依赖仅限 `serde_json` / `sha2` / `hex`
//! （`src-tauri/Cargo.toml` 的 `[build-dependencies]` 已具备）。

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// 构建期清单条目（与 specs §3.2 `RuleFileEntry` 对齐；`probeKeyword` 供健康检查使用）。
#[derive(Debug, Clone)]
pub struct RuleFileEntry {
    pub id: String,
    pub path: String,
    pub sha256: String,
    pub name: String,
    pub version: String,
    pub content_type: String,
    pub lang: String,
    pub nsfw: bool,
    pub probe_keyword: String,
}

/// 静态校验单条规则（spec §3.1 必填字段）。`rel_path` 仅用于错误信息。
pub fn validate_rule_value(v: &serde_json::Value, rel_path: &str) -> Result<(), String> {
    let mut missing: Vec<&str> = Vec::new();
    for field in [
        "name",
        "version",
        "contentType",
        "baseUrl",
        "language",
        "nsfw",
        "probeKeyword",
    ] {
        if v.get(field).is_none() {
            missing.push(field);
        }
    }
    for field in ["search", "detail", "chapter", "parse"] {
        match v.get(field).and_then(|x| x.as_str()) {
            Some(s) if s.contains("function") => {}
            Some(_) => missing.push(field),
            None => missing.push(field),
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "规则 {rel_path} 缺少或非法必需字段: {}",
            missing.join(", ")
        ));
    }
    let base_url = v
        .get("baseUrl")
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .trim();
    if !(base_url.starts_with("http://") || base_url.starts_with("https://")) {
        return Err(format!("规则 {rel_path} 的 baseUrl 必须是 http(s) 地址"));
    }
    Ok(())
}

/// 读取一条规则文件的 id：优先取文件内 `id` 字段（[a-z0-9-] slug），否则用文件名 stem。
fn rule_id(v: &serde_json::Value, file_name: &str) -> String {
    if let Some(id) = v.get("id").and_then(|x| x.as_str()) {
        if !id.is_empty() {
            return id.to_string();
        }
    }
    file_name
        .strip_suffix(".json")
        .map(str::to_string)
        .unwrap_or_else(|| file_name.to_string())
}

/// 生成规则清单并写入 `rules_root/manifest.json`。
///
/// - `rules_root`：`resources/rules` 目录；
/// - `package_version`：包版本（日期戳格式 `YYYY.M.N`，如 `"2026.08.1"`）；
/// - `published_at`：打包时间戳。
///
/// 任意规则缺失必填字段 → 返回 `Err`（build 失败）；返回写入的 manifest 路径。
pub fn generate_rules_manifest(
    rules_root: &Path,
    package_version: &str,
    published_at: i64,
) -> Result<PathBuf, String> {
    let mut entries: Vec<RuleFileEntry> = Vec::new();
    for subdir in ["anime", "manga", "novel"] {
        let dir = rules_root.join(subdir);
        if !dir.is_dir() {
            continue;
        }
        let mut file_names: Vec<String> = std::fs::read_dir(&dir)
            .map_err(|e| format!("读取规则目录 {} 失败: {e}", dir.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".json"))
            .collect();
        file_names.sort();
        for name in file_names {
            let path = dir.join(&name);
            let rel_path = format!("{subdir}/{name}");
            let bytes =
                std::fs::read(&path).map_err(|e| format!("读取规则文件 {rel_path} 失败: {e}"))?;
            let v: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|e| format!("规则文件 {rel_path} 不是合法 JSON: {e}"))?;
            validate_rule_value(&v, &rel_path)?;
            entries.push(RuleFileEntry {
                id: rule_id(&v, &name),
                path: rel_path,
                sha256: hex::encode(Sha256::digest(&bytes)),
                name: v
                    .get("name")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
                version: v
                    .get("version")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
                content_type: v
                    .get("contentType")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
                lang: v
                    .get("language")
                    .and_then(|x| x.as_str())
                    .unwrap_or("zh-CN")
                    .to_string(),
                nsfw: v.get("nsfw").and_then(|x| x.as_bool()).unwrap_or(false),
                probe_keyword: v
                    .get("probeKeyword")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default()
                    .to_string(),
            });
        }
    }
    if entries.is_empty() {
        return Err(format!(
            "{} 下未找到任何规则文件（需 anime/manga/novel 子目录各含 .json 规则）",
            rules_root.display()
        ));
    }

    let rules: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id,
                "path": e.path,
                "sha256": e.sha256,
                "name": e.name,
                "version": e.version,
                "contentType": e.content_type,
                "lang": e.lang,
                "nsfw": e.nsfw,
                "probeKeyword": e.probe_keyword,
            })
        })
        .collect();
    let manifest = serde_json::json!({
        "packageVersion": package_version,
        "publishedAt": published_at,
        "rules": rules,
    });
    let pretty = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    let out_path = rules_root.join("manifest.json");
    std::fs::write(&out_path, format!("{pretty}\n")).map_err(|e| e.to_string())?;
    Ok(out_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PACKAGE_VERSION: &str = "2026.08.1";
    const PUBLISHED_AT: i64 = 1785830400;

    /// 构造一条可用的规则 JSON 字符串（可覆盖字段以便制造缺失场景）。
    fn rule_json(overrides: &[(&str, serde_json::Value)]) -> serde_json::Value {
        let mut v = serde_json::json!({
            "name": "测试源",
            "id": "test-source",
            "version": "1.0.0",
            "contentType": "anime",
            "baseUrl": "https://example.com",
            "language": "zh-CN",
            "nsfw": false,
            "probeKeyword": "进击的巨人",
            "licenseNote": "Self-authored.",
            "search": "function search(kw, page) { return []; }",
            "detail": "function detail(url) { return { title: url }; }",
            "chapter": "function chapter(url) { return []; }",
            "parse": "function parse(url) { return { urls: [], kind: 'video' }; }",
        });
        for (k, val) in overrides {
            v[k] = val.clone();
        }
        v
    }

    /// 往临时目录写入一条规则文件，返回 (目录, 文件路径)。
    fn write_rule(dir: &Path, subdir: &str, name: &str, value: &serde_json::Value) {
        let sub = dir.join(subdir);
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join(name), serde_json::to_string_pretty(value).unwrap()).unwrap();
    }

    #[test]
    fn generate_manifest_rejects_missing_parse_field() {
        let tmp = tempfile::tempdir().unwrap();
        let bad = rule_json(&[("parse", serde_json::Value::Null)]);
        write_rule(tmp.path(), "anime", "bad.json", &bad);
        // 用「缺 parse」的规则构造：parse 字段为 null，validate_rule_value 应报错
        let v = rule_json(&[("parse", serde_json::json!(""))]);
        assert!(validate_rule_value(&v, "anime/bad.json").is_err());
        let err = validate_rule_value(&bad, "anime/bad.json");
        assert!(err.is_err(), "缺失 parse 应校验失败");
        assert!(generate_rules_manifest(tmp.path(), PACKAGE_VERSION, PUBLISHED_AT).is_err());
    }

    #[test]
    fn generate_manifest_counts_respect_minimums() {
        let tmp = tempfile::tempdir().unwrap();
        for (subdir, count) in [("anime", 6usize), ("manga", 4), ("novel", 3)] {
            for i in 0..count {
                let name = format!("{subdir}-{i}.json");
                let mut v = rule_json(&[]);
                v["name"] = serde_json::json!(format!("{subdir} {i}"));
                v["id"] = serde_json::json!(format!("{subdir}-{i}"));
                v["contentType"] = serde_json::json!(subdir);
                write_rule(tmp.path(), subdir, &name, &v);
            }
        }
        let out = generate_rules_manifest(tmp.path(), PACKAGE_VERSION, PUBLISHED_AT).unwrap();
        assert_eq!(out, tmp.path().join("manifest.json"));
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
        let rules = manifest["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 13, "规则总数 ≥13");
        let count = |t: &str| rules.iter().filter(|r| r["contentType"] == t).count();
        assert!(count("anime") >= 6, "anime ≥6");
        assert!(count("manga") >= 4, "manga ≥4");
        assert!(count("novel") >= 3, "novel ≥3");
        // 每个条目含 id/path/sha256
        for r in rules {
            assert!(!r["id"].as_str().unwrap().is_empty());
            assert!(r["path"].as_str().unwrap().ends_with(".json"));
            assert_eq!(r["sha256"].as_str().unwrap().len(), 64);
        }
    }

    #[test]
    fn real_rules_package_passes_validation() {
        // 回归防线：真实 resources/rules 集合能通过构建期校验，防止删源/残缺规则失守。
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("rules");
        if !root.is_dir() {
            // 非本仓库环境（如构建期单独编译）跳过
            return;
        }
        let out = generate_rules_manifest(&root, PACKAGE_VERSION, PUBLISHED_AT).unwrap();
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
        let rules = manifest["rules"].as_array().unwrap();
        assert!(rules.len() >= 13, "真实规则数 ≥13，实际 {}", rules.len());
        let count = |t: &str| rules.iter().filter(|r| r["contentType"] == t).count();
        assert!(count("anime") >= 6, "anime ≥6");
        assert!(count("manga") >= 4, "manga ≥4");
        assert!(count("novel") >= 3, "novel ≥3");
    }
}
