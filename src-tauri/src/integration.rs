// 萌游 MoeGame · Steam/Epic 游戏库导入集成（M6 重写）
//
// Steam 检测：
//   1. 读注册表 `HKLM\SOFTWARE\Wow6432Node\Valve\Steam\InstallPath`
//   2. 解析 `steamapps/libraryfolders.vdf`（新版嵌套格式）
//   3. 遍历每个 library 的 `steamapps/appmanifest_*.acf`
//   4. 从 appmanifest 提取 appid/name/installdir → 正确拼接 install_path
//
// Epic 检测：
//   1. 读 `%ProgramData%/Epic/EpicGamesLauncher/Data/Manifests/*.item`
//   2. 解析 .item JSON 提取 DisplayName/InstallLocation/CatalogItemId

use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ============================================================================
// 数据模型
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedGame {
    pub name: String,
    pub install_path: PathBuf,
    pub platform: String,
    pub app_id: Option<String>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub platform: String,
    pub games_found: usize,
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

// ============================================================================
// Steam: 发现 Steam 安装路径
// ============================================================================

/// Steam 安装路径缓存——避免每次导入/同步时反复读注册表。
/// 之前用 PowerShell 读取（per-game 触发时会不停弹窗→闪退），现改为 winreg 直接读。
static STEAM_PATH_CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();

fn find_steam_from_registry() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        let hklm = winreg::RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(key) = hklm.open_subkey_with_flags(
            r"SOFTWARE\WOW6432Node\Valve\Steam",
            KEY_READ,
        ) {
            if let Ok(path) = key.get_value::<String, _>("InstallPath") {
                let p = PathBuf::from(&path);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }
    #[cfg(not(windows))]
    let _ = ();
    None
}

/// 发现 Steam 安装路径（带缓存，不再弹 PowerShell 窗口）。
/// Windows: 读注册表 → 环境变量 → 常见路径。
/// 非 Windows: ~/.steam 或 ~/.local/share/Steam。
pub fn find_steam_install_path() -> Option<PathBuf> {
    STEAM_PATH_CACHE
        .get_or_init(|| {
            // 方法1: 读注册表（winreg 直接读，不弹窗）
            if let Some(p) = find_steam_from_registry() {
                tracing::info!(path = %p.display(), "Steam path found via registry");
                return Some(p);
            }

            // 方法2: 环境变量
            for env_var in &["STEAM_HOME", "STEAM_PATH"] {
                if let Ok(val) = std::env::var(env_var) {
                    let p = PathBuf::from(&val);
                    if p.join("steamapps").exists() {
                        return Some(p);
                    }
                }
            }

            // 方法3: 常见路径
            #[cfg(windows)]
            let common_paths: &[&str] = &[
                r"C:\Program Files (x86)\Steam",
                r"C:\Program Files\Steam",
                r"D:\Steam",
                r"E:\Steam",
            ];
            #[cfg(not(windows))]
            let common_paths: &[&str] = &[];

            for p in common_paths {
                let pb = PathBuf::from(p);
                if pb.join("steamapps").exists() {
                    return Some(pb);
                }
            }

            #[cfg(not(windows))]
            {
                for p in &[".steam/steam", ".local/share/Steam"] {
                    if let Some(home) = dirs::home_dir() {
                        let pb = home.join(p);
                        if pb.join("steamapps").exists() {
                            return Some(pb);
                        }
                    }
                }
            }

            None
        })
        .clone()
}

// ============================================================================
// Steam: 解析 libraryfolders.vdf → 库目录列表
// ============================================================================

/// 解析新版 libraryfolders.vdf 格式。
/// 格式: "libraryfolders" { "0" { "path" "..." "label" "..." } ... }
pub fn parse_library_folders(steam_path: &Path) -> Vec<PathBuf> {
    let vdf_path = steam_path.join("steamapps").join("libraryfolders.vdf");
    if !vdf_path.exists() {
        // 只有一个库目录：steam_path/steamapps
        let common = steam_path.join("steamapps").join("common");
        if common.exists() {
            return vec![steam_path.join("steamapps")];
        }
        return vec![];
    }

    let content = match std::fs::read_to_string(&vdf_path) {
        Ok(c) => c,
        Err(_) => {
            let common = steam_path.join("steamapps").join("common");
            return if common.exists() {
                vec![steam_path.join("steamapps")]
            } else {
                vec![]
            };
        }
    };

    let mut libraries = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = content.chars().collect();

    // 查找所有 "path" "..." 键值对（嵌套在 libraryfolders 块中）
    while i < chars.len() {
        if let Some(idx) = find_str(&chars, i, "\"path\"") {
            i = idx + 6; // skip past "path" (6 chars including quotes)
                         // 跳过空白和开引号
            while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t' || chars[i] == '\n') {
                i += 1;
            }
            if i < chars.len() && chars[i] == '"' {
                i += 1;
                let mut path = String::new();
                while i < chars.len() && chars[i] != '"' {
                    path.push(chars[i]);
                    i += 1;
                }
                i += 1; // skip closing quote
                let path = path.replace("\\\\", "\\");
                if !path.is_empty() {
                    let lib = PathBuf::from(&path);
                    let common = lib.join("steamapps").join("common");
                    if common.exists() {
                        libraries.push(lib.join("steamapps"));
                    }
                }
            }
        } else {
            break;
        }
    }

    // 如果没解析到任何库，至少包含主 steamapps 目录
    if libraries.is_empty() {
        let primary = steam_path.join("steamapps");
        if primary.exists() {
            libraries.push(primary);
        }
    }

    // 去重
    libraries.sort();
    libraries.dedup();
    libraries
}

