// 萌游 MoeGame · 安卓掌机 ROM 库（ES-DE 式）
//
// 与桌面端 `emulators.rs`（扫描 exe + 组装命令行）不同，Android 上的
// 「导入」= 按平台扩展名扫描 ROM 目录，「启动」= 向模拟器应用发送 Intent。
// 启动信息持久化为 `android-intent://<package>?rom=<urlencoded>` 形式的
// launch_uri，由 play.rs::launch_game 在 Android 上识别并走 handheld 插件。
//
// ES-DE 深度整合：若设备上存在 ES-DE 数据目录（gamelists + downloaded_media），
// 扫描时自动读取 gamelist.xml 的显示名 / 最后游玩时间，并优先匹配
// downloaded_media/<system>/covers 下已刮削好的封面。

use crate::db::Database;
use crate::models::Game;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::State;

/// ES-DE / Daijishō 常见 ROM 根目录候选（内置存储）。
const ROM_ROOT_CANDIDATES: &[(&str, &str)] = &[
    ("/storage/emulated/0/ROMs", "ROMs"),
    ("/storage/emulated/0/Roms", "Roms"),
    (
        "/storage/emulated/0/Emulation/roms",
        "ES-DE (Emulation/roms)",
    ),
    ("/storage/emulated/0/Games", "Games"),
    ("/storage/emulated/0/Download", "Download"),
    ("/sdcard/ROMs", "ROMs (sdcard)"),
];

/// 外置卷（SD 卡 / U 盘）下常见的 ROM 目录名。
const EXTERNAL_ROM_SUBDIRS: &[&str] = &["rom", "roms", "ROMs", "Roms", "Emulation/roms"];

/// 平台 → 扩展名。顺序即歧义扩展名的兜底归属优先级。
const PLATFORM_EXTENSIONS: &[(&str, &[&str])] = &[
    ("psp", &["cso", "pbp"]),
    ("ps2", &["chd", "gz"]),
    ("gamecube", &["gcm", "rvz", "wbfs", "ciso"]),
    ("wii", &["rvz", "wbfs"]),
    ("3ds", &["3ds", "cci", "cxi", "cia"]),
    ("nds", &["nds"]),
    ("gba", &["gba", "agb"]),
    ("gbc", &["gbc"]),
    ("gb", &["gb"]),
    ("n64", &["n64", "z64", "v64"]),
    ("snes", &["sfc", "smc"]),
    ("nes", &["nes", "fds", "unf"]),
    ("ps1", &["pbp", "m3u", "mdf", "img"]),
    ("dreamcast", &["cdi", "gdi"]),
    ("saturn", &["mds"]),
    ("md", &["md", "gen", "smd", "32x"]),
    ("switch", &["nsp", "xci", "nca"]),
    ("psvita", &["vpk"]),
    ("arcade", &["zip", "7z"]),
    // 高歧义扩展名最后兜底：iso/bin/cue/chd 依目录名判定，否则归 ps1/psp。
    ("psp", &["iso"]),
    ("ps1", &["bin", "cue", "chd", "iso"]),
];

