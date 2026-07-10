// 萌游 MoeGame · 压缩包解压模块（M4）
//
// 支持：
//   - zip 格式（via `zip` crate，纯 Rust，零系统依赖）
//   - 7z/rar 格式（via 命令行 7z.exe 回退，Windows 主）
//   - 自动根目录定位（跳过包裹层目录，直接找到游戏根）
//   - 覆盖策略控制

use crate::security::SecurityScope;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 解压结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResult {
    /// 解压到的目标目录
    pub output_dir: String,
    /// 解压出的文件数
    pub files_extracted: usize,
    /// 检测到的游戏根目录（可能为子目录）
    pub game_root: Option<String>,
}

/// 支持的归档格式
#[derive(Debug, Clone, PartialEq)]
pub enum ArchiveFormat {
    Zip,
    SevenZip,
    Rar,
    Tar,
    Gz,
    Unknown,
}

/// 当前归档实现可以承诺的取消粒度。
///
/// 纯 Rust ZIP 循环可以在条目边界检查取消，但单个条目的同步 copy
/// 仍不可中断；外部 7z/rar 使用阻塞 `Command::output`，只能在进程退出后
/// 观察取消。调用方不得把这两类行为描述成即时取消。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveCancellationSupport {
    BetweenEntries,
    AfterExternalProcessExit,
    NotApplicable,
}

pub const ARCHIVE_CANCELLATION_LIMITATION: &str =
    "ZIP 仅能在条目之间取消，当前条目会继续写完；7z/rar/tar/gz 要等待外部进程退出，已写文件不会回滚";

impl ArchiveFormat {
    pub fn cancellation_support(&self) -> ArchiveCancellationSupport {
        match self {
            Self::Zip => ArchiveCancellationSupport::BetweenEntries,
            Self::SevenZip | Self::Rar | Self::Tar | Self::Gz => {
                ArchiveCancellationSupport::AfterExternalProcessExit
            }
            Self::Unknown => ArchiveCancellationSupport::NotApplicable,
        }
    }

    pub fn from_path(path: &Path) -> Self {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str()
        {
            "zip" => Self::Zip,
            "7z" => Self::SevenZip,
            "rar" | "rar5" => Self::Rar,
            "tar" => Self::Tar,
            "gz" | "xz" => Self::Gz,
            _ => Self::Unknown,
        }
    }