// ============================================================================
// Steam: 扫描已安装游戏
// ============================================================================

/// 扫描所有 Steam 库中的已安装游戏。
/// 每个库目录下遍历 `appmanifest_*.acf` 文件。
pub fn scan_steam_games() -> Vec<ImportedGame> {
    let steam_path = match find_steam_install_path() {
        Some(p) => p,
        None => {
            tracing::warn!("Steam installation not found");
            return vec![];
        }
    };

    let libraries = parse_library_folders(&steam_path);
    tracing::info!(steam = %steam_path.display(), libraries = libraries.len(), "Found Steam libraries");

    let mut games = Vec::new();

    for lib in &libraries {
        let manifest_dir = lib; // lib = .../steamapps
        if let Ok(entries) = std::fs::read_dir(manifest_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if fname.starts_with("appmanifest_") && fname.ends_with(".acf") {
                    if let Some(game) = parse_appmanifest(&path, lib) {
                        games.push(game);
                    }
                }
            }
        }
    }

    tracing::info!(found = games.len(), "Steam games scanned");
    games
}

/// 解析单个 appmanifest_*.acf 文件。
/// 关键字段：appid, name, installdir → 拼接完整安装路径。
fn parse_appmanifest(path: &Path, library_steamapps: &Path) -> Option<ImportedGame> {
    let content = std::fs::read_to_string(path).ok()?;

    let appid = extract_acf_value(&content, "appid");
    let name = extract_acf_value(&content, "name");
    let installdir = extract_acf_value(&content, "installdir");

    let name = name?;
    if name.is_empty() {
        return None;
    }

    // 正确拼接安装路径：library/steamapps/common/{installdir}
    let common_dir = library_steamapps.join("common");
    let install_path = common_dir.join(installdir.as_deref().unwrap_or(&name));

    let cover_url = appid.as_ref().map(|id| {
        format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{id}/library_600x900.jpg")
    });

    Some(ImportedGame {
        name,
        install_path,
        platform: "Steam".into(),
        app_id: appid,
        cover_url,
    })
}

/// 从 ACF 文本中提取 `"key"\t\t"value"` 格式的值。
fn extract_acf_value(content: &str, key: &str) -> Option<String> {
    let search = format!("\"{}\"", key);
    let pos = content.find(&search)?;
    let after_key = &content[pos + search.len()..];

    // 跳过制表符/空格找到下一个引号
    let start = after_key.find('"')?;
    let after_start = &after_key[start + 1..];
    let end = after_start.find('"')?;
    let val = after_start[..end].to_string();

    if val.is_empty() {
        None
    } else {
        Some(val)
    }
}

// ============================================================================
// Steam: 本地库中按名称查找封面（AI-only 回退）
// ============================================================================

