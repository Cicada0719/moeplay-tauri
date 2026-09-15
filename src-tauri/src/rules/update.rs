//! 规则包热更新（spec task-02 Step 3）：远端拉取 → ed25519 签名 + SHA-256 校验 →
//! 原子替换 → 失败回退本地缓存。
//!
//! 链路：`check_and_update` → `fetch_remote_package` → `verify_package` → `is_newer` →
//! `apply_update`（原子替换 `$APPDATA/rules-cache/current`）→ 更新 `rules-meta.json`。
//! 任何网络/签名/应用失败都**回退本地缓存**（`FallbackCached`），绝不让前端看到错误弹窗（FR-04）。

use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine as _;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};
use thiserror::Error;

use crate::rules::schema::{validate_manifest, RuleFileFormat as SourceRuleFileFormat};

/// 远端规则仓库根地址（可在设置中修改；本期使用默认值）。
pub const DEFAULT_REMOTE_BASE: &str =
    "https://raw.githubusercontent.com/Cicada0719/moeplay-tauri-rules/main/";

/// 构建期注入的 ed25519 公钥（base64，32 字节 raw）。
/// 对应私钥由维护者持有（不进入仓库），用于签署远端 `rules-package.json`。
/// 生成时间 2026-08-04；见内存记录 moeplay-rules-signing-key。
const EMBEDDED_PUBKEY_B64: &str = "DaQtB93xjf6MkwOUY9Hjtp2GpVGQ5lcaQdoEpMRrp94=";

/// FR-04：每 24h 自动检查一次更新。
const MIN_CHECK_INTERVAL_SECS: i64 = 24 * 3600;
const FETCH_TIMEOUT: Duration = Duration::from_secs(15);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30);
/// 单次更新并发下载规则文件的上限。
const MAX_CONCURRENT_DOWNLOADS: usize = 8;

/// 热更新错误（thiserror）。
#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("网络请求失败: {0}")]
    Network(String),
    #[error("远端包解析失败: {0}")]
    Parse(String),
    #[error("manifest SHA-256 校验不匹配")]
    ChecksumMismatch,
    #[error("ed25519 签名校验失败")]
    InvalidSignature,
    #[error("规则文件 SHA-256 校验不匹配: {0}")]
    RuleChecksumMismatch(String),
    #[error("写入缓存失败: {0}")]
    Io(String),
    #[error("应用数据目录不可用: {0}")]
    AppDataDir(String),
}

/// 规则包清单条目（spec §3.2 `RuleFileEntry`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleFileEntry {
    pub id: String,
    pub path: String,
    pub sha256: String,
    pub name: String,
    pub version: String,
    pub content_type: String,
    pub lang: String,
    pub nsfw: bool,
    /// 健康检查搜索关键词（构建期 manifest 会写入；远端包可能省略）。
    #[serde(default)]
    pub probe_keyword: Option<String>,
}

/// 规则包清单（本地 manifest / 远端包共用结构，spec §3.2）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleManifest {
    /// 日期戳版本 `YYYY.M.N`，如 `2026.08.1`（semver 前导零非法，见 `is_newer` 注释）。
    pub package_version: String,
    pub published_at: i64,
    pub rules: Vec<RuleFileEntry>,
}

/// 远端规则包（spec §3.2 `rules-package.json`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePackage {
    pub manifest: RuleManifest,
    /// canonical manifest JSON 字节的 SHA-256（hex）。
    pub manifest_sha256: String,
    /// 对 `manifest_sha256` 字节的 ed25519 签名（base64）。
    pub signature: String,
}

/// 规则来源（设置页展示）。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RuleSource {
    Bundled,
    RemoteCache,
}

/// 规则包元信息（`rules_get_meta` 返回）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesMetaInfo {
    pub package_version: String,
    pub source: RuleSource,
    pub updated_at: i64,
    pub last_check_at: Option<i64>,
    pub rule_count: usize,
    pub remote_base: String,
}

/// 更新结果（`rules_check_and_update` 返回）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOutcome {
    /// `#[serde(flatten)]`：`UpdateStatus` 的 tag 字段直接铺平到本结构顶层。
    #[serde(flatten)]
    pub status: UpdateStatus,
    pub from_version: Option<String>,
    pub to_version: String,
    pub updated_rules: usize,
}

/// 更新状态（内部标签枚举 → 前端 `{ status: 'updated' | 'alreadyLatest' | 'fallbackCached', reason? }`）。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum UpdateStatus {
    Updated,
    AlreadyLatest,
    /// 更新失败，已继续使用本地缓存（用户无感知，符合 FR-04）。
    FallbackCached {
        reason: String,
    },
}

impl UpdateOutcome {
    fn fallback(to_version: String, reason: String) -> Self {
        Self {
            status: UpdateStatus::FallbackCached { reason },
            from_version: None,
            to_version,
            updated_rules: 0,
        }
    }
}

/// `$APPDATA/rules-meta.json`（节流 + 设置页展示）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesMetaFile {
    #[serde(default)]
    pub package_version: String,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default)]
    pub last_check_at: Option<i64>,
}

// ── 远端拉取 / 校验 ───────────────────────────────────────────────────────

/// 拼接远端 URL：容忍 base 带/不带尾部斜杠（远端地址可在设置中修改，spec §2.3）。
fn join_remote_url(base: &str, rel: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        rel.trim_start_matches('/')
    )
}