/// 目录名 → 平台别名（ES-DE 约定按平台分目录存放 ROM）。
/// 覆盖 ES-DE 官方系统目录名（rom/<system>/）。
const PLATFORM_DIR_ALIASES: &[(&str, &str)] = &[
    ("psp", "psp"),
    ("psx", "ps1"),
    ("ps1", "ps1"),
    ("playstation", "ps1"),
    ("ps2", "ps2"),
    ("playstation2", "ps2"),
    ("gba", "gba"),
    ("gameboyadvance", "gba"),
    ("gbc", "gbc"),
    ("gameboycolor", "gbc"),
    ("gb", "gb"),
    ("gameboy", "gb"),
    ("nds", "nds"),
    ("ds", "nds"),
    ("nintendods", "nds"),
    ("3ds", "3ds"),
    ("n3ds", "3ds"),
    ("nintendo3ds", "3ds"),
    ("n64", "n64"),
    ("nintendo64", "n64"),
    ("snes", "snes"),
    ("sfc", "snes"),
    ("supernintendo", "snes"),
    ("nes", "nes"),
    ("fc", "nes"),
    ("famicom", "nes"),
    ("gc", "gamecube"),
    ("gamecube", "gamecube"),
    ("ngc", "gamecube"),
    ("wii", "wii"),
    ("dreamcast", "dreamcast"),
    ("dc", "dreamcast"),
    ("saturn", "saturn"),
    ("ss", "saturn"),
    ("megadrive", "md"),
    ("genesis", "md"),
    ("md", "md"),
    ("sega32x", "md"),
    ("arcade", "arcade"),
    ("mame", "arcade"),
    ("fba", "arcade"),
    ("fbneo", "arcade"),
    ("cps1", "arcade"),
    ("cps2", "arcade"),
    ("cps3", "arcade"),
    ("naomi", "arcade"),
    ("neogeo", "arcade"),
    ("neogeocd", "arcade"),
    ("switch", "switch"),
    ("ns", "switch"),
    ("psvita", "psvita"),
    ("vita", "psvita"),
    ("bbk", "bbk"),
    ("j2me", "j2me"),
    ("msx", "msx"),
    ("pcengine", "pcengine"),
    ("pce", "pcengine"),
    ("pc98", "pc98"),
    ("virtualboy", "virtualboy"),
    ("wonderswancolor", "wsc"),
    ("wonderswan", "wsc"),
    ("ngpc", "ngpc"),
    ("pokemini", "pokemini"),
    ("gameandwatch", "gameandwatch"),
    ("arduboy", "arduboy"),
    ("atari7800", "atari7800"),
    ("supervision", "supervision"),
    ("vectrex", "vectrex"),
    ("zxspectrum", "zxspectrum"),
    ("dos", "dos"),
    ("easyrpg", "easyrpg"),
    ("pico8", "pico8"),
    ("tic80", "tic80"),
    ("onscripter", "onscripter"),
    ("cdimono1", "cdi"),
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RomRootInfo {
    pub path: String,
    pub label: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedRom {
    pub path: String,
    pub name: String,
    pub platform: String,
    pub size_bytes: u64,
    /// 自动匹配到的封面（源路径；导入时复制进应用数据目录）。
    pub cover: Option<String>,
    /// ES-DE gamelist 中的最后游玩时间（已转成 "YYYY-MM-DD HH:MM"）。
    pub last_played: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRomInput {
    pub path: String,
    pub platform: String,
    pub name: Option<String>,
    /// 该 ROM 指定的启动模拟器包名（缺省用顶层 emulator_package 兜底）。
    pub emulator_package: Option<String>,
    pub last_played: Option<String>,
    /// 扫描阶段匹配到的封面（含 ES-DE downloaded_media 结果），导入时复制进应用数据目录。
    pub cover: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRomsResult {
    pub ok: u32,
    pub fail: u32,
}

/// 常见 ROM 根目录及其存在性（前端据此高亮可扫目录）。
/// 动态枚举 /storage 下的外置卷（SD 卡 / U 盘），再附加内置存储候选。
#[tauri::command]
pub fn handheld_rom_roots() -> Vec<RomRootInfo> {
    let mut roots: Vec<RomRootInfo> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if let Ok(entries) = std::fs::read_dir("/storage") {
        for entry in entries.flatten() {
            let volume = entry.file_name().to_string_lossy().to_string();
            // emulated = 内置存储（走静态候选），self 是 shell 视角别名
            if volume == "emulated" || volume == "self" {
                continue;
            }
            for sub in EXTERNAL_ROM_SUBDIRS {
                let dir = entry.path().join(sub);
                let exists = dir.is_dir();
                tracing::info!(volume = %volume, sub, path = %dir.display(), exists, "外置卷 ROM 目录探测");
                if !exists {
                    continue;
                }
                let path = dir.to_string_lossy().to_string();
                if seen.insert(path.clone()) {
                    roots.push(RomRootInfo {
                        label: format!("SD 卡 {sub} ({volume})"),
                        path,
                        exists: true,
                    });
                }
            }
        }
    } else if let Err(e) = std::fs::read_dir("/storage") {
        tracing::warn!(error = %e, "无法枚举 /storage，外置卷探测跳过");
    }

    for (path, label) in ROM_ROOT_CANDIDATES {
        if seen.insert(path.to_string()) {
            roots.push(RomRootInfo {
                path: path.to_string(),
                label: label.to_string(),
                exists: Path::new(path).is_dir(),
            });
        }
    }
    roots
}

/// 递归扫描目录，按扩展名 + 目录名推断平台；
/// 若发现 ES-DE 数据目录，则用 gamelist 显示名 / 最后游玩 + downloaded_media 封面。
#[tauri::command]
pub fn handheld_scan_roms(dir: String) -> Result<Vec<ScannedRom>, String> {
    let root = PathBuf::from(&dir);
    if !root.is_dir() {
        return Err(format!("目录不存在或不可读: {dir}"));
    }
    let mut out = Vec::new();
    scan_dir_recursive(&root, &root, 0, &mut out);

    let mut esde = EsDeContext::discover(&root);
    for rom in &mut out {
        esde.enrich(&root, rom);
    }

    out.sort_by(|a, b| a.platform.cmp(&b.platform).then(a.name.cmp(&b.name)));
    Ok(out)
}

/// 批量入库：launch_uri 记录 android-intent 启动信息，封面复制进应用数据目录。
#[tauri::command]
pub fn handheld_import_roms(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    emulator_package: String,
    roms: Vec<ImportRomInput>,
) -> Result<ImportRomsResult, String> {
    let mut result = ImportRomsResult { ok: 0, fail: 0 };
    for rom in roms {
        match import_one_rom(&app, &db, &emulator_package, &rom) {
            Ok(()) => result.ok += 1,
            Err(e) => {
                tracing::warn!(rom = %rom.path, error = %e, "ROM 导入失败");
                result.fail += 1;
            }
        }
    }
    Ok(result)
}

fn import_one_rom(
    app: &tauri::AppHandle,
    db: &Database,
    fallback_package: &str,
    rom: &ImportRomInput,
) -> Result<(), String> {
    let path = Path::new(&rom.path);
    if !path.is_file() {
        return Err(format!("文件不存在: {}", rom.path));
    }
    let name = rom
        .name
        .clone()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| rom_display_name(path));
    let package = rom
        .emulator_package
        .clone()
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| fallback_package.to_string());

    let mut game = Game::new(name, rom.path.clone());
    game.launch_uri = Some(build_android_intent_uri(&package, &rom.path));
    game.game_type = Some(rom.platform.clone());
    game.metadata.engine = Some(format!("Emulator: {}", rom.platform));
    if let Some(last_played) = rom.last_played.clone().filter(|s| !s.is_empty()) {
        game.play_tracker.last_played = Some(last_played.clone());
        game.last_played = Some(last_played);
    }

    // 扫描阶段已匹配的封面（ES-DE downloaded_media 优先）→ 本地约定目录兜底。
    let cover_src = rom
        .cover
        .clone()
        .filter(|c| !c.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .or_else(|| find_cover_for_rom(path));
    if let Some(cover_src) = cover_src {
        match persist_cover(app, &cover_src) {
            Ok(dest) => {
                let dest_str = dest.to_string_lossy().to_string();
                game.cover = Some(dest_str.clone());
                game.metadata.cover = Some(dest_str);
            }
            Err(e) => tracing::warn!(error = %e, "封面复制失败，跳过封面"),
        }
    }

    db.add_game(game).map(|_| ())
}

/// android-intent://<package>?rom=<urlencoded absolute path>
pub(crate) fn build_android_intent_uri(package: &str, rom_path: &str) -> String {
    format!(
        "android-intent://{}?rom={}",
        package,
        urlencoding::encode(rom_path)
    )
}

/// 解析 android-intent URI → (package, rom_path)。play.rs 在 Android 启动时使用。
#[cfg(target_os = "android")]
pub(crate) fn parse_android_intent_uri(uri: &str) -> Option<(String, String)> {
    let rest = uri.strip_prefix("android-intent://")?;
    let (package, query) = rest.split_once('?').unwrap_or((rest, ""));
    if package.is_empty() {
        return None;
    }
    for pair in query.split('&') {
        if let Some(value) = pair.strip_prefix("rom=") {
            let decoded = urlencoding::decode(value).ok()?.into_owned();
            if !decoded.is_empty() {
                return Some((package.to_string(), decoded));
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// 启动候选链解析（Android）
//
// 「打开游戏就跳转进去」：按平台 + ROM 所在系统目录（sysdir）解析出有序的
// 启动候选，Kotlin 侧按序 startActivity 直到成功。直启（RetroArch 核心 /
// 独立模拟器 VIEW）优先，主界面唤起兜底；空链 = 该平台暂不支持。
// ---------------------------------------------------------------------------

pub use tauri_plugin_handheld::LaunchCandidate;

/// RetroArch（64 位）默认包名与游戏启动 Activity（真机验证）。
pub(crate) const RETROARCH_PACKAGE: &str = "com.retroarch.aarch64";
const RETROARCH_ACTIVITY: &str = "com.retroarch.browser.retroactivity.RetroActivityFuture";

fn candidate_view(package: &str) -> LaunchCandidate {
    LaunchCandidate {
        mode: "view".into(),
        package: package.into(),
        activity: None,
        core: None,
    }
}

fn candidate_menu(package: &str) -> LaunchCandidate {
    LaunchCandidate {
        mode: "menu".into(),
        package: package.into(),
        activity: None,
        core: None,
    }
}

fn candidate_retroarch(package: &str, core: &str) -> LaunchCandidate {
    LaunchCandidate {
        mode: "retroarch".into(),
        package: package.into(),
        activity: Some(RETROARCH_ACTIVITY.into()),
        core: Some(core.into()),
    }
}

/// sysdir（ROM 系统目录名，小写）→ RetroArch 核心文件名。用于细分聚合平台：
/// arcade 聚合了 cps*/fbneo/mame/naomi/neogeo*，md 含 sega32x。
/// 均为真机逐一截图验证过的可用核心（裸文件名，RetroArch 自动解析内部核心目录）。
fn core_from_sysdir(sysdir: &str) -> Option<&'static str> {
    match sysdir {
        "cps1" | "cps2" | "cps3" | "fbneo" | "neogeo" => Some("fbneo_plus_libretro.so"),
        "mame" => Some("mame2003_plus_libretro_android.so"),
        "naomi" | "atomiswave" => Some("flycast_libretro_android.so"),
        "neogeocd" => Some("neocd_libretro_android.so"),
        "sega32x" | "32x" => Some("picodrive_libretro_android.so"),
        _ => None,
    }
}

/// 平台 → RetroArch 核心文件名兜底映射（真机验证）。
fn core_from_type(game_type: &str) -> Option<&'static str> {
    match game_type {
        "nes" => Some("fceumm_libretro_android.so"),
        "snes" => Some("snes9x_libretro_android.so"),
        "gb" | "gbc" => Some("gambatte_libretro_android.so"),
        "gba" => Some("mgba_libretro_android.so"),
        "md" => Some("genesis_plus_gx_libretro_android.so"),
        "ps1" => Some("pcsx_rearmed_libretro_android.so"),
        "dreamcast" => Some("flycast_libretro_android.so"),
        // arcade 无 sysdir 命中时兜底 fbneo（覆盖率最高的街机核心）
        "arcade" => Some("fbneo_plus_libretro.so"),
        "msx" => Some("fmsx_libretro_android.so"),
        "pcengine" => Some("mednafen_pce_fast_libretro_android.so"),
        "pc98" => Some("np2kai_libretro_android.so"),
        "virtualboy" => Some("beetle_vb_libretro_android.so"),
        "wsc" => Some("mednafen_wswan_libretro_android.so"),
        "pokemini" => Some("pokemini_libretro_android.so"),
        "gameandwatch" => Some("gw_libretro_android.so"),
        "arduboy" => Some("ardens_libretro_android.so"),
        "atari7800" => Some("prosystem_libretro_android.so"),
        "supervision" => Some("potator_libretro_android.so"),
        "vectrex" => Some("vecx_libretro_android.so"),
        "zxspectrum" => Some("fuse_libretro_android.so"),
        "dos" => Some("dosbox_pure_libretro_android.so"),
        "easyrpg" => Some("easyrpg_libretro_android.so"),
        "pico8" => Some("fake08_libretro.so"),
        "tic80" => Some("tic80_libretro_android.so"),
        "bbk" => Some("gam4980_libretro.so"),
        // ngpc / cdi 核心已真机复验可用（mednafen_ngp / same_cdi）。
        "ngpc" => Some("mednafen_ngp_libretro_android.so"),
        "cdi" => Some("same_cdi_libretro_android.so"),
        // saturn 核心存在于设备核心目录，但无 ROM 可复验，按 ES-DE 常规映射给出。
        "saturn" => Some("mednafen_saturn_libretro_android.so"),
        _ => None,
    }
}

/// 解析启动候选链。返回空 Vec = 该平台暂不支持直接启动。
///
/// - 独立模拟器直启已真机验证：PPSSPP / DraStic / Mupen64Plus FZ / J2ME Loader
///   均可 ACTION_VIEW + content URI 直进游戏；AetherSX2 / DuckStation 无可用
///   VIEW 入口，退回主界面；ngpc / cdi 无可用 RetroArch 核心，不支持。
/// - stored_package 非 RetroArch 且非空时，链首追加该包 VIEW 候选（尊重用户
///   在导入/编辑时显式选择的模拟器）。
pub fn resolve_launch_candidates(
    game_type: &str,
    rom_path: &str,
    stored_package: &str,
) -> Vec<LaunchCandidate> {
    let parent_dir = Path::new(rom_path)
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    // ROM 可能位于系统目录下一级的游戏子目录（如 rom/psx/<游戏>/game.chd）。
    let grand_dir = Path::new(rom_path)
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    let core = core_from_sysdir(&parent_dir)
        .or_else(|| core_from_sysdir(&grand_dir))
        .or_else(|| core_from_type(game_type));

    let ra_pkg = if stored_package.starts_with("com.retroarch") {
        stored_package
    } else {
        RETROARCH_PACKAGE
    };

    let mut chain: Vec<LaunchCandidate> = Vec::new();
    if !stored_package.is_empty() && !stored_package.starts_with("com.retroarch") {
        chain.push(candidate_view(stored_package));
    }

    match game_type {
        "psp" => {
            chain.push(candidate_view("org.ppsspp.ppsspp"));
            chain.push(candidate_menu("org.ppsspp.ppsspp"));
        }
        "nds" => {
            chain.push(candidate_view("com.dsemu.drastic"));
            chain.push(candidate_retroarch(ra_pkg, "melonds_libretro_android.so"));
            chain.push(candidate_menu("com.dsemu.drastic"));
        }
        "n64" => {
            // RetroArch mupen64plus_next（gles3 变体）已真机验证直进游戏；
            // 独立版 Mupen64 只认 SAF URI（FileProvider 会落到游戏库），故仅作菜单兜底。
            chain.push(candidate_retroarch(
                ra_pkg,
                "mupen64plus_next_gles3_libretro_android.so",
            ));
            chain.push(candidate_menu("org.mupen64plusae.v3.fzurita.pro"));
            chain.push(candidate_menu("org.mupen64plusae.v3.fzurita"));
            chain.push(candidate_menu(ra_pkg));
        }
        "j2me" => {
            chain.push(candidate_view("ru.playsoftware.j2meloader"));
            chain.push(candidate_menu("ru.playsoftware.j2meloader"));
        }
        "ps2" => {
            // AetherSX2 v1.5 无可用直启入口（已验证），退回主界面。
            chain.push(candidate_menu("xyz.aethersx2.android"));
        }
        "onscripter" => {
            chain.push(candidate_menu("com.onscripter.plus"));
        }
        // 无可用核心 / 无可用直启入口的平台：明确不支持。
        "gamecube" | "wii" | "3ds" | "switch" | "psvita" => {}
        _ => {
            if let Some(core) = core {
                chain.push(candidate_retroarch(ra_pkg, core));
                chain.push(candidate_menu(ra_pkg));
            }
        }
    }

    // 去重（保序）：stored 包与平台默认包可能重复。
    let mut seen = std::collections::HashSet::new();
    chain.retain(|c| seen.insert((c.mode.clone(), c.package.clone(), c.core.clone())));
    chain
}

fn scan_dir_recursive(root: &Path, dir: &Path, depth: usize, out: &mut Vec<ScannedRom>) {
    if depth > 6 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_dir() {
            scan_dir_recursive(root, &path, depth + 1, out);
            continue;
        }
        let Some(ext) = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
        else {
            continue;
        };
        let Some(platform) = infer_platform(&path, &ext) else {
            continue;
        };
        let cover = find_cover_for_rom(&path).map(|p| p.to_string_lossy().to_string());
        out.push(ScannedRom {
            path: path.to_string_lossy().to_string(),
            name: rom_display_name(&path),
            platform,
            size_bytes: meta.len(),
            cover,
            last_played: None,
        });
    }
    let _ = root;
}

/// 目录名优先（ES-DE 约定），扩展名兜底。
fn infer_platform(path: &Path, ext: &str) -> Option<String> {
    let dir_platform = path.parent().and_then(|parent| {
        parent
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|name| {
                let normalized = name.to_ascii_lowercase().replace(['-', '_', ' '], "");
                PLATFORM_DIR_ALIASES
                    .iter()
                    .find(|(alias, _)| normalized == alias.replace('-', ""))
                    .map(|(_, platform)| platform.to_string())
            })
    });

    // 扩展名是否属于该目录平台？不属于则目录名无效（防误判，如 "gba hacks"）。
    let ext_platforms: Vec<&str> = PLATFORM_EXTENSIONS
        .iter()
        .filter(|(_, exts)| exts.contains(&ext))
        .map(|(platform, _)| *platform)
        .collect();
    if ext_platforms.is_empty() {
        return None;
    }
    if let Some(platform) = dir_platform {
        if ext_platforms.contains(&platform.as_str()) {
            return Some(platform);
        }
    }
    ext_platforms.first().map(|p| p.to_string())
}

fn rom_display_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("未知游戏")
        .replace(['_', '.'], " ")
        .trim()
        .to_string()
}

// ---------------------------------------------------------------------------
// ES-DE 数据目录整合（gamelists / downloaded_media）
// ---------------------------------------------------------------------------

/// gamelist.xml 单条记录的子集。
#[derive(Debug, Default, Clone)]
struct GamelistMeta {
    name: Option<String>,
    last_played: Option<String>,
}

/// 设备上发现的 ES-DE 数据目录集合（按优先级排列）。
struct EsDeContext {
    roots: Vec<PathBuf>,
    /// system 目录名 → gamelist（键为 "./<相对路径>" 与纯文件名两种形态）。
    gamelists: HashMap<String, HashMap<String, GamelistMeta>>,
}

impl EsDeContext {
    /// 依据扫描根目录发现 ES-DE 数据目录：
    ///   /storage/emulated/0/ES-DE、/sdcard/ES-DE、与 ROM 根同级的 ES-DE。
    fn discover(scan_root: &Path) -> Self {
        let mut candidates: Vec<PathBuf> = vec![
            PathBuf::from("/storage/emulated/0/ES-DE"),
            PathBuf::from("/sdcard/ES-DE"),
        ];
        if let Some(parent) = scan_root.parent() {
            candidates.push(parent.join("ES-DE"));
            // ROM 根位于 <卷>/rom 时，其祖父级（卷根）也常见 ES-DE 目录
            if let Some(grand) = parent.parent() {
                candidates.push(grand.join("ES-DE"));
            }
        }
        let mut seen = std::collections::HashSet::new();
        let roots = candidates
            .into_iter()
            .filter(|p| p.is_dir())
            .filter(|p| seen.insert(p.clone()))
            .collect();
        Self {
            roots,
            gamelists: HashMap::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    /// 用 ES-DE 数据补全单个扫描结果（显示名 / 最后游玩 / 封面）。
    fn enrich(&mut self, scan_root: &Path, rom: &mut ScannedRom) {
        if self.is_empty() {
            return;
        }
        let rom_path = PathBuf::from(&rom.path);
        let Some(system) = system_dir_name(scan_root, &rom_path) else {
            return;
        };

        // 1) gamelist：显示名 + 最后游玩
        let gamelist = self.gamelist_for(&system);
        if let Some(gamelist) = gamelist {
            let rel = relative_rom_key(&rom_path, &system);
            let file_key = rom_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());
            let meta = rel
                .as_ref()
                .and_then(|k| gamelist.get(k))
                .or_else(|| file_key.as_ref().and_then(|k| gamelist.get(k)));
            if let Some(meta) = meta {
                if let Some(name) = &meta.name {
                    if !name.trim().is_empty() {
                        rom.name = name.trim().to_string();
                    }
                }
                if let Some(last) = &meta.last_played {
                    rom.last_played = Some(format_esde_lastplayed(last));
                }
            }
        }

        // 2) downloaded_media 封面（优先级高于同目录图片）
        if rom.cover.is_none() {
            if let Some(stem) = rom_path.file_stem().and_then(|s| s.to_str()) {
                for root in &self.roots {
                    let covers = root.join("downloaded_media").join(&system).join("covers");
                    for ext in ["png", "jpg", "jpeg", "webp"] {
                        let candidate = covers.join(stem).with_extension(ext);
                        if candidate.is_file() {
                            rom.cover = Some(candidate.to_string_lossy().to_string());
                            return;
                        }
                    }
                }
            }
        }
    }

    /// 按系统目录名懒加载 gamelist.xml（可能来自多个 ES-DE root，前者优先）。
    fn gamelist_for(&mut self, system: &str) -> Option<&HashMap<String, GamelistMeta>> {
        if !self.gamelists.contains_key(system) {
            let mut merged: HashMap<String, GamelistMeta> = HashMap::new();
            for root in &self.roots {
                let file = root.join("gamelists").join(system).join("gamelist.xml");
                if let Ok(text) = std::fs::read_to_string(&file) {
                    for (key, meta) in parse_gamelist_xml(&text) {
                        merged.entry(key).or_insert(meta);
                    }
                }
            }
            self.gamelists.insert(system.to_string(), merged);
        }
        self.gamelists.get(system).filter(|m| !m.is_empty())
    }
}

/// ROM 相对扫描根的第一层目录名（ES-DE 系统目录，如 gba / psx）。
fn system_dir_name(scan_root: &Path, rom: &Path) -> Option<String> {
    let rel = rom.strip_prefix(scan_root).ok()?;
    let mut components = rel.components();
    let first = components.next()?;
    // ROM 直接放在根目录下时无系统目录
    components.next()?;
    Some(first.as_os_str().to_string_lossy().to_string())
}

/// gamelist 键形态："./<系统目录内相对路径>"（正斜杠）。
fn relative_rom_key(rom: &Path, _system: &str) -> Option<String> {
    let file = rom.file_name()?.to_str()?;
    let parent = rom.parent()?.file_name().and_then(|n| n.to_str());
    Some(match parent {
        // 多级子目录（如 psx 的 .m3u 在子目录）保留相对路径
        Some(dir) if dir != _system => format!("./{dir}/{file}"),
        _ => format!("./{file}"),
    })
}

/// 手工解析 ES-DE gamelist.xml：返回 键(./路径 或 文件名) → 元数据。
/// 只取 <game> 块内的 <path>/<name>/<lastplayed>；实体解码 &amp; &lt; &gt; &quot; &apos;。
fn parse_gamelist_xml(text: &str) -> Vec<(String, GamelistMeta)> {
    fn tag<'a>(block: &'a str, name: &str) -> Option<&'a str> {
        let open = format!("<{name}>");
        let close = format!("</{name}>");
        let start = block.find(&open)? + open.len();
        let end = block[start..].find(&close)? + start;
        Some(block[start..end].trim())
    }
    fn unescape(s: &str) -> String {
        s.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
    }

    let mut out = Vec::new();
    for block in text.split("<game>").skip(1) {
        let Some(path) = tag(block, "path").map(unescape) else {
            continue;
        };
        let meta = GamelistMeta {
            name: tag(block, "name").map(unescape),
            last_played: tag(block, "lastplayed").map(unescape),
        };
        let key = path.clone();
        out.push((key.clone(), meta.clone()));
        // 文件名兜底键（兼容子目录条目）
        if let Some(file) = key.rsplit('/').next() {
            if file != key.trim_start_matches("./") {
                out.push((file.to_string(), meta));
            }
        }
    }
    out
}

/// ES-DE lastplayed 格式 20240628T144730 → 2024-06-28 14:47。
fn format_esde_lastplayed(raw: &str) -> String {
    let raw = raw.trim();
    if raw.len() >= 13 && raw.chars().nth(8) == Some('T') {
        let (date, time) = raw.split_at(8);
        let time = &time[1..];
        if date.len() == 8 && time.len() >= 4 {
            return format!(
                "{}-{}-{} {}:{}",
                &date[0..4],
                &date[4..6],
                &date[6..8],
                &time[0..2],
                &time[2..4]
            );
        }
    }
    raw.to_string()
}

/// 按 ES-DE 媒体约定找封面：同目录同名图片，或邻近 boxart/covers/images/media 目录。
fn find_cover_for_rom(rom: &Path) -> Option<PathBuf> {
    const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp"];
    let stem = rom.file_stem()?;
    let parent = rom.parent()?;

    let candidates: Vec<PathBuf> = std::iter::once(parent.to_path_buf())
        .chain([
            parent.join("covers"),
            parent.join("boxart"),
            parent.join("images"),
            parent.join("media").join("boxart"),
            parent.join("media").join("covers"),
            // ES-DE 下载的媒体默认落在与 ROM 目录同级的 ../media/<system>/boxart
            parent.join("..").join("media").join("boxart"),
        ])
        .collect();

    for dir in candidates {
        for ext in IMAGE_EXTS {
            let candidate = dir.join(stem).with_extension(ext);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// 把封面复制进应用数据目录 covers/，保证 asset protocol scope 内可读。
fn persist_cover(app: &tauri::AppHandle, src: &Path) -> Result<PathBuf, String> {
    use tauri::Manager;
    let covers_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("moeplay")
        .join("covers");
    std::fs::create_dir_all(&covers_dir).map_err(|e| e.to_string())?;

    let filename = src
        .file_name()
        .ok_or_else(|| "封面文件名无效".to_string())?;
    // 同名封面（不同平台的同名游戏）加哈希后缀防覆盖。
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::{Hash, Hasher};
    src.to_string_lossy().hash(&mut hasher);
    let hashed = format!(
        "{}-{:x}.{}",
        src.file_stem().and_then(|s| s.to_str()).unwrap_or("cover"),
        hasher.finish() % 0x1_0000_0000u64,
        src.extension().and_then(|e| e.to_str()).unwrap_or("png")
    );
    let _ = filename;
    let dest = covers_dir.join(hashed);
    std::fs::copy(src, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_platform_from_extension_without_dir_hint() {
        let path = Path::new("/storage/emulated/0/ROMs/Game.nsp");
        assert_eq!(infer_platform(path, "nsp"), Some("switch".to_string()));
    }

    #[test]
    fn directory_alias_disambiguates_shared_extensions() {
        let psp = Path::new("/storage/emulated/0/ROMs/PSP/Game.iso");
        assert_eq!(infer_platform(psp, "iso"), Some("psp".to_string()));
        let psx = Path::new("/storage/emulated/0/ROMs/psx/Game.iso");
        assert_eq!(infer_platform(psx, "iso"), Some("ps1".to_string()));
    }

    #[test]
    fn esde_system_dirs_map_to_platforms() {
        let gba = Path::new("/storage/ABCD/rom/gba/口袋妖怪.gba");
        assert_eq!(infer_platform(gba, "gba"), Some("gba".to_string()));
        let psx = Path::new("/storage/ABCD/rom/psx/生化危机.chd");
        assert_eq!(infer_platform(psx, "chd"), Some("ps1".to_string()));
        let fbneo = Path::new("/storage/ABCD/rom/fbneo/拳皇97.zip");
        assert_eq!(infer_platform(fbneo, "zip"), Some("arcade".to_string()));
        let md = Path::new("/storage/ABCD/rom/megadrive/索尼克.md");
        assert_eq!(infer_platform(md, "md"), Some("md".to_string()));
    }

    #[test]
    fn dir_alias_without_matching_extension_falls_back_to_extension() {
        // "gba" 目录里的 .zip 不归 gba（zip 属 arcade），按扩展名兜底。
        let path = Path::new("/storage/emulated/0/ROMs/gba/pack.zip");
        assert_eq!(infer_platform(path, "zip"), Some("arcade".to_string()));
    }

    #[test]
    fn unknown_extension_is_rejected() {
        let path = Path::new("/storage/emulated/0/ROMs/psp/readme.txt");
        assert_eq!(infer_platform(path, "txt"), None);
    }

    #[test]
    fn builds_intent_uri_with_urlencoded_rom_path() {
        let uri = build_android_intent_uri(
            "org.ppsspp.ppsspp",
            "/storage/emulated/0/ROMs/psp/我的 游戏.iso",
        );
        assert!(uri.starts_with("android-intent://org.ppsspp.ppsspp?rom="));
        assert!(uri.contains("%2Fstorage%2Femulated%2F0"));
    }

    #[test]
    fn parses_gamelist_xml_entries() {
        let xml = r#"<?xml version="1.0"?>
<gameList>
    <game>
        <path>./口袋妖怪 红宝石.gba</path>
        <name>口袋妖怪 红宝石</name>
        <playcount>1</playcount>
        <lastplayed>20240628T144730</lastplayed>
    </game>
    <game>
        <path>./sub/生化危机 (汉化).m3u</path>
        <name>生化危机 &amp; 维罗妮卡</name>
    </game>
</gameList>"#;
        let entries = parse_gamelist_xml(xml);
        let map: HashMap<String, GamelistMeta> = entries.into_iter().collect();
        let ruby = map.get("./口袋妖怪 红宝石.gba").expect("ruby entry");
        assert_eq!(ruby.name.as_deref(), Some("口袋妖怪 红宝石"));
        assert_eq!(ruby.last_played.as_deref(), Some("20240628T144730"));
        let bio = map.get("./sub/生化危机 (汉化).m3u").expect("bio entry");
        assert_eq!(bio.name.as_deref(), Some("生化危机 & 维罗妮卡"));
        // 文件名兜底键
        assert!(map.contains_key("生化危机 (汉化).m3u"));
    }

    #[test]
    fn formats_esde_lastplayed_timestamp() {
        assert_eq!(
            format_esde_lastplayed("20240628T144730"),
            "2024-06-28 14:47"
        );
        assert_eq!(format_esde_lastplayed("未知"), "未知");
    }

    #[test]
    fn esde_context_enriches_from_gamelist_and_media() {
        let tmp = std::env::temp_dir().join(format!("moeplay-esde-test-{}", std::process::id()));
        let rom_root = tmp.join("rom");
        let rom_dir = rom_root.join("gba");
        std::fs::create_dir_all(&rom_dir).unwrap();
        let rom_path = rom_dir.join("ruby.gba");
        std::fs::write(&rom_path, b"rom").unwrap();

        let esde = tmp.join("ES-DE");
        std::fs::create_dir_all(esde.join("gamelists").join("gba")).unwrap();
        std::fs::write(
            esde.join("gamelists").join("gba").join("gamelist.xml"),
            r#"<gameList><game><path>./ruby.gba</path><name>口袋妖怪 红宝石</name><lastplayed>20240628T144730</lastplayed></game></gameList>"#,
        )
        .unwrap();
        let cover_dir = esde.join("downloaded_media").join("gba").join("covers");
        std::fs::create_dir_all(&cover_dir).unwrap();
        std::fs::write(cover_dir.join("ruby.png"), b"png").unwrap();

        let mut ctx = EsDeContext {
            roots: vec![esde.clone()],
            gamelists: HashMap::new(),
        };
        let mut rom = ScannedRom {
            path: rom_path.to_string_lossy().to_string(),
            name: "ruby".to_string(),
            platform: "gba".to_string(),
            size_bytes: 3,
            cover: None,
            last_played: None,
        };
        ctx.enrich(&rom_root, &mut rom);
        assert_eq!(rom.name, "口袋妖怪 红宝石");
        assert_eq!(rom.last_played.as_deref(), Some("2024-06-28 14:47"));
        assert!(rom.cover.unwrap().ends_with("ruby.png"));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn resolves_retroarch_core_by_sysdir_for_arcade_and_md() {
        let cps = resolve_launch_candidates(
            "arcade",
            "/storage/AB/rom/cps2/1941.zip",
            "com.retroarch.aarch64",
        );
        assert_eq!(cps[0].mode, "retroarch");
        assert_eq!(cps[0].core.as_deref(), Some("fbneo_plus_libretro.so"));
        assert_eq!(cps[0].package, "com.retroarch.aarch64");
        assert_eq!(
            cps[0].activity.as_deref(),
            Some("com.retroarch.browser.retroactivity.RetroActivityFuture")
        );

        let mame = resolve_launch_candidates(
            "arcade",
            "/storage/AB/rom/mame/pacman.zip",
            "com.retroarch.aarch64",
        );
        assert_eq!(
            mame[0].core.as_deref(),
            Some("mame2003_plus_libretro_android.so")
        );

        let x32 = resolve_launch_candidates("md", "/storage/AB/rom/sega32x/virtua.32x", "");
        assert_eq!(
            x32[0].core.as_deref(),
            Some("picodrive_libretro_android.so")
        );

        let md = resolve_launch_candidates("md", "/storage/AB/rom/megadrive/sonic.md", "");
        assert_eq!(
            md[0].core.as_deref(),
            Some("genesis_plus_gx_libretro_android.so")
        );

        // 游戏子目录情况：sysdir 向上多查一层。
        let nested = resolve_launch_candidates(
            "arcade",
            "/storage/AB/rom/neogeo/拳皇97/kof97.zip",
            "com.retroarch.aarch64",
        );
        assert_eq!(nested[0].core.as_deref(), Some("fbneo_plus_libretro.so"));
    }

    #[test]
    fn nds_chain_prefers_drastic_view_then_melonds_fallback() {
        let c =
            resolve_launch_candidates("nds", "/storage/AB/rom/nds/a.nds", "com.retroarch.aarch64");
        assert_eq!(c[0].mode, "view");
        assert_eq!(c[0].package, "com.dsemu.drastic");
        assert!(c
            .iter()
            .any(|x| x.mode == "retroarch"
                && x.core.as_deref() == Some("melonds_libretro_android.so")));
        assert!(c
            .iter()
            .any(|x| x.mode == "menu" && x.package == "com.dsemu.drastic"));
    }

    #[test]
    fn psp_chain_uses_ppsspp_view_then_menu() {
        let c =
            resolve_launch_candidates("psp", "/storage/AB/rom/psp/a.iso", "com.retroarch.aarch64");
        assert_eq!(c[0].mode, "view");
        assert_eq!(c[0].package, "org.ppsspp.ppsspp");
        assert_eq!(c[1].mode, "menu");
    }

    #[test]
    fn ps1_chain_uses_verified_pcsx_core() {
        let c = resolve_launch_candidates(
            "ps1",
            "/storage/AB/rom/psx/bio.chd",
            "com.retroarch.aarch64",
        );
        assert_eq!(c[0].mode, "retroarch");
        assert_eq!(
            c[0].core.as_deref(),
            Some("pcsx_rearmed_libretro_android.so")
        );
    }

    #[test]
    fn n64_chain_prefers_verified_gles3_core_then_standalone_menu() {
        let c = resolve_launch_candidates("n64", "/rom/n64/zelda.z64", "com.retroarch.aarch64");
        assert_eq!(c[0].mode, "retroarch");
        assert_eq!(
            c[0].core.as_deref(),
            Some("mupen64plus_next_gles3_libretro_android.so")
        );
        assert!(c
            .iter()
            .any(|x| x.mode == "menu" && x.package == "org.mupen64plusae.v3.fzurita.pro"));
    }

    #[test]
    fn ngpc_and_cdi_use_reverified_cores() {
        let ngpc = resolve_launch_candidates("ngpc", "/rom/ngpc/svc.zip", "com.retroarch.aarch64");
        assert_eq!(ngpc[0].mode, "retroarch");
        assert_eq!(
            ngpc[0].core.as_deref(),
            Some("mednafen_ngp_libretro_android.so")
        );
        let cdi =
            resolve_launch_candidates("cdi", "/rom/cdimono1/zelda.chd", "com.retroarch.aarch64");
        assert_eq!(cdi[0].core.as_deref(), Some("same_cdi_libretro_android.so"));
    }

    #[test]
    fn unsupported_platform_returns_empty_chain() {
        assert!(resolve_launch_candidates("psvita", "/rom/psvita/a.vpk", "").is_empty());
        assert!(resolve_launch_candidates("switch", "/rom/switch/a.nsp", "").is_empty());
    }

    #[test]
    fn stored_non_retroarch_package_gets_view_candidate_first() {
        let c = resolve_launch_candidates("gba", "/rom/gba/a.gba", "it.dbtecno.pizzaboygbapro");
        assert_eq!(c[0].mode, "view");
        assert_eq!(c[0].package, "it.dbtecno.pizzaboygbapro");
        // RetroArch 核心候选仍在后方兜底。
        assert!(c.iter().any(
            |x| x.mode == "retroarch" && x.core.as_deref() == Some("mgba_libretro_android.so")
        ));
    }

    #[test]
    fn candidates_dedup_when_stored_package_matches_default() {
        let c = resolve_launch_candidates("psp", "/rom/psp/a.iso", "org.ppsspp.ppsspp");
        let view_count = c
            .iter()
            .filter(|x| x.mode == "view" && x.package == "org.ppsspp.ppsspp")
            .count();
        assert_eq!(view_count, 1);
    }
}
