// 萌游 MoeGame · 自动入库刮削管线（M6 — 对标 C# AutoScrapeService）
//
// 原版 C# 一条龙流程：
//   ① GameDetector: 递归扫描 → 识别引擎 → 多碟合并 → 选择最佳 exe
//   ② AiTitleResolver: LLM 从脏文件夹名推断规范标题
//   ③ GameLibraryDeduplicator: 去重（路径+名称+模糊匹配）
//   ④ MetadataService: 多源并行刮削 + 缓存
//   ⑤ AiMetadataEnhancer: AI 中文简介润色 + 标签翻译
//   ⑥ ScreenshotFetchService: 截图/CG/立绘自动下载
//   ⑦ 写入数据库
//
// 策略:
//   Full:         完整检测→AI标题→刮削→增强→截图（全量）
//   Incremental:  只处理新游戏/目录
//   PatchMissing: 补缺已有游戏的缺失字段
//   RetryFailed:  重新处理之前失败的
//
// 设计原则: 所有步骤通过 `PipelineState` 传递状态，每一步可选，失败不阻塞下一步。

use crate::db::Database;
use crate::models::Game;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;

// ============================================================================
// 管线状态
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    /// 处理阶段
    pub stage: String,
    /// 当前进度
    pub current: usize,
    pub total: usize,
    /// 检测到的候选游戏路径
    pub detected: Vec<PathBuf>,
    /// 成功入库数
    pub imported: usize,
    /// 更新（已存在但信息不全补全）
    pub updated: usize,
    /// 跳过（重复 / 非游戏）
    pub skipped: usize,
    /// 错误信息
    pub errors: Vec<String>,
}

impl Default for PipelineState {
    fn default() -> Self {
        Self {
            stage: "idle".into(),
            current: 0,
            total: 0,
            detected: vec![],
            imported: 0,
            updated: 0,
            skipped: 0,
            errors: vec![],
        }
    }
}

/// 扫描结果 — 检测到的游戏候选
#[derive(Debug, Clone)]
pub struct GameCandidate {
    pub path: PathBuf,
    pub is_archive: bool,
    pub suggested_name: String,
    pub engine: Option<String>,
    pub best_exe: Option<PathBuf>,
}

// ============================================================================
// 步骤 1: GameDetector — 递归扫描目录，多碟合并，引擎识别
// ============================================================================

/// 递归扫描目录，返回所有 .exe 候选（跳过 unins/setup/update/patcher）
pub fn detect_games(root: &Path, state: &mut PipelineState) -> Vec<GameCandidate> {
    state.stage = "detect".into();
    let mut candidates = Vec::new();

    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.starts_with('.') || name.starts_with('$') {
                continue;
            }

            if path.is_dir() {
                // 递归子目录
                candidates.extend(detect_games(&path, state));
            } else if path.is_file() {
                // 检测可执行文件
                if crate::import::is_executable(&path) && !crate::import::is_skip_exe(&path) {
                    let parent = path.parent().unwrap_or_else(|| Path::new("."));
                    let dir_name = parent
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "未知游戏".into());
                    let engine = crate::locale::EngineLibrary::detect_engine(parent)
                        .map(|ec| format!("{:?}", ec.engine));

                    candidates.push(GameCandidate {
                        path: path.clone(),
                        is_archive: false,
                        suggested_name: dir_name,
                        engine,
                        best_exe: Some(path),
                    });
                } else if crate::import::is_archive(&path) {
                    candidates.push(GameCandidate {
                        path: path.clone(),
                        is_archive: true,
                        suggested_name: name
                            .trim_end_matches(".zip")
                            .trim_end_matches(".7z")
                            .trim_end_matches(".rar")
                            .to_string(),
                        engine: None,
                        best_exe: None,
                    });
                }
            }
        }
    }

    // 多碟合并：同一父目录下相同前缀的文件夹合并为一个候选
    candidates = merge_multi_disc(candidates);

    state.detected = candidates.iter().map(|c| c.path.clone()).collect();
    state.total = candidates.len();
    candidates
}