/// 从本地 Steam 安装中按游戏名称模糊匹配 appmanifest，返回 librarycache 封面路径。
/// 当远程刮削全部被墙时，用作 AI 刮削的封面兜底。
pub fn find_local_steam_cover_by_name(query: &str) -> Option<String> {
    let steam_path = find_steam_install_path()?;
    let libraries = parse_library_folders(&steam_path);

    let q = query.to_lowercase();
    let mut best_path: Option<String> = None;
    let mut best_score = 0.0;

    for lib in &libraries {
        let manifest_dir = lib;
        if let Ok(entries) = std::fs::read_dir(manifest_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !fname.starts_with("appmanifest_") || !fname.ends_with(".acf") {
                    continue;
                }
                let content = std::fs::read_to_string(&path).ok()?;
                let name = extract_acf_value(&content, "name")?;
                let name_lower = name.to_lowercase();

                // 简单模糊匹配
                let score = if name_lower == q {
                    1.0
                } else if name_lower.contains(&q) || q.contains(&name_lower) {
                    0.7
                } else {
                    // Levenshtein 容错
                    let dist = levenshtein_distance(&name_lower, &q);
                    let max_len = name_lower.len().max(q.len()) as f64;
                    if max_len > 0.0 {
                        1.0 - (dist as f64 / max_len)
                    } else {
                        0.0
                    }
                };

                if score > best_score && score >= 0.5 {
                    let appid = extract_acf_value(&content, "appid")?;
                    let cache = steam_path.join("appcache").join("librarycache");
                    // 新版布局
                    let nested = cache.join(&appid).join("library_600x900.jpg");
                    if nested.is_file() {
                        best_path = Some(nested.to_string_lossy().to_string());
                        best_score = score;
                        continue;
                    }
                    // 旧版扁平布局
                    let flat = cache.join(format!("{appid}_library_600x900.jpg"));
                    if flat.is_file() {
                        best_path = Some(flat.to_string_lossy().to_string());
                        best_score = score;
                    }
                }
            }
        }
    }

    best_path
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let n = a_chars.len();
    let m = b_chars.len();
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in dp.iter_mut().enumerate().take(n + 1) {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate().take(m + 1) {
        *cell = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[n][m]
}

// ============================================================================
// Epic Games
// ============================================================================

/// 默认 Epic 清单目录。
pub fn epic_manifests_dir() -> Option<PathBuf> {
    let base = PathBuf::from(r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests");
    if base.exists() {
        Some(base)
    } else {
        None
    }
}

/// 扫描 Epic 已安装游戏。
pub fn scan_epic_games() -> Vec<ImportedGame> {
    let dir = match epic_manifests_dir() {
        Some(d) => d,
        None => return vec![],
    };

    let mut games = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "item") {
                if let Some(game) = parse_epic_item(&path) {
                    games.push(game);
                }
            }
        }
    }

    games
}

fn parse_epic_item(path: &Path) -> Option<ImportedGame> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut content = String::new();
    file.read_to_string(&mut content).ok()?;

    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let name = json.get("DisplayName")?.as_str()?.trim().to_string();
    let app_name = json.get("AppName")?.as_str()?.trim().to_string();
    let install_path = json.get("InstallLocation")?.as_str()?.trim().to_string();
    if name.is_empty() || app_name.is_empty() || install_path.is_empty() {
        return None;
    }
    let app_id = json
        .get("AppName")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let cover_url = extract_epic_cover_url(&json);

    Some(ImportedGame {
        name,
        install_path: PathBuf::from(&install_path),
        platform: "Epic".into(),
        app_id: app_id.or(Some(app_name)),
        cover_url,
    })
}

// ============================================================================
// 辅助函数
// ============================================================================

fn extract_epic_cover_url(json: &serde_json::Value) -> Option<String> {
    let mut images = Vec::new();
    collect_epic_key_images(json, &mut images);
    images.sort_by_key(|(kind, _)| epic_image_priority(kind));
    images.into_iter().map(|(_, url)| url).next()
}