    pub fn is_supported(&self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

/// 解压配置
#[derive(Debug, Clone)]
pub struct ExtractConfig {
    /// 输出根目录（解压到此目录下的子文件夹中）
    pub output_base: PathBuf,
    /// 是否覆盖已存在的文件
    pub overwrite: bool,
    /// 是否自动检测游戏根目录
    pub detect_game_root: bool,
}

impl Default for ExtractConfig {
    fn default() -> Self {
        Self {
            output_base: dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("moeplay")
                .join("extracted"),
            overwrite: false,
            detect_game_root: true,
        }
    }
}

/// 解压归档文件。返回解压结果（含检测的游戏根目录）。
pub fn extract_archive(
    archive_path: &Path,
    config: &ExtractConfig,
    on_progress: &dyn Fn(u64, u64, &str), // current, total, message
) -> Result<ExtractResult, String> {
    if !archive_path.is_file() {
        return Err(format!("文件不存在: {}", archive_path.display()));
    }

    let format = ArchiveFormat::from_path(archive_path);
    if !format.is_supported() {
        return Err(format!("不支持的格式: {:?}", format));
    }

    // 创建目标目录：output_base/archive_stem/
    let stem = archive_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let output_dir = config.output_base.join(sanitize_dir_name(stem));

    // 如果已存在且不覆盖，返回已存在的目录
    if output_dir.exists() && !config.overwrite {
        let files_count = count_files(&output_dir);
        let game_root = if config.detect_game_root {
            find_game_root(&output_dir).map(|p| p.to_string_lossy().to_string())
        } else {
            None
        };
        return Ok(ExtractResult {
            output_dir: output_dir.to_string_lossy().to_string(),
            files_extracted: files_count,
            game_root,
        });
    }

    on_progress(0, 1, &format!("正在解压: {}", stem));

    let files_extracted = match format {
        ArchiveFormat::Zip => extract_zip(archive_path, &output_dir)?,
        ArchiveFormat::SevenZip | ArchiveFormat::Rar | ArchiveFormat::Tar | ArchiveFormat::Gz => {
            extract_via_7z(archive_path, &output_dir)?
        }
        ArchiveFormat::Unknown => return Err("未知格式".into()),
    };

    on_progress(1, 1, "解压完成，正在检测游戏目录...");

    let game_root = if config.detect_game_root {
        find_game_root(&output_dir).map(|p| p.to_string_lossy().to_string())
    } else {
        None
    };

    Ok(ExtractResult {
        output_dir: output_dir.to_string_lossy().to_string(),
        files_extracted,
        game_root,
    })
}

/// 解压 zip 文件（纯 Rust），并防止 Zip Slip 攻击。
fn extract_zip(archive_path: &Path, output_dir: &Path) -> Result<usize, String> {
    let file = fs::File::open(archive_path).map_err(|e| format!("打开 zip 失败: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("解析 zip 失败: {}", e))?;

    fs::create_dir_all(output_dir).map_err(|e| format!("创建目录失败: {}", e))?;

    let mut scope = SecurityScope::new();
    scope.allow(output_dir);

    let total = archive.len();
    let mut extracted = 0;
    for i in 0..total {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("读取 zip 条目 {}: {}", i, e))?;

        // 使用 enclosed_name 过滤掉 ../ 等越界路径
        let safe_name = match entry.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => {
                tracing::warn!("跳过不安全的 zip 条目: {}", entry.name());
                continue;
            }
        };

        let name = safe_name.to_string_lossy().to_string();

        // 跳过 macOS 资源文件
        if name.starts_with("__MACOSX") || name.contains(".DS_Store") {
            continue;
        }

        let dest = output_dir.join(&safe_name);

        if entry.is_dir() {
            fs::create_dir_all(&dest).map_err(|e| format!("创建目录 {}: {}", name, e))?;
            let _ = scope.resolve(&dest)?;
        } else {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("创建父目录 {}: {}", parent.display(), e))?;
            }
            let dest = scope.resolve(&dest)?;
            let mut out =
                fs::File::create(&dest).map_err(|e| format!("创建文件 {}: {}", name, e))?;
            std::io::copy(&mut entry, &mut out).map_err(|e| format!("写入文件 {}: {}", name, e))?;
            extracted += 1;
        }
    }

    Ok(extracted)
}

/// 通过 7z 命令行解压（7z/rar/tar/gz）。
#[cfg(windows)]
fn extract_via_7z(archive_path: &Path, output_dir: &Path) -> Result<usize, String> {
    // 尝试找到 7z.exe
    let seven_zip = find_7z();
    let cmd =
        seven_zip.ok_or("未安装 7-Zip，无法解压 7z/rar 格式。请安装 7-Zip 或使用 zip 格式。")?;

    fs::create_dir_all(output_dir).map_err(|e| format!("创建目录失败: {}", e))?;

    let output = std::process::Command::new(&cmd)
        .args([
            "x", // extract with full paths
            &archive_path.to_string_lossy(),
            &format!("-o{}", output_dir.display()),
            "-y",   // yes to all
            "-aoa", // overwrite all
        ])
        .output()
        .map_err(|e| format!("启动 7z 失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("7z 解压失败: {}", stderr));
    }

    Ok(count_files(output_dir))
}

#[cfg(not(windows))]
fn extract_via_7z(archive_path: &Path, output_dir: &Path) -> Result<usize, String> {
    // Linux/Mac: 尝试 7z 或 unzip/tar
    let cmd = if which("7z").is_some() {
        "7z"
    } else if which("7zz").is_some() {
        "7zz"
    } else {
        return Err("未找到 7z/7zz。请安装 p7zip。".into());
    };

    fs::create_dir_all(output_dir).map_err(|e| format!("创建目录失败: {}", e))?;

    let output = std::process::Command::new(cmd)
        .args([
            "x",
            &archive_path.to_string_lossy(),
            &format!("-o{}", output_dir.display()),
            "-y",
        ])
        .output()
        .map_err(|e| format!("启动 {} 失败: {}", cmd, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{} 解压失败: {}", cmd, stderr));
    }

    Ok(count_files(output_dir))
}

/// 查找 7z.exe（Windows）。
#[cfg(windows)]
fn find_7z() -> Option<PathBuf> {
    for candidate in &[
        r"C:\Program Files\7-Zip\7z.exe",
        r"C:\Program Files (x86)\7-Zip\7z.exe",
    ] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return Some(p);
        }
    }
    // PATH 中查找
    if let Ok(output) = std::process::Command::new("where").arg("7z").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = stdout.lines().next() {
            let p = PathBuf::from(line.trim());
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

/// 简单 which 命令（非 Windows）。
#[cfg(not(windows))]
fn which(cmd: &str) -> Option<PathBuf> {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .ok()
        .and_then(|o| {
            let path = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if path.is_empty() {
                None
            } else {
                Some(PathBuf::from(path))
            }
        })
}

// ============================================================================
// 游戏根目录检测
// ============================================================================

/// 在解压后的目录中查找游戏根目录。
/// 规则：
///   1. 如果当前目录直接包含 .exe，返回当前目录
///   2. 如果只有一个子目录，递归进入
///   3. 否则返回当前目录（多子目录的情况，选最佳）
pub fn find_game_root(dir: &Path) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }

    // 当前目录有 exe → 这就是根
    if has_exe_in(dir) {
        return Some(dir.to_path_buf());
    }

    // 收集子目录
    let subdirs: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect();

    // 过滤掉明显的非游戏目录
    let game_subdirs: Vec<PathBuf> = subdirs
        .into_iter()
        .filter(|d| {
            let name = d
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            !name.starts_with('.')
                && !name.starts_with('$')
                && name != "__macosx"
                && name != "system volume information"
        })
        .collect();

    if game_subdirs.is_empty() {
        return Some(dir.to_path_buf());
    }

    // 只有一个子目录 → 递归
    if game_subdirs.len() == 1 {
        return find_game_root(&game_subdirs[0]).or_else(|| Some(dir.to_path_buf()));
    }

    // 多个子目录 → 选 exe 最多的那个
    let best = game_subdirs.iter().max_by_key(|d| count_exe_in(d)).cloned();

    best.or_else(|| Some(dir.to_path_buf()))
}