/// 多碟合并：如果同目录下有 "Game Disc 1" 和 "Game Disc 2"，只保留第一个
fn merge_multi_disc(candidates: Vec<GameCandidate>) -> Vec<GameCandidate> {
    let mut merged = Vec::new();
    let mut skip: Vec<usize> = Vec::new();

    for (i, c) in candidates.iter().enumerate() {
        if skip.contains(&i) {
            continue;
        }
        let base = c
            .suggested_name
            .to_lowercase()
            .replace(" disc 1", "")
            .replace(" disc1", "")
            .replace(" (disc 1)", "")
            .replace(" [disc 1]", "")
            .trim()
            .to_string();

        // 检查后续是否有同名多碟
        for (j, other) in candidates.iter().enumerate().skip(i + 1) {
            let ob = other
                .suggested_name
                .to_lowercase()
                .replace(['(', ')', '[', ']'], " ")
                .trim()
                .to_string();
            if ob.contains(&base) || base.contains(&ob) {
                skip.push(j);
            }
        }
        merged.push(c.clone());
    }
    merged
}

// ============================================================================
// 步骤 2: AiTitleResolver — 从文件夹名推断规范标题
// ============================================================================

/// 从文件夹名推断规范游戏标题（清理版本号、社团名、括号内容等）
pub fn infer_title_from_folder(folder_name: &str) -> String {
    let mut name = folder_name.to_string();

    // 移除括号内的版本/语言/加密信息
    for pattern in &[
        "v1.",
        "v2.",
        "v3.",
        "ver1.",
        "ver2.",
        "ver.",
        "DL版",
        "PKG版",
        "パッケージ版",
        "Package",
        "Chinese",
        "汉化",
        "中文",
        "官方中文",
        "Steam",
        "DMM",
        "DLsite",
        "生肉",
        "未汉化",
        "日文",
        "无码",
        "解码",
        "骑兵",
        "步兵",
        "HD",
        "Remaster",
        "重制",
        "完全版",
        "フルボイス",
        "FullVoice",
        "修正",
        "patched",
        "cracked",
    ] {
        name = remove_bracket_content(&name, pattern);
    }

    // 移除末尾的版本号
    if let Some(pos) = name.rfind(['v', 'V']) {
        if pos > 0 && pos < name.len() - 1 {
            let after = &name[pos + 1..];
            if after.chars().all(|c| c.is_ascii_digit() || c == '.') {
                name = name[..pos].trim().to_string();
            }
        }
    }

    // 移除尾部的社团名标记（通常在括号里）
    // "ゲーム名 (開発元)" → "ゲーム名"
    if name.ends_with(')') {
        if let Some(open) = name.rfind('(') {
            let content = &name[open + 1..name.len() - 1];
            // 如果括号内容包含常见社团名后缀，移除
            if content.contains("社")
                || content.contains("工房")
                || content.contains("Studio")
                || content.contains("soft")
                || content.contains("SOFT")
            {
                name = name[..open].trim().to_string();
            }
        }
    }

    name.trim().to_string()
}

fn remove_bracket_content(s: &str, keyword: &str) -> String {
    let lower = s.to_lowercase();
    let kw = keyword.to_lowercase();

    for (open, close) in &[('（', '）'), ('(', ')'), ('[', ']')] {
        if let Some(start) = lower.find(*open) {
            if let Some(end) = lower[start..].find(*close) {
                let content = &lower[start + 1..start + end];
                if content.contains(&kw) {
                    let before = &s[..start];
                    let after = &s[start + end + 1..];
                    return format!("{} {}", before, after).trim().to_string();
                }
            }
        }
    }
    s.to_string()
}

// ============================================================================
// 步骤 3: GameLibraryDeduplicator — 去重
// ============================================================================

/// 检查候选是否与已有库重复。
///
/// 名称（包括归一化/模糊名称）仅用于召回，绝不能自动判重或合并。
/// 这里没有平台 ID 输入，因此只接受规范化启动路径这一强身份。
pub fn is_duplicate(_name: &str, exe_path: &str, existing: &[Game]) -> bool {
    let exe_normalized = normalize_identity_path(exe_path);
    !exe_normalized.is_empty()
        && existing
            .iter()
            .any(|game| normalize_identity_path(&game.exe_path) == exe_normalized)
}