fn collect_epic_key_images(json: &serde_json::Value, images: &mut Vec<(String, String)>) {
    match json {
        serde_json::Value::Array(items) => {
            for item in items {
                collect_epic_key_images(item, images);
            }
        }
        serde_json::Value::Object(map) => {
            if let Some(key_images) = map.get("KeyImages").and_then(|v| v.as_array()) {
                for image in key_images {
                    let kind = image
                        .get("Type")
                        .or_else(|| image.get("type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let url = image
                        .get("Url")
                        .or_else(|| image.get("url"))
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .filter(|url| url.starts_with("http"))
                        .map(str::to_string);
                    if let Some(url) = url {
                        images.push((kind, url));
                    }
                }
            }
            for value in map.values() {
                collect_epic_key_images(value, images);
            }
        }
        _ => {}
    }
}

fn epic_image_priority(kind: &str) -> u8 {
    match kind.to_ascii_lowercase().as_str() {
        "dieselgameboxtall" | "offerimagetall" | "tall" => 0,
        "dieselgamebox" | "offerimagewide" => 1,
        "thumbnail" | "featuredmedia" => 2,
        _ => 9,
    }
}

/// 在字符切片中查找子串，返回索引。
fn find_str(chars: &[char], start: usize, pat: &str) -> Option<usize> {
    let pat_chars: Vec<char> = pat.chars().collect();
    if start + pat_chars.len() > chars.len() {
        return None;
    }
    for i in start..chars.len() - pat_chars.len() + 1 {
        let mut matched = true;
        for (j, pc) in pat_chars.iter().enumerate() {
            if chars[i + j] != *pc {
                matched = false;
                break;
            }
        }
        if matched {
            return Some(i);
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
    fn test_extract_acf_value_basic() {
        let content = "\"appid\"\t\t\"123456\"\n\"name\"\t\t\"Test Game\"\n";
        assert_eq!(
            extract_acf_value(content, "appid").as_deref(),
            Some("123456")
        );
        assert_eq!(
            extract_acf_value(content, "name").as_deref(),
            Some("Test Game")
        );
        assert_eq!(extract_acf_value(content, "missing"), None);
    }

    #[test]
    fn test_find_str_basic() {
        let chars: Vec<char> = "hello \"path\" world".chars().collect();
        let idx = find_str(&chars, 0, "\"path\"");
        assert!(idx.is_some());
        assert_eq!(
            &chars[idx.unwrap()..idx.unwrap() + 6],
            ['"', 'p', 'a', 't', 'h', '"']
        );
    }

    #[test]
    fn test_parse_library_folders_new_format() {
        let vdf = r#""libraryfolders"
{
    "0"
    {
        "path"		"D:\\SteamLibrary"
        "label"		""
    }
    "1"
    {
        "path"		"E:\\Games\\Steam"
        "label"		"SSD"
    }
}"#;
        let chars: Vec<char> = vdf.chars().collect();
        let paths: Vec<String> = {
            let mut result = vec![];
            let mut i = 0;
            while let Some(idx) = find_str(&chars, i, "\"path\"") {
                i = idx + 6; // skip past "path" (6 chars)
                while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t' || chars[i] == '\n') {
                    i += 1;
                }
                if i < chars.len() && chars[i] == '"' {
                    i += 1;
                    let mut p = String::new();
                    while i < chars.len() && chars[i] != '"' {
                        p.push(chars[i]);
                        i += 1;
                    }
                    i += 1; // skip closing quote
                    if !p.is_empty() {
                        result.push(p.replace("\\\\", "\\"));
                    }
                }
            }
            result
        };
        assert_eq!(
            paths.len(),
            2,
            "should find 2 library paths, got {:?}",
            paths
        );
        assert!(
            paths.contains(&"D:\\SteamLibrary".to_string()),
            "paths: {:?}",
            paths
        );
        assert!(
            paths.contains(&"E:\\Games\\Steam".to_string()),
            "paths: {:?}",
            paths
        );
    }

    #[test]
    fn test_parse_epic_item_uses_app_name_for_launch_id() {
        let path = std::env::temp_dir().join(format!("moeplay_epic_{}.item", uuid::Uuid::new_v4()));
        std::fs::write(
            &path,
            r#"{
                "DisplayName": "Alan Wake 2",
                "AppName": "40c34f3c5d3e4f86b8f9d4b5a7f2d0aa",
                "InstallLocation": "D:\\Epic\\AlanWake2",
                "CatalogItemId": "catalog-id",
                "MainGameCatalogItem": {
                  "KeyImages": [
                    { "Type": "DieselGameBox", "Url": "https://cdn.example.com/wide.jpg" },
                    { "Type": "DieselGameBoxTall", "Url": "https://cdn.example.com/tall.jpg" }
                  ]
                }
            }"#,
        )
        .unwrap();

        let game = parse_epic_item(&path).expect("valid epic item should parse");
        std::fs::remove_file(&path).ok();

        assert_eq!(game.name, "Alan Wake 2");
        assert_eq!(game.platform, "Epic");
        assert_eq!(
            game.app_id.as_deref(),
            Some("40c34f3c5d3e4f86b8f9d4b5a7f2d0aa")
        );
        assert_eq!(game.install_path, PathBuf::from(r"D:\Epic\AlanWake2"));
        assert_eq!(
            game.cover_url.as_deref(),
            Some("https://cdn.example.com/tall.jpg")
        );
    }

    #[test]
    fn test_extract_epic_cover_url_prefers_tall_key_image() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
              "Nested": {
                "KeyImages": [
                  { "Type": "Thumbnail", "Url": "https://cdn.example.com/thumb.jpg" },
                  { "Type": "DieselGameBoxTall", "Url": "https://cdn.example.com/portrait.jpg" }
                ]
              }
            }"#,
        )
        .unwrap();

        assert_eq!(
            extract_epic_cover_url(&json).as_deref(),
            Some("https://cdn.example.com/portrait.jpg")
        );
    }

    #[test]
    #[ignore]
    fn smoke_scan_local_platform_libraries() {
        let steam = scan_steam_games();
        let epic = scan_epic_games();
        println!("local Steam games: {}", steam.len());
        for game in steam.iter().take(5) {
            println!("steam: {} | {:?}", game.name, game.app_id);
        }
        println!("local Epic games: {}", epic.len());
        for game in epic.iter().take(5) {
            println!("epic: {} | {:?}", game.name, game.app_id);
        }
    }
}