fn has_exe_in(dir: &Path) -> bool {
    fs::read_dir(dir)
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "exe")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn count_exe_in(dir: &Path) -> usize {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "exe")
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

// ============================================================================
// 智能 EXE 识别与评分
// ============================================================================

/// EXE 候选
#[derive(Debug, Clone)]
pub struct ExeCandidate {
    pub path: PathBuf,
    pub name: String,
    pub score: u32,
    pub reason: String,
}

/// 在目录中搜索可执行文件并评分排序。
/// 使用 `EngineLibrary` 引擎检测 + 本地评分规则。
pub fn find_best_exe(dir: &Path) -> Option<ExeCandidate> {
    let mut candidates = find_all_exes(dir);
    if candidates.is_empty() {
        return None;
    }

    // 引擎评分
    let engine_config = crate::locale::EngineLibrary::detect_engine(dir);
    for c in &mut candidates {
        if let Some(ref ec) = engine_config {
            let engine_score = crate::locale::EngineLibrary::score_executable(&c.path, dir, ec);
            c.score += engine_score;
        }
        // 本地规则：文件名和目录名匹配加分
        if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
            if c.name.to_lowercase().contains(&dir_name.to_lowercase()) {
                c.score += 20;
                c.reason.push_str(" | 名称匹配");
            }
        }
    }

    candidates.sort_by_key(|c| -(c.score as i64));
    candidates.into_iter().next()
}

fn find_all_exes(dir: &Path) -> Vec<ExeCandidate> {
    let mut results = vec![];
    if !dir.is_dir() {
        return results;
    }

    let skip_keywords = &[
        "unins",
        "uninst",
        "uninstall",
        "setup",
        "install",
        "update",
        "config",
        "launcher_patch",
        "patcher",
        "cleanup",
        "remove",
        "vc_redist",
        "dxsetup",
        "dotnet",
        "vcredist",
    ];

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "exe").unwrap_or(false) {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                let lower = name.to_lowercase();
                if skip_keywords.iter().any(|kw| lower.contains(kw)) {
                    continue;
                }
                let mut size_score = 0u32;
                if let Ok(meta) = fs::metadata(&path) {
                    if meta.len() > 1_000_000 {
                        size_score = 10;
                    } else if meta.len() > 100_000 {
                        size_score = 5;
                    }
                }
                results.push(ExeCandidate {
                    path,
                    name,
                    score: size_score,
                    reason: format!("文件大小评分:{}", size_score),
                });
            }
        }
    }
    results
}

// ============================================================================
// 重复检测
// ============================================================================