/// 拉取远端规则包：GET `{base}/rules-package.json`，15s 超时。
///
/// 证书校验不可关闭（PRD §4.2）：本函数使用调用方传入的 `reqwest::Client`，
/// 该 client 一律由 `build_client()` 构造（默认 rustls，绝不含 `danger_accept_invalid_certs`）。
pub async fn fetch_remote_package(
    client: &reqwest::Client,
    base: &str,
) -> Result<RemotePackage, UpdateError> {
    let url = join_remote_url(base, "rules-package.json");
    let resp = client
        .get(&url)
        .timeout(FETCH_TIMEOUT)
        .send()
        .await
        .map_err(|e| UpdateError::Network(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(UpdateError::Network(format!(
            "GET {url} -> {}",
            resp.status()
        )));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| UpdateError::Network(e.to_string()))?;
    serde_json::from_slice(&bytes).map_err(|e| UpdateError::Parse(e.to_string()))
}

/// 用内置公钥校验包（checksum + 签名）。
pub fn verify_package(pkg: &RemotePackage) -> Result<(), UpdateError> {
    let key = embedded_verifying_key()?;
    verify_signature(pkg, &key)
}

/// 用指定公钥校验（测试可注入测试密钥，防止「验证函数恒真」的假实现）。
fn verify_signature(pkg: &RemotePackage, key: &VerifyingKey) -> Result<(), UpdateError> {
    // canonical JSON：紧凑序列化（无空白），字段顺序由 struct 声明固定。
    // 外部签名方必须复现同一 canonical 形式（spec §3.2）。
    let canonical =
        serde_json::to_vec(&pkg.manifest).map_err(|e| UpdateError::Parse(e.to_string()))?;
    let sha = hex::encode(Sha256::digest(&canonical));
    if sha != pkg.manifest_sha256 {
        return Err(UpdateError::ChecksumMismatch);
    }
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(&pkg.signature)
        .map_err(|_| UpdateError::InvalidSignature)?;
    let signature = Signature::from_slice(&sig_bytes).map_err(|_| UpdateError::InvalidSignature)?;
    // 签名对象是 manifest_sha256 的原始字节（32 字节）。
    let msg = hex::decode(&pkg.manifest_sha256).map_err(|_| UpdateError::InvalidSignature)?;
    key.verify(&msg, &signature)
        .map_err(|_| UpdateError::InvalidSignature)
}

fn embedded_verifying_key() -> Result<VerifyingKey, UpdateError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(EMBEDDED_PUBKEY_B64)
        .map_err(|_| UpdateError::InvalidSignature)?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| UpdateError::InvalidSignature)?;
    VerifyingKey::from_bytes(&arr).map_err(|_| UpdateError::InvalidSignature)
}

/// 版本比较。
///
/// 包版本固定采用日期戳格式 `YYYY.M.N`（示例 `2026.08.1`，semver 因前导零拒绝），
/// 同时兼容标准 semver（如 `1.2.0`）：先试 `semver::Version`，失败回退数值点号比较。
pub fn is_newer(remote: &str, local: &str) -> bool {
    match (
        semver::Version::parse(remote),
        semver::Version::parse(local),
    ) {
        (Ok(r), Ok(l)) => r > l,
        _ => compare_dot_versions(remote, local) > 0,
    }
}

fn compare_dot_versions(a: &str, b: &str) -> i32 {
    let pa = numeric_parts(a);
    let pb = numeric_parts(b);
    for i in 0..pa.len().max(pb.len()) {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        if x != y {
            return if x > y { 1 } else { -1 };
        }
    }
    0
}

/// 取版本串的点号数值分量；忽略非数字后缀（如 `2026.08.1` 的 `.1`）。
fn numeric_parts(v: &str) -> Vec<u64> {
    v.split('.')
        .map(|p| {
            p.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
        })
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().unwrap_or(0))
        .collect()
}

// ── 应用更新（原子替换） ───────────────────────────────────────────────────

/// 应用更新：下载全量到 `.tmp-<ts>` → 原子 `rename` 到 `current`（先下载全量再切换，
/// 部分失败不动现有缓存）。
pub async fn apply_update(app: &AppHandle, pkg: &RemotePackage) -> Result<usize, UpdateError> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| UpdateError::AppDataDir(e.to_string()))?;
    let cache_dir = app_data.join("rules-cache");
    let client = build_client()?;
    let n = apply_update_to_dir(&client, DEFAULT_REMOTE_BASE, pkg, &cache_dir).await?;
    write_rules_meta(&cache_dir, pkg)?;
    Ok(n)
}