fn normalize_identity_path(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

#[cfg(test)]
fn normalize_game_name(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
fn levenshtein_distance(a: &str, b: &str) -> Option<usize> {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let n = a_chars.len();
    let m = b_chars.len();
    if n == 0 {
        return Some(m);
    }
    if m == 0 {
        return Some(n);
    }
    if n * m > 10000 {
        return None;
    }

    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in dp.iter_mut().enumerate().take(n + 1) {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate().take(m + 1) {
        *cell = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    Some(dp[n][m])
}

// ============================================================================
// 步骤 4+5+6: 完整入库管线（一站式）
// ============================================================================

/// 执行完整入库管线：检测 → 去重 → 入库 → 排队刮削
pub fn run_full_pipeline(db: &Database, dir: &Path, auto_scrape: bool, state: &mut PipelineState) {
    let start = Instant::now();
    let existing = db.get_games();

    // Step 1: 检测
    let candidates = detect_games(dir, state);
    tracing::info!(
        dir = %dir.display(),
        found = candidates.len(),
        "AutoScrape: detected candidates"
    );

    // Step 2+3: 标题推断 + 去重
    state.stage = "import".into();
    for (i, candidate) in candidates.iter().enumerate() {
        state.current = i + 1;

        let name = if candidate.is_archive {
            candidate.suggested_name.clone()
        } else {
            infer_title_from_folder(&candidate.suggested_name)
        };

        let exe_path_str = candidate
            .best_exe
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        // 去重
        if is_duplicate(&name, &exe_path_str, &existing) {
            state.skipped += 1;
            continue;
        }

        // 入库
        let mut game = Game::new(name.clone(), exe_path_str.clone());
        if let Some(parent) = candidate.path.parent() {
            game.install_dir = Some(parent.to_string_lossy().to_string());
        }
        if let Some(ref engine) = candidate.engine {
            game.metadata.engine = Some(engine.clone());
        }

        match db.add_game(game) {
            Ok(g) => {
                state.imported += 1;
                tracing::info!(name = %g.name, "AutoScrape: imported");

                // 异步排入刮削队列
                if auto_scrape {
                    // 这里由调用方负责异步刮削
                    // 通过 Tauri emit 通知前端
                }
            }
            Err(e) => {
                state.skipped += 1;
                if !e.contains("已存在") {
                    state.errors.push(format!("{}: {}", name, e));
                }
            }
        }
    }

    let elapsed = start.elapsed();
    tracing::info!(
        imported = state.imported,
        skipped = state.skipped,
        errors = state.errors.len(),
        elapsed_ms = elapsed.as_millis(),
        "AutoScrape: pipeline complete"
    );
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_title_removes_version() {
        let result = infer_title_from_folder("CLANNAD v1.2");
        assert!(
            !result.contains("v1.2"),
            "Expected no version in '{}', got '{}",
            "CLANNAD v1.2",
            result
        );
    }

    #[test]
    fn test_infer_title_removes_junk_tags() {
        let result = infer_title_from_folder("美少女万華鏡 (DL版 汉化)");
        assert!(
            result.contains("美少女万華鏡"),
            "Expected 美少女万華鏡 in '{}', got '{}",
            "美少女万華鏡 (DL版 汉化)",
            result
        );
        assert!(!result.contains("DL版"));
        assert!(!result.contains("汉化"));
    }

    #[test]
    fn test_normalize_name() {
        assert_eq!(normalize_game_name("Steins; Gate"), "steins gate");
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein_distance("clannad", "clannad").unwrap(), 0);
        assert_eq!(levenshtein_distance("clanad", "clannad").unwrap(), 1);
        assert!(levenshtein_distance("abcdefghij", "xyz").unwrap() > 0);
    }

    #[test]
    fn test_is_duplicate_same_name() {
        use crate::models::{Game, GameMetadata, PlayTracker, SaveData};
        let existing = vec![Game {
            id: "test1".into(),
            name: "CLANNAD".into(),
            exe_path: "c:/games/clannad/clannad.exe".into(),
            install_dir: None,
            game_type: None,
            library_source: None,
            library_id: None,
            launch_uri: None,
            last_imported_at: None,
            created_at: "2026-01-01".into(),
            updated_at: "2026-01-01".into(),
            description: None,
            cover: None,
            background: None,
            icon: None,
            screenshots: vec![],
            favorite: false,
            hidden: false,
            tags: vec![],
            tag_entries: vec![],
            aliases: vec![],
            metadata: GameMetadata::default(),
            play_tracker: PlayTracker::default(),
            save_data: SaveData::default(),
            release_year: None,
            rating: None,
            last_played: None,
            vndb_id: None,
            bangumi_id: None,
            play_time_seconds: 0,
            add_date: None,
        }];
        assert!(is_duplicate(
            "CLANNAD",
            "c:/games/clannad/clannad.exe",
            &existing
        ));
        assert!(!is_duplicate("Clannad", "c:/other/game.exe", &existing));
        assert!(!is_duplicate("Steins Gate", "c:/sg/sg.exe", &existing));
    }
}