/// 重复检测只接受规范化启动路径这一强身份。
/// 同名/近似名仅用于上层召回，不能在解压导入阶段自动合并。
pub fn is_duplicate(_new_name: &str, new_path: &str, existing: &[crate::models::Game]) -> bool {
    let normalized = normalize_identity_path(new_path);
    !normalized.is_empty()
        && existing
            .iter()
            .any(|game| normalize_identity_path(&game.exe_path) == normalized)
}

fn normalize_identity_path(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

// ============================================================================
// 工具函数
// ============================================================================

fn count_files(dir: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                count += 1;
            } else if path.is_dir() {
                count += count_files(&path);
            }
        }
    }
    count
}

fn sanitize_dir_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .chars()
        .take(80)
        .collect()
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_from_ext() {
        assert_eq!(
            ArchiveFormat::from_path(Path::new("game.zip")),
            ArchiveFormat::Zip
        );
        assert_eq!(
            ArchiveFormat::from_path(Path::new("game.7z")),
            ArchiveFormat::SevenZip
        );
        assert_eq!(
            ArchiveFormat::from_path(Path::new("game.rar")),
            ArchiveFormat::Rar
        );
        assert_eq!(
            ArchiveFormat::from_path(Path::new("game.exe")),
            ArchiveFormat::Unknown
        );
    }

    #[test]
    fn cancellation_support_is_explicit_and_never_claims_immediate_interrupt() {
        assert_eq!(
            ArchiveFormat::Zip.cancellation_support(),
            ArchiveCancellationSupport::BetweenEntries
        );
        assert_eq!(
            ArchiveFormat::SevenZip.cancellation_support(),
            ArchiveCancellationSupport::AfterExternalProcessExit
        );
        assert_eq!(
            ArchiveFormat::Rar.cancellation_support(),
            ArchiveCancellationSupport::AfterExternalProcessExit
        );
        assert!(ARCHIVE_CANCELLATION_LIMITATION.contains("不会回滚"));
    }

    #[test]
    fn test_find_game_root_current_has_exe() {
        let tmp = std::env::temp_dir().join("m4_test_root1");
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("game.exe"), b"fake").unwrap();
        let root = find_game_root(&tmp);
        assert_eq!(root, Some(tmp.clone()));
        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_find_game_root_single_subdir() {
        let tmp = std::env::temp_dir().join("m4_test_root2");
        let sub = tmp.join("GameData");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("game.exe"), b"fake").unwrap();
        let root = find_game_root(&tmp);
        assert_eq!(root, Some(sub));
        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_find_best_exe_skips_uninstall() {
        let tmp = std::env::temp_dir().join("m4_test_exe");
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("uninstall.exe"), b"fake").unwrap();
        fs::write(tmp.join("MyGame.exe"), b"real game").unwrap();
        let best = find_best_exe(&tmp);
        assert!(best.is_some());
        assert_eq!(best.unwrap().name, "MyGame");
        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_is_duplicate_path() {
        let g = crate::models::Game::new("Test".into(), "C:\\Games\\Test.exe".into());
        assert!(is_duplicate("Test", "C:\\Games\\Test.exe", &[g]));
    }

    #[test]
    fn test_is_duplicate_name() {
        let g = crate::models::Game::new("Test Game".into(), "C:\\Games\\Other.exe".into());
        assert!(!is_duplicate(
            "Test Game",
            "D:\\Test.exe",
            std::slice::from_ref(&g)
        ));
        assert!(!is_duplicate("Different", "D:\\Test.exe", &[g]));
    }

    #[test]
    fn test_sanitize_dir_name() {
        // CJK characters pass is_alphanumeric, so they're kept
        let result = sanitize_dir_name("Steins;Gate シュタインズ");
        assert!(result.starts_with("Steins_Gate"));
        assert!(!result.contains(';'));
        assert!(!result.contains(' '));
    }

    #[test]
    fn test_zip_slip_blocks_traversal() {
        use std::io::Write;

        let tmp = std::env::temp_dir().join("m4_test_zip_slip");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let zip_path = tmp.join("evil.zip");
        let output_dir = tmp.join("extracted");

        {
            let file = fs::File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("safe.txt", options).unwrap();
            writer.write_all(b"hello").unwrap();
            writer.start_file("../evil.txt", options).unwrap();
            writer.write_all(b"bad").unwrap();
            writer.finish().unwrap();
        }

        let result = extract_zip(&zip_path, &output_dir);
        assert!(result.is_ok());
        assert!(output_dir.join("safe.txt").exists());
        assert!(!tmp.join("evil.txt").exists());

        fs::remove_dir_all(&tmp).ok();
    }
}