/// 核心下载 + 原子切换（可注入 client / base / 缓存目录，供测试使用）。
pub async fn apply_update_to_dir(
    client: &reqwest::Client,
    base: &str,
    pkg: &RemotePackage,
    cache_dir: &Path,
) -> Result<usize, UpdateError> {
    validate_remote_manifest(&pkg.manifest)?;
    std::fs::create_dir_all(cache_dir).map_err(|e| UpdateError::Io(e.to_string()))?;
    let ts = now_ts();
    let tmp_dir = cache_dir.join(format!(".tmp-{ts}"));
    if tmp_dir.exists() {
        std::fs::remove_dir_all(&tmp_dir).map_err(|e| UpdateError::Io(e.to_string()))?;
    }
    std::fs::create_dir_all(&tmp_dir).map_err(|e| UpdateError::Io(e.to_string()))?;

    let permits = std::sync::Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_DOWNLOADS));
    let futs = pkg.manifest.rules.iter().map(|entry| {
        let client = client.clone();
        let base = base.to_string();
        let path = entry.path.clone();
        let sha = entry.sha256.clone();
        let tmp = tmp_dir.clone();
        let permits = permits.clone();
        async move {
            let _permit = permits
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| UpdateError::Io("下载并发门闩已关闭".into()))?;
            download_rule_file(&client, &base, &path, &sha, &tmp).await
        }
    });
    let result = futures_util::future::try_join_all(futs).await;
    if let Err(e) = result {
        // 部分失败 → 清理 .tmp，现有缓存不动（原子性）。
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(e);
    }

    // 下载完成后按规则 schema 校验每个文件；在校验完成前绝不触碰 current。
    if let Err(e) = validate_downloaded_rules(pkg, &tmp_dir) {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return Err(e);
    }

    // manifest 与规则文件一起进入暂存目录，避免切换后出现没有清单的 current。
    let manifest_json = serde_json::to_string_pretty(&pkg.manifest)
        .map_err(|e| UpdateError::Parse(e.to_string()))?;
    std::fs::write(tmp_dir.join("manifest.json"), &manifest_json)
        .map_err(|e| UpdateError::Io(e.to_string()))?;

    // 全部下载成功 → 原子切换：current → .old-<ts>，tmp → current。
    let current_dir = cache_dir.join("current");
    let old_dir = cache_dir.join(format!(".old-{ts}"));
    if current_dir.exists() {
        if old_dir.exists() {
            std::fs::remove_dir_all(&old_dir).map_err(|e| UpdateError::Io(e.to_string()))?;
        }
        std::fs::rename(&current_dir, &old_dir).map_err(|e| UpdateError::Io(e.to_string()))?;
    }
    if let Err(e) = std::fs::rename(&tmp_dir, &current_dir) {
        // 切换失败 → 尽力回滚 old → current。
        if old_dir.exists() && !current_dir.exists() {
            let _ = std::fs::rename(&old_dir, &current_dir);
        }
        return Err(UpdateError::Io(e.to_string()));
    }
    if old_dir.exists() {
        std::fs::remove_dir_all(&old_dir).map_err(|e| UpdateError::Io(e.to_string()))?;
    }

    // 根目录清单是兼容旧版本的索引；current/manifest.json 已随暂存目录原子切换。
    // 索引写失败不影响刚刚验证通过的 active 规则。
    let _ = std::fs::write(cache_dir.join("manifest.json"), manifest_json);

    Ok(pkg.manifest.rules.len())
}

fn validate_remote_manifest(manifest: &RuleManifest) -> Result<(), UpdateError> {
    if manifest.package_version.trim().is_empty() {
        return Err(UpdateError::Parse("规则包缺少 packageVersion".into()));
    }
    if manifest.rules.is_empty() {
        return Err(UpdateError::Parse("规则包未包含任何规则".into()));
    }
    let mut ids = std::collections::HashSet::new();
    let mut paths = std::collections::HashSet::new();
    for entry in &manifest.rules {
        if entry.id.trim().is_empty() || !ids.insert(entry.id.clone()) {
            return Err(UpdateError::Parse(format!(
                "规则 id 重复或为空: {}",
                entry.id
            )));
        }
        if entry.name.trim().is_empty() || entry.version.trim().is_empty() {
            return Err(UpdateError::Parse(format!(
                "规则元数据不完整: {}",
                entry.id
            )));
        }
        if !paths.insert(entry.path.clone()) || !is_safe_rule_path(&entry.path) {
            return Err(UpdateError::Parse(format!(
                "规则路径非法或重复: {}",
                entry.path
            )));
        }
        if SourceRuleFileFormat::from_path(Path::new(&entry.path)).is_none() {
            return Err(UpdateError::Parse(format!(
                "规则格式不支持: {}",
                entry.path
            )));
        }
        if entry.sha256.len() != 64 || !entry.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(UpdateError::Parse(format!(
                "规则 SHA-256 非法: {}",
                entry.id
            )));
        }
    }
    Ok(())
}

fn is_safe_rule_path(path: &str) -> bool {
    let p = Path::new(path);
    !p.is_absolute()
        && p.components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

fn validate_downloaded_rules(pkg: &RemotePackage, tmp_dir: &Path) -> Result<(), UpdateError> {
    for entry in &pkg.manifest.rules {
        let path = tmp_dir.join(&entry.path);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| UpdateError::Io(format!("读取规则 {} 失败: {e}", entry.id)))?;
        let format = SourceRuleFileFormat::from_path(Path::new(&entry.path))
            .ok_or_else(|| UpdateError::Parse(format!("规则格式不支持: {}", entry.path)))?;
        let source_manifest =
            crate::rules::schema::RuleManifest::from_str(&text, format).map_err(|e| {
                UpdateError::Parse(format!("规则 {} 结构无效: {}", entry.id, e.message))
            })?;
        validate_manifest(&source_manifest).map_err(|e| {
            UpdateError::Parse(format!("规则 {} 校验失败: {}", entry.id, e.message))
        })?;
    }
    Ok(())
}

async fn download_rule_file(
    client: &reqwest::Client,
    base: &str,
    path: &str,
    expected_sha: &str,
    tmp_dir: &Path,
) -> Result<(), UpdateError> {
    let url = join_remote_url(base, &format!("rules/{path}"));
    let resp = client
        .get(&url)
        .timeout(DOWNLOAD_TIMEOUT)
        .send()
        .await
        .map_err(|e| UpdateError::Network(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(UpdateError::Network(format!(
            "GET {url} -> {}",
            resp.status()
        )));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| UpdateError::Network(e.to_string()))?;
    let sha = hex::encode(Sha256::digest(&bytes));
    if sha != expected_sha {
        return Err(UpdateError::RuleChecksumMismatch(path.to_string()));
    }
    let dest = tmp_dir.join(path);
    if let Some(parent) = dest.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| UpdateError::Io(e.to_string()))?;
        }
    }
    std::fs::write(&dest, bytes).map_err(|e| UpdateError::Io(e.to_string()))?;
    Ok(())
}

/// 更新 `$APPDATA/rules-meta.json`。
fn write_rules_meta(cache_dir: &Path, pkg: &RemotePackage) -> Result<(), UpdateError> {
    let app_data = cache_dir
        .parent()
        .ok_or_else(|| UpdateError::Io("无法定位应用数据目录".into()))?;
    let meta = RulesMetaFile {
        package_version: pkg.manifest.package_version.clone(),
        updated_at: now_ts(),
        last_check_at: None,
    };
    let json =
        serde_json::to_string_pretty(&meta).map_err(|e| UpdateError::Parse(e.to_string()))?;
    std::fs::write(app_data.join("rules-meta.json"), json)
        .map_err(|e| UpdateError::Io(e.to_string()))?;
    Ok(())
}

// ── 检查并更新入口 ────────────────────────────────────────────────────────

/// `rules_check_and_update` 命令入口（AppHandle 版本）。
pub async fn check_and_update(app: &AppHandle, force: bool) -> Result<UpdateOutcome, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| UpdateError::AppDataDir(e.to_string()))
        .map_err(|e| e.to_string())?;
    let client = build_client().map_err(|e| e.to_string())?;
    check_and_update_with(&client, DEFAULT_REMOTE_BASE, &app_data, force)
        .await
        .map_err(|e| e.to_string())
}

/// 可注入 client / base / app_data 的检查链路（供 wiremock 测试）。
///
/// 语义（spec Step 3.6）：24h 节流 → 拉取 → 校验 → 版本比较 → 应用；
/// 网络/签名/应用失败一律 `FallbackCached`（不返回 Err 给前端）。
pub async fn check_and_update_with(
    client: &reqwest::Client,
    base: &str,
    app_data: &Path,
    force: bool,
) -> Result<UpdateOutcome, UpdateError> {
    let key = embedded_verifying_key()?;
    check_and_update_with_key(client, base, app_data, force, &key).await
}

/// 校验密钥可注入的检查链路核心（测试注入测试密钥，私钥不进入仓库，spec §8）。
/// 生产路径经 [`check_and_update_with`] 传入内嵌公钥；其余逻辑完全一致。
async fn check_and_update_with_key(
    client: &reqwest::Client,
    base: &str,
    app_data: &Path,
    force: bool,
    key: &VerifyingKey,
) -> Result<UpdateOutcome, UpdateError> {
    let meta = load_rules_meta(app_data);
    let now = now_ts();

    // FR-04 每 24h 节流（force=true 跳过）。
    if !force {
        if let Some(last_check) = meta.last_check_at {
            if now.saturating_sub(last_check) < MIN_CHECK_INTERVAL_SECS {
                return Ok(UpdateOutcome {
                    status: UpdateStatus::AlreadyLatest,
                    from_version: None,
                    to_version: meta.package_version,
                    updated_rules: 0,
                });
            }
        }
    }

    let remote = match fetch_remote_package(client, base).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("规则包拉取失败，继续使用本地规则: {e}");
            let _ = save_last_check_at(app_data, now);
            return Ok(UpdateOutcome::fallback(
                meta.package_version,
                format!("远端拉取失败: {e}"),
            ));
        }
    };

    if let Err(e) = verify_signature(&remote, key) {
        tracing::warn!(
            "规则包校验失败（版本 {}），继续使用本地规则: {e}",
            remote.manifest.package_version
        );
        let _ = save_last_check_at(app_data, now);
        return Ok(UpdateOutcome::fallback(
            meta.package_version,
            format!("签名/校验失败: {e}"),
        ));
    }

    let local_version = if meta.package_version.is_empty() {
        "0.0.0".to_string()
    } else {
        meta.package_version.clone()
    };
    if !is_newer(&remote.manifest.package_version, &local_version) {
        let _ = save_last_check_at(app_data, now);
        return Ok(UpdateOutcome {
            status: UpdateStatus::AlreadyLatest,
            from_version: Some(local_version.clone()),
            to_version: local_version,
            updated_rules: 0,
        });
    }

    let cache_dir = app_data.join("rules-cache");
    let updated = match apply_update_to_dir(client, base, &remote, &cache_dir).await {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!("规则包应用失败，保留本地缓存: {e}");
            let _ = save_last_check_at(app_data, now);
            return Ok(UpdateOutcome::fallback(
                local_version,
                format!("应用失败: {e}"),
            ));
        }
    };
    if let Err(e) = write_rules_meta(&cache_dir, &remote) {
        // 规则已应用成功；meta 写入失败仅影响节流/展示，不把已成功的更新判为失败。
        tracing::warn!("写入 rules-meta.json 失败: {e}");
    }
    let _ = save_last_check_at(app_data, now);
    tracing::info!(
        "规则包更新成功: {local_version} -> {}（{updated} 条）",
        remote.manifest.package_version
    );
    Ok(UpdateOutcome {
        status: UpdateStatus::Updated,
        from_version: Some(local_version),
        to_version: remote.manifest.package_version,
        updated_rules: updated,
    })
}

/// 规则目录解析（与子任务 1 的唯一集成点）：
/// 若 `$APPDATA/rules-cache/current/manifest.json` 存在且可解析 → 缓存目录；
/// 否则回退打包资源目录 `resources/rules/`。
pub fn resolve_rules_dir(app: &AppHandle) -> PathBuf {
    resolve_rules_dir_from_paths(
        app.path().app_data_dir().ok().as_deref(),
        &bundled_rules_dir(app),
    )
}

/// 测试可注入版本：`app_data=None`（无缓存）或损坏缓存 → 回退 `bundled`。
pub fn resolve_rules_dir_from_paths(app_data: Option<&Path>, bundled: &Path) -> PathBuf {
    if let Some(ad) = app_data {
        let cache = ad.join("rules-cache").join("current");
        let manifest = cache.join("manifest.json");
        if manifest.is_file() {
            if let Ok(text) = std::fs::read_to_string(&manifest) {
                if serde_json::from_str::<RuleManifest>(&text).is_ok() {
                    return cache;
                }
            }
            tracing::warn!("规则缓存 manifest.json 无法解析，回退内置规则目录");
        }
    }
    bundled.to_path_buf()
}

/// Support both portable rules/ and Tauri's preserved resources/rules/ layout.
pub fn bundled_rules_dir(app: &AppHandle) -> PathBuf {
    if let Ok(dir) = app.path().resource_dir() {
        if let Some(candidate) = packaged_rules_dir(&dir) {
            return candidate;
        }
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("rules");
    if dev.is_dir() {
        return dev;
    }
    dev
}

fn packaged_rules_dir(resource_dir: &Path) -> Option<PathBuf> {
    ["rules", "resources/rules"]
        .into_iter()
        .map(|relative| resource_dir.join(relative))
        .find(|candidate| candidate.join("manifest.json").is_file())
}

/// `rules_get_meta` 命令实现：读取当前生效规则清单 + `rules-meta.json`。
pub fn rules_meta_info(app: &AppHandle) -> Result<RulesMetaInfo, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| UpdateError::AppDataDir(e.to_string()))
        .map_err(|e| e.to_string())?;
    let meta = load_rules_meta(&app_data);
    let active_dir = resolve_rules_dir(app);
    let manifest_path = active_dir.join("manifest.json");
    let mut package_version = meta.package_version.clone();
    let mut updated_at = meta.updated_at;
    let mut rule_count = 0usize;
    let mut source = RuleSource::Bundled;
    if manifest_path.is_file() {
        if let Ok(text) = std::fs::read_to_string(&manifest_path) {
            if let Ok(m) = serde_json::from_str::<RuleManifest>(&text) {
                package_version = m.package_version;
                updated_at = m.published_at;
                rule_count = m.rules.len();
                source = if active_dir.to_string_lossy().contains("rules-cache") {
                    RuleSource::RemoteCache
                } else {
                    RuleSource::Bundled
                };
            }
        }
    }
    Ok(RulesMetaInfo {
        package_version,
        source,
        updated_at,
        last_check_at: meta.last_check_at,
        rule_count,
        remote_base: DEFAULT_REMOTE_BASE.to_string(),
    })
}

// ── 工具函数 ──────────────────────────────────────────────────────────────

fn build_client() -> Result<reqwest::Client, UpdateError> {
    reqwest::Client::builder()
        .user_agent(concat!("moeplay/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| UpdateError::Network(e.to_string()))
}

fn load_rules_meta(app_data: &Path) -> RulesMetaFile {
    let path = app_data.join("rules-meta.json");
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(m) = serde_json::from_str::<RulesMetaFile>(&text) {
            return m;
        }
    }
    RulesMetaFile::default()
}

fn save_last_check_at(app_data: &Path, ts: i64) -> Result<(), UpdateError> {
    let mut meta = load_rules_meta(app_data);
    meta.last_check_at = Some(ts);
    let json =
        serde_json::to_string_pretty(&meta).map_err(|e| UpdateError::Parse(e.to_string()))?;
    std::fs::write(app_data.join("rules-meta.json"), json)
        .map_err(|e| UpdateError::Io(e.to_string()))
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 构造带签名的远端包（测试与发布脚本共用）：canonical JSON → SHA-256 → 对哈希字节签名。
pub fn sign_package(manifest: &RuleManifest, key: &SigningKey) -> RemotePackage {
    let canonical = serde_json::to_vec(manifest).expect("manifest 序列化不应失败");
    let sha = hex::encode(Sha256::digest(&canonical));
    let msg = hex::decode(&sha).expect("hex 解码不应失败");
    let signature = key.sign(&msg);
    RemotePackage {
        manifest: manifest.clone(),
        manifest_sha256: sha,
        signature: base64::engine::general_purpose::STANDARD.encode(signature.to_bytes()),
    }
}

#[cfg(test)]
mod tests {
    //! 规则包热更新测试（spec §6.1 测试 1~9）。
    //!
    //! 私钥不进入仓库（spec §8）：测试统一用**固定字节派生的测试密钥**，经
    //! `check_and_update_with_key` 注入校验密钥走真实链路（拉取 → checksum →
    //! 签名 → 版本比较 → 原子应用），同时用「篡改签名/换错误 key」证明校验函数
    //! 不是「恒真」假实现（测试 9）。

    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn packaged_rules_resolve_both_installer_layouts_without_developer_directory() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(packaged_rules_dir(root.path()), None);
        let nested = root.path().join("resources/rules");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("manifest.json"), "{}").unwrap();
        assert_eq!(packaged_rules_dir(root.path()), Some(nested));
        let flat = root.path().join("rules");
        std::fs::create_dir_all(&flat).unwrap();
        std::fs::write(flat.join("manifest.json"), "{}").unwrap();
        assert_eq!(packaged_rules_dir(root.path()), Some(flat));
    }

    /// 固定字节派生的测试签名密钥（避免依赖 rand_core feature 与熵源）。
    fn test_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn http_client() -> reqwest::Client {
        reqwest::Client::builder().build().expect("client 构建失败")
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        hex::encode(Sha256::digest(bytes))
    }

    /// 构造一个带 `rules` 条目的测试 manifest（包版本固定 `2026.08.2`）。
    fn test_manifest(rules: Vec<(String, String, String)>) -> RuleManifest {
        RuleManifest {
            package_version: "2026.08.2".to_string(),
            published_at: now_ts(),
            rules: rules
                .into_iter()
                .map(|(id, path, sha)| RuleFileEntry {
                    id: id.clone(),
                    path,
                    sha256: sha,
                    name: id.clone(),
                    version: "1.0.0".to_string(),
                    content_type: "anime".to_string(),
                    lang: "zh-CN".to_string(),
                    nsfw: false,
                    probe_keyword: Some("进击的巨人".to_string()),
                })
                .collect(),
        }
    }

    /// 挂载远端包与规则文件。
    async fn mount_package(server: &MockServer, pkg: &RemotePackage, files: &[(&str, &[u8])]) {
        Mock::given(method("GET"))
            .and(path("/rules-package.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(pkg))
            .mount(server)
            .await;
        for (file_path, content) in files {
            Mock::given(method("GET"))
                .and(path(format!("/rules/{file_path}")))
                .respond_with(ResponseTemplate::new(200).set_body_bytes(content.to_vec()))
                .mount(server)
                .await;
        }
    }

    /// 预置一个带标记文件的 `current/` 缓存，用于验证「更新失败不动现有缓存」。
    fn seed_cache(app_data: &Path) -> std::path::PathBuf {
        let cache = app_data.join("rules-cache").join("current");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("marker.txt"), "keep").unwrap();
        cache
    }

    // ── 测试 1：正常路径 → Updated，缓存齐全、sha256 正确、meta 更新 ─────────

    #[tokio::test]
    async fn happy_path_applies_remote_package() {
        let server = MockServer::start().await;
        let key = test_key();
        let file_content: &[u8] = br#"{
          "name": "SourceA",
          "version": "1.0.0",
          "contentType": "anime",
          "baseUrl": "https://example.com",
          "language": "zh-CN",
          "nsfw": false,
          "search": "function search(keyword, page) { return [{title: keyword, url: 'https://example.com'}]; }",
          "detail": "function detail(url) { return {title: url}; }",
          "chapter": "function chapter(url) { return []; }",
          "parse": "function parse(url) { return {urls: [url], kind: 'video'}; }"
        }"#;
        let manifest = test_manifest(vec![(
            "source-a".into(),
            "anime/source-a.json".into(),
            sha256_hex(file_content),
        )]);
        let pkg = sign_package(&manifest, &key);
        mount_package(&server, &pkg, &[("anime/source-a.json", file_content)]).await;

        let app_data = tempfile::tempdir().unwrap();
        let outcome = check_and_update_with_key(
            &http_client(),
            &server.uri(),
            app_data.path(),
            true,
            &key.verifying_key(),
        )
        .await
        .expect("更新应成功");
        assert!(matches!(outcome.status, UpdateStatus::Updated));
        assert_eq!(outcome.updated_rules, 1);
        assert_eq!(outcome.to_version, "2026.08.2");

        let cache = app_data.path().join("rules-cache").join("current");
        assert!(cache.join("manifest.json").exists(), "应写入缓存 manifest");
        let on_disk = std::fs::read(cache.join("anime/source-a.json")).unwrap();
        assert_eq!(on_disk, file_content, "规则文件字节应与远端一致");
        assert_eq!(sha256_hex(&on_disk), manifest.rules[0].sha256);

        let meta = load_rules_meta(app_data.path());
        assert_eq!(meta.package_version, "2026.08.2");
        assert!(meta.last_check_at.is_some(), "应记录 lastCheckAt");
    }

    // ── 测试 2：版本相同 → AlreadyLatest，缓存不被触碰 ───────────────────────

    #[tokio::test]
    async fn same_version_is_already_latest_and_cache_untouched() {
        let server = MockServer::start().await;
        let key = test_key();
        let manifest = test_manifest(vec![]);
        let pkg = sign_package(&manifest, &key);
        mount_package(&server, &pkg, &[]).await;

        let app_data = tempfile::tempdir().unwrap();
        // 本地 meta 与远端同版本
        std::fs::write(
            app_data.path().join("rules-meta.json"),
            r#"{ "packageVersion": "2026.08.2", "updatedAt": 0 }"#,
        )
        .unwrap();
        let cache = seed_cache(app_data.path());

        let outcome = check_and_update_with_key(
            &http_client(),
            &server.uri(),
            app_data.path(),
            true,
            &key.verifying_key(),
        )
        .await
        .expect("应返回 AlreadyLatest 而非 Err");
        assert!(matches!(outcome.status, UpdateStatus::AlreadyLatest));
        assert!(
            cache.join("marker.txt").exists(),
            "版本未变时缓存不得被触碰"
        );
        assert!(
            !cache.join("manifest.json").exists(),
            "未应用更新，不应写入缓存 manifest"
        );
    }

    // ── 测试 3：签名被篡改 → FallbackCached，原缓存完好 ───────────────────────

    #[tokio::test]
    async fn tampered_signature_falls_back_and_keeps_cache() {
        let server = MockServer::start().await;
        let key = test_key();
        let manifest = test_manifest(vec![]);
        let mut pkg = sign_package(&manifest, &key);
        // 篡改签名：base64 解码后翻转末字节再编码
        let mut sig = base64::engine::general_purpose::STANDARD
            .decode(&pkg.signature)
            .unwrap();
        *sig.last_mut().unwrap() ^= 0x01;
        pkg.signature = base64::engine::general_purpose::STANDARD.encode(sig);
        mount_package(&server, &pkg, &[]).await;

        let app_data = tempfile::tempdir().unwrap();
        let cache = seed_cache(app_data.path());

        let outcome = check_and_update_with_key(
            &http_client(),
            &server.uri(),
            app_data.path(),
            true,
            &key.verifying_key(),
        )
        .await
        .expect("校验失败应回退而非 Err");
        let reason = match outcome.status {
            UpdateStatus::FallbackCached { reason } => reason,
            other => panic!("期望 FallbackCached，得到 {other:?}"),
        };
        assert!(reason.contains("签名"), "reason: {reason}");
        assert!(
            cache.join("marker.txt").exists(),
            "签名失败时原缓存必须完好"
        );
    }

    // ── 测试 4：manifestSha256 不匹配 → FallbackCached ────────────────────────

    #[tokio::test]
    async fn checksum_mismatch_falls_back() {
        let server = MockServer::start().await;
        let key = test_key();
        let manifest = test_manifest(vec![]);
        let mut pkg = sign_package(&manifest, &key);
        pkg.manifest_sha256 = hex::encode([0u8; 32]); // 与真实 canonical JSON 不匹配
        mount_package(&server, &pkg, &[]).await;

        let app_data = tempfile::tempdir().unwrap();
        let outcome = check_and_update_with_key(
            &http_client(),
            &server.uri(),
            app_data.path(),
            true,
            &key.verifying_key(),
        )
        .await
        .expect("校验失败应回退");
        assert!(matches!(
            outcome.status,
            UpdateStatus::FallbackCached { .. }
        ));
    }

    // ── 测试 5：单规则文件 sha256 不符 → 整体放弃更新，缓存原子性 ─────────────

    #[tokio::test]
    async fn rule_file_checksum_mismatch_aborts_atomically() {
        let server = MockServer::start().await;
        let key = test_key();
        // manifest 声明的内容哈希与实际服务内容不符
        let claimed_sha = sha256_hex(b"expected bytes");
        let file_content: &[u8] = b"actual different bytes";
        let manifest = test_manifest(vec![(
            "source-a".into(),
            "anime/source-a.json".into(),
            claimed_sha,
        )]);
        let pkg = sign_package(&manifest, &key);
        mount_package(&server, &pkg, &[("anime/source-a.json", file_content)]).await;

        let app_data = tempfile::tempdir().unwrap();
        let cache = seed_cache(app_data.path());

        let outcome = check_and_update_with_key(
            &http_client(),
            &server.uri(),
            app_data.path(),
            true,
            &key.verifying_key(),
        )
        .await
        .expect("应回退而非 Err");
        assert!(matches!(
            outcome.status,
            UpdateStatus::FallbackCached { .. }
        ));
        assert!(
            cache.join("marker.txt").exists(),
            "部分失败时现有缓存必须逐字节保持"
        );
        // .tmp-* 临时目录已清理
        let leftover = std::fs::read_dir(app_data.path().join("rules-cache"))
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().starts_with(".tmp-"));
        assert!(!leftover, "失败的下载残留 .tmp-* 目录");
    }

    #[tokio::test]
    async fn invalid_downloaded_rule_structure_falls_back_atomically() {
        let server = MockServer::start().await;
        let key = test_key();
        let invalid: &[u8] = br#"{ "name": "broken", "version": "1.0.0" }"#;
        let manifest = test_manifest(vec![(
            "broken".into(),
            "anime/broken.json".into(),
            sha256_hex(invalid),
        )]);
        let pkg = sign_package(&manifest, &key);
        mount_package(&server, &pkg, &[("anime/broken.json", invalid)]).await;

        let app_data = tempfile::tempdir().unwrap();
        let cache = seed_cache(app_data.path());
        let outcome = check_and_update_with_key(
            &http_client(),
            &server.uri(),
            app_data.path(),
            true,
            &key.verifying_key(),
        )
        .await
        .unwrap();
        let reason = match outcome.status {
            UpdateStatus::FallbackCached { reason } => reason,
            other => panic!("期望结构校验失败回退，得到 {other:?}"),
        };
        assert!(reason.contains("结构") || reason.contains("校验"));
        assert!(
            cache.join("marker.txt").exists(),
            "结构失败不得替换现有缓存"
        );
    }

    // ── 测试 6：远端 500 → FallbackCached，不返回 Err 给前端 ──────────────────

    #[tokio::test]
    async fn network_500_falls_back_not_err() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/rules-package.json"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let key = test_key();
        let app_data = tempfile::tempdir().unwrap();
        let outcome = check_and_update_with_key(
            &http_client(),
            &server.uri(),
            app_data.path(),
            true,
            &key.verifying_key(),
        )
        .await
        .expect("网络失败必须回退，不得把 Err 抛给前端");
        assert!(matches!(
            outcome.status,
            UpdateStatus::FallbackCached { .. }
        ));
    }

    // ── 测试 7：24h 节流内 force=false → 不发起任何 HTTP 请求 ─────────────────

    #[tokio::test]
    async fn throttle_skips_http_within_24h() {
        let server = MockServer::start().await;
        let key = test_key();
        let app_data = tempfile::tempdir().unwrap();
        // lastCheckAt 在 24h 内
        std::fs::write(
            app_data.path().join("rules-meta.json"),
            format!(
                r#"{{ "packageVersion": "2026.08.1", "updatedAt": 0, "lastCheckAt": {} }}"#,
                now_ts()
            ),
        )
        .unwrap();

        let outcome = check_and_update_with_key(
            &http_client(),
            &server.uri(),
            app_data.path(),
            false,
            &key.verifying_key(),
        )
        .await
        .expect("节流命中应直接返回");
        assert!(matches!(outcome.status, UpdateStatus::AlreadyLatest));
        let reqs = server.received_requests().await.unwrap();
        assert!(
            reqs.is_empty(),
            "节流内不得发起 HTTP 请求，实际 {} 次",
            reqs.len()
        );
    }

    // ── 测试 8：resolve_rules_dir 缓存优先 + 损坏回退内置 ──────────────────────

    #[test]
    fn resolve_rules_dir_prefers_cache_and_falls_back() {
        let bundled = std::path::Path::new("/bundled/rules");
        // 无 app_data（无缓存）→ 内置目录
        assert_eq!(resolve_rules_dir_from_paths(None, bundled), bundled);

        // 缓存 manifest 可解析 → 用缓存目录
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path().join("rules-cache").join("current");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(
            cache.join("manifest.json"),
            serde_json::to_string(&test_manifest(vec![])).unwrap(),
        )
        .unwrap();
        assert_eq!(
            resolve_rules_dir_from_paths(Some(tmp.path()), bundled),
            cache
        );

        // 缓存 manifest 损坏 → 回退内置目录
        std::fs::write(cache.join("manifest.json"), "not json {").unwrap();
        assert_eq!(
            resolve_rules_dir_from_paths(Some(tmp.path()), bundled),
            bundled
        );
    }

    // ── 测试 9：密钥固化——正确签名通过、篡改/错误 key 拒绝（防恒真假实现）──────

    #[test]
    fn signature_verification_is_not_always_true() {
        let key = test_key();
        let manifest = test_manifest(vec![]);
        let pkg = sign_package(&manifest, &key);

        // 正确签名 → 通过
        verify_signature(&pkg, &key.verifying_key()).expect("正确签名应通过");

        // manifest_sha256 被改 → ChecksumMismatch
        let mut bad_checksum = pkg.clone();
        bad_checksum.manifest_sha256 = hex::encode([1u8; 32]);
        assert!(matches!(
            verify_signature(&bad_checksum, &key.verifying_key()),
            Err(UpdateError::ChecksumMismatch)
        ));

        // 签名被篡改 → InvalidSignature
        let mut bad_sig = pkg.clone();
        let mut sig = base64::engine::general_purpose::STANDARD
            .decode(&bad_sig.signature)
            .unwrap();
        *sig.last_mut().unwrap() ^= 0x01;
        bad_sig.signature = base64::engine::general_purpose::STANDARD.encode(sig);
        assert!(matches!(
            verify_signature(&bad_sig, &key.verifying_key()),
            Err(UpdateError::InvalidSignature)
        ));

        // 用错误密钥验证正确包 → InvalidSignature（证明不是恒真函数）
        let other_key = SigningKey::from_bytes(&[9u8; 32]);
        assert!(matches!(
            verify_signature(&pkg, &other_key.verifying_key()),
            Err(UpdateError::InvalidSignature)
        ));

        // 内嵌公钥可解析为合法 ed25519 密钥
        assert!(embedded_verifying_key().is_ok());
    }

    #[test]
    fn remote_manifest_rejects_unsafe_paths_and_duplicate_ids() {
        let mut manifest = test_manifest(vec![(
            "source-a".into(),
            "../escape.json".into(),
            "a".repeat(64),
        )]);
        assert!(matches!(
            validate_remote_manifest(&manifest),
            Err(UpdateError::Parse(message)) if message.contains("路径")
        ));

        manifest.rules = vec![
            RuleFileEntry {
                id: "same".into(),
                path: "anime/a.json".into(),
                sha256: "a".repeat(64),
                name: "A".into(),
                version: "1.0.0".into(),
                content_type: "anime".into(),
                lang: "zh-CN".into(),
                nsfw: false,
                probe_keyword: None,
            },
            RuleFileEntry {
                id: "same".into(),
                path: "anime/b.json".into(),
                sha256: "b".repeat(64),
                name: "B".into(),
                version: "1.0.0".into(),
                content_type: "anime".into(),
                lang: "zh-CN".into(),
                nsfw: false,
                probe_keyword: None,
            },
        ];
        assert!(matches!(
            validate_remote_manifest(&manifest),
            Err(UpdateError::Parse(message)) if message.contains("重复")
        ));
    }
}
