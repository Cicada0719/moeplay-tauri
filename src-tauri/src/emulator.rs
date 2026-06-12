// 萌游 MoeGame · 模拟器检测与 ROM 导入
//
// 从原版 C# Playnite 移植的模拟器导入管线：
//   ① search_emulators() — 扫描磁盘正则匹配 exe 名 → 检测依赖文件 → 返回已安装的模拟器
//   ② scan_roms() — 按扩展名扫描目录中的 ROM 文件
//   ③ builtin_emulator_definitions() — 内置 20+ 模拟器 YAML 定义
//   ④ import_rom_game() — 一键入库（自动组装启动参数）

use serde::{Deserialize, Serialize};
use std::path::Path;

// ============================================================================
// 数据类型
// ============================================================================

/// 模拟器定义（对应 emulator.yaml）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorDefinition {
    pub id: String,
    pub name: String,
    pub website: Option<String>,
    pub profiles: Vec<EmulatorProfile>,
}

/// 模拟器配置集
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorProfile {
    pub name: String,
    pub startup_arguments: Option<String>,
    pub platforms: Vec<String>,
    pub image_extensions: Vec<String>,
    pub startup_executable: Option<String>,
    pub profile_files: Option<Vec<String>>,
}

/// 扫描到的已安装模拟器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedEmulator {
    pub id: String,
    pub name: String,
    pub install_dir: String,
    pub executable: String,
    pub profiles: Vec<ScannedProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedProfile {
    pub profile_name: String,
    pub platform_ids: Vec<String>,
    pub image_extensions: Vec<String>,
    pub startup_arguments: Option<String>,
}

/// 扫描到的 ROM 文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RomFile {
    pub path: String,
    pub filename: String,
    pub name: String,
    pub extension: String,
    pub size_bytes: u64,
    pub platform: Option<String>,
}

// ============================================================================
// 模拟器检测
// ============================================================================

pub fn search_emulators(
    search_paths: &[String],
    definitions: &[EmulatorDefinition],
) -> Vec<ScannedEmulator> {
    let mut found = Vec::new();
    for sp in search_paths {
        let path = Path::new(sp);
        if !path.exists() {
            continue;
        }
        scan_dir_for_emulators(path, definitions, &mut found, 0);
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found.dedup_by(|a, b| a.id == b.id && a.install_dir == b.install_dir);
    found
}

fn scan_dir_for_emulators(
    dir: &Path,
    definitions: &[EmulatorDefinition],
    result: &mut Vec<ScannedEmulator>,
    depth: usize,
) {
    if depth > 2 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    // Skip system/hidden dirs that can't contain emulators
    const SKIP_DIRS: &[&str] = &[
        "Windows",
        "windows",
        "$Recycle.Bin",
        "$WinREAgent",
        "System Volume Information",
        "PerfLogs",
        "Recovery",
        "Config.Msi",
        "MSOCache",
        "node_modules",
        ".git",
    ];

    for entry in entries.flatten() {
        let path = entry.path();
        let fname = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if path.is_dir() {
            if fname.starts_with('.') || SKIP_DIRS.contains(&fname.as_str()) {
                continue;
            }
            scan_dir_for_emulators(&path, definitions, result, depth + 1);
        } else if path.is_file() && fname.ends_with(".exe") {
            for def in definitions {
                for profile in &def.profiles {
                    let exe_pat = profile.startup_executable.as_deref().unwrap_or("");
                    if exe_pat.is_empty() {
                        continue;
                    }

                    if let Ok(re) = regex::Regex::new(&format!(
                        "(?i){}",
                        regex::escape(exe_pat).replace("\\*", ".*")
                    )) {
                        if !re.is_match(&fname) {
                            continue;
                        }
                    } else {
                        if !fname.to_lowercase().contains(
                            &exe_pat
                                .to_lowercase()
                                .replace("^", "")
                                .replace("$", "")
                                .replace(".*", "")
                                .replace("\\", ""),
                        ) {
                            continue;
                        }
                    }

                    // Check required profile files
                    if let Some(ref req_files) = profile.profile_files {
                        let parent = path.parent().unwrap_or(dir);
                        if !req_files.iter().all(|rf| parent.join(rf).exists()) {
                            continue;
                        }
                    }

                    let parent = path.parent().unwrap_or(dir);
                    let install_dir = parent.to_string_lossy().to_string();
                    let exe_path = path.to_string_lossy().to_string();

                    if let Some(em) = result
                        .iter_mut()
                        .find(|e| e.id == def.id && e.install_dir == install_dir)
                    {
                        if !em.profiles.iter().any(|p| p.profile_name == profile.name) {
                            em.profiles.push(ScannedProfile {
                                profile_name: profile.name.clone(),
                                platform_ids: profile.platforms.clone(),
                                image_extensions: profile.image_extensions.clone(),
                                startup_arguments: profile.startup_arguments.clone(),
                            });
                        }
                    } else {
                        result.push(ScannedEmulator {
                            id: def.id.clone(),
                            name: def.name.clone(),
                            install_dir,
                            executable: exe_path,
                            profiles: vec![ScannedProfile {
                                profile_name: profile.name.clone(),
                                platform_ids: profile.platforms.clone(),
                                image_extensions: profile.image_extensions.clone(),
                                startup_arguments: profile.startup_arguments.clone(),
                            }],
                        });
                    }
                    break;
                }
            }
        }
    }
}

// ============================================================================
// ROM 扫描
// ============================================================================

pub fn scan_roms(dir: &str, extensions: &[String], recursive: bool) -> Vec<RomFile> {
    let mut roms = Vec::new();
    let path = Path::new(dir);
    if !path.exists() {
        return roms;
    }
    scan_dir_for_roms(path, extensions, recursive, 0, &mut roms);
    roms.sort_by(|a, b| a.name.cmp(&b.name));
    roms
}

fn scan_dir_for_roms(
    dir: &Path,
    extensions: &[String],
    recursive: bool,
    depth: usize,
    result: &mut Vec<RomFile>,
) {
    if depth > 5 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if recursive {
                scan_dir_for_roms(&path, extensions, recursive, depth + 1, result);
            }
        } else if path.is_file() {
            let fname = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                if extensions
                    .iter()
                    .any(|e| e.eq_ignore_ascii_case(&ext_lower))
                {
                    let name = path
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&fname)
                        .to_string();
                    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    result.push(RomFile {
                        path: path.to_string_lossy().to_string(),
                        filename: fname,
                        name,
                        extension: ext_lower,
                        size_bytes: size,
                        platform: None,
                    });
                }
            }
        }
    }
}

// ============================================================================
// 内置模拟器定义（从原版 C# YAML 转换 — 20+ 模拟器）
// ============================================================================

pub fn builtin_emulator_definitions() -> Vec<EmulatorDefinition> {
    vec![
        retroarch_def(),
        pcsx2_def(),
        dolphin_def(),
        rpcs3_def(),
        ppsspp_def(),
        duckstation_def(),
        citra_def(),
        yuzu_def(),
        ryujinx_def(),
        xenia_def(),
        cemu_def(),
        mame_def(),
        dosbox_def(),
        scummvm_def(),
        bizhawk_def(),
        melonds_def(),
        mgba_def(),
        snes9x_def(),
        project64_def(),
        flycast_def(),
    ]
}

fn emu(id: &str, name: &str, website: &str, profiles: Vec<EmulatorProfile>) -> EmulatorDefinition {
    EmulatorDefinition {
        id: id.into(),
        name: name.into(),
        website: Some(website.into()),
        profiles,
    }
}

fn prof(
    name: &str,
    args: &str,
    platforms: Vec<&str>,
    exts: Vec<&str>,
    exe: &str,
    files: Option<Vec<&str>>,
) -> EmulatorProfile {
    EmulatorProfile {
        name: name.into(),
        startup_arguments: Some(args.into()),
        platforms: platforms.into_iter().map(String::from).collect(),
        image_extensions: exts.into_iter().map(String::from).collect(),
        startup_executable: Some(exe.into()),
        profile_files: files.map(|v| v.into_iter().map(String::from).collect()),
    }
}

fn retroarch_def() -> EmulatorDefinition {
    emu(
        "retroarch",
        "RetroArch",
        "http://www.retroarch.com/",
        vec![
            prof(
                "Nintendo NES",
                "-L \".\\cores\\nestopia_libretro.dll\" \"{ImagePath}\"",
                vec!["nintendo_nes"],
                vec!["7z", "bin", "nes", "unf", "unif", "zip"],
                "^retroarch.*\\.exe$",
                Some(vec!["cores\\nestopia_libretro.dll"]),
            ),
            prof(
                "Nintendo SNES",
                "-L \".\\cores\\snes9x_libretro.dll\" \"{ImagePath}\"",
                vec!["nintendo_snes"],
                vec!["7z", "bin", "bs", "smc", "sfc", "swc", "zip"],
                "^retroarch.*\\.exe$",
                Some(vec!["cores\\snes9x_libretro.dll"]),
            ),
            prof(
                "Sega Genesis",
                "-L \".\\cores\\genesis_plus_gx_libretro.dll\" \"{ImagePath}\"",
                vec!["sega_genesis"],
                vec!["7z", "bin", "gen", "md", "smd", "zip"],
                "^retroarch.*\\.exe$",
                Some(vec!["cores\\genesis_plus_gx_libretro.dll"]),
            ),
            prof(
                "Nintendo GB/GBC",
                "-L \".\\cores\\gambatte_libretro.dll\" \"{ImagePath}\"",
                vec!["nintendo_gb", "nintendo_gbc"],
                vec!["7z", "gb", "gbc", "zip"],
                "^retroarch.*\\.exe$",
                Some(vec!["cores\\gambatte_libretro.dll"]),
            ),
            prof(
                "Nintendo GBA",
                "-L \".\\cores\\mgba_libretro.dll\" \"{ImagePath}\"",
                vec!["nintendo_gba"],
                vec!["7z", "gba", "zip"],
                "^retroarch.*\\.exe$",
                Some(vec!["cores\\mgba_libretro.dll"]),
            ),
            prof(
                "Nintendo 64",
                "-L \".\\cores\\mupen64plus_next_libretro.dll\" \"{ImagePath}\"",
                vec!["nintendo_n64"],
                vec!["7z", "n64", "v64", "z64", "zip"],
                "^retroarch.*\\.exe$",
                Some(vec!["cores\\mupen64plus_next_libretro.dll"]),
            ),
            prof(
                "PlayStation",
                "-L \".\\cores\\swanstation_libretro.dll\" \"{ImagePath}\"",
                vec!["sony_playstation"],
                vec!["cue", "iso", "chd", "pbp", "bin", "m3u"],
                "^retroarch.*\\.exe$",
                Some(vec!["cores\\swanstation_libretro.dll"]),
            ),
        ],
    )
}

fn pcsx2_def() -> EmulatorDefinition {
    emu(
        "pcsx2",
        "PCSX2",
        "https://pcsx2.net/",
        vec![prof(
            "Default",
            "\"{ImagePath}\" --nogui --fullboot",
            vec!["sony_playstation2"],
            vec!["iso", "bin", "mdf", "nrg", "img", "gz", "cso", "chd", "m3u"],
            "^pcsx2.*\\.exe$",
            None,
        )],
    )
}

fn dolphin_def() -> EmulatorDefinition {
    emu(
        "dolphin",
        "Dolphin",
        "https://dolphin-emu.org/",
        vec![
            prof(
                "GameCube",
                "--exec=\"{ImagePath}\" --batch",
                vec!["nintendo_gamecube"],
                vec![
                    "elf", "dol", "gcm", "tgc", "ciso", "gcz", "iso", "wad", "rvz", "m3u",
                ],
                "^Dolphin\\.exe$",
                None,
            ),
            prof(
                "Wii",
                "--exec=\"{ImagePath}\" --batch",
                vec!["nintendo_wii"],
                vec![
                    "elf", "dol", "tgc", "wbfs", "ciso", "gcz", "iso", "wad", "rvz", "wia", "m3u",
                ],
                "^Dolphin\\.exe$",
                None,
            ),
        ],
    )
}

fn rpcs3_def() -> EmulatorDefinition {
    emu(
        "rpcs3",
        "RPCS3",
        "https://rpcs3.net/",
        vec![prof(
            "Default",
            "\"{ImagePath}\"",
            vec!["sony_playstation3"],
            vec!["ps3dir", "ps3", "pkg"],
            "^rpcs3\\.exe$",
            None,
        )],
    )
}

fn ppsspp_def() -> EmulatorDefinition {
    emu(
        "ppsspp",
        "PPSSPP",
        "https://ppsspp.org/",
        vec![prof(
            "Default",
            "\"{ImagePath}\"",
            vec!["sony_psp"],
            vec!["iso", "cso", "pbp", "elf"],
            "^PPSSPP.*\\.exe$",
            None,
        )],
    )
}

fn duckstation_def() -> EmulatorDefinition {
    emu(
        "duckstation",
        "DuckStation",
        "https://duckstation.org/",
        vec![prof(
            "Default",
            "\"{ImagePath}\"",
            vec!["sony_playstation"],
            vec!["iso", "bin", "cue", "img", "chd", "psexe", "pbp"],
            "^duckstation.*\\.exe$|^DuckStation.*\\.exe$",
            None,
        )],
    )
}

fn citra_def() -> EmulatorDefinition {
    emu(
        "citra",
        "Citra",
        "https://citra-emu.org/",
        vec![prof(
            "Default",
            "\"{ImagePath}\"",
            vec!["nintendo_3ds"],
            vec!["3ds", "3dsx", "app", "cci", "cxi", "cia"],
            "^citra.*\\.exe$",
            None,
        )],
    )
}

fn yuzu_def() -> EmulatorDefinition {
    emu(
        "yuzu",
        "Yuzu",
        "https://yuzu-emu.org/",
        vec![prof(
            "Default",
            "\"{ImagePath}\"",
            vec!["nintendo_switch"],
            vec!["nsp", "xci", "nca"],
            "^yuzu.*\\.exe$",
            None,
        )],
    )
}

fn ryujinx_def() -> EmulatorDefinition {
    emu(
        "ryujinx",
        "Ryujinx",
        "https://ryujinx.org/",
        vec![prof(
            "Default",
            "\"{ImagePath}\"",
            vec!["nintendo_switch"],
            vec!["nsp", "xci", "nca"],
            "^Ryujinx\\.exe$|^Ryujinx\\.sh",
            None,
        )],
    )
}

fn xenia_def() -> EmulatorDefinition {
    emu(
        "xenia",
        "Xenia",
        "https://xenia.jp/",
        vec![prof(
            "Default",
            "\"{ImagePath}\"",
            vec!["microsoft_xbox360"],
            vec!["iso", "xex", "xbe"],
            "^xenia.*\\.exe$",
            None,
        )],
    )
}

fn cemu_def() -> EmulatorDefinition {
    emu(
        "cemu",
        "Cemu",
        "https://cemu.info/",
        vec![prof(
            "Default",
            "-g \"{ImagePath}\"",
            vec!["nintendo_wiiu"],
            vec!["wud", "wux", "iso", "rpx"],
            "^Cemu\\.exe$",
            None,
        )],
    )
}

fn mame_def() -> EmulatorDefinition {
    emu(
        "mame",
        "MAME",
        "https://mamedev.org/",
        vec![prof(
            "Default",
            "\"{ImagePath}\"",
            vec!["arcade"],
            vec!["7z", "zip", "chd"],
            "^mame\\.exe$|^mame64\\.exe$",
            None,
        )],
    )
}

fn dosbox_def() -> EmulatorDefinition {
    emu(
        "dosbox",
        "DOSBox",
        "https://www.dosbox.com/",
        vec![prof(
            "Default",
            "\"{ImagePath}\" -conf \"{ImageDir}\\dosbox.conf\"",
            vec!["ms_dos"],
            vec!["exe", "com", "bat", "conf", "iso", "img"],
            "^DOSBox.*\\.exe$",
            None,
        )],
    )
}

fn scummvm_def() -> EmulatorDefinition {
    emu(
        "scummvm",
        "ScummVM",
        "https://www.scummvm.org/",
        vec![prof(
            "Default",
            "\"{ImagePath}\"",
            vec!["scumm"],
            vec!["exe", "bat", "sh"],
            "^scummvm.*\\.exe$",
            None,
        )],
    )
}

fn bizhawk_def() -> EmulatorDefinition {
    emu(
        "bizhawk",
        "BizHawk",
        "https://tasvideos.org/BizHawk",
        vec![prof(
            "Default",
            "\"{ImagePath}\"",
            vec!["multi"],
            vec![
                "nes", "smc", "sfc", "gba", "gb", "gbc", "gen", "smd", "z64", "v64",
            ],
            "^EmuHawk\\.exe$",
            None,
        )],
    )
}

fn melonds_def() -> EmulatorDefinition {
    emu(
        "melonds",
        "melonDS",
        "https://melonds.kuribo64.net/",
        vec![prof(
            "Default",
            "\"{ImagePath}\"",
            vec!["nintendo_ds"],
            vec!["nds", "dsi", "bin", "zip", "7z"],
            "^melonDS\\.exe$",
            None,
        )],
    )
}

fn mgba_def() -> EmulatorDefinition {
    emu(
        "mgba",
        "mGBA",
        "https://mgba.io/",
        vec![prof(
            "Default",
            "\"{ImagePath}\"",
            vec!["nintendo_gba", "nintendo_gb", "nintendo_gbc"],
            vec!["gba", "gb", "gbc", "zip", "7z"],
            "^mGBA\\.exe$|^mGBA-qt\\.exe$",
            None,
        )],
    )
}

fn snes9x_def() -> EmulatorDefinition {
    emu(
        "snes9x",
        "Snes9x",
        "https://www.snes9x.com/",
        vec![prof(
            "Default",
            "\"{ImagePath}\"",
            vec!["nintendo_snes"],
            vec!["smc", "sfc", "swc", "fig", "bs", "st", "zip", "7z"],
            "^snes9x.*\\.exe$",
            None,
        )],
    )
}

fn project64_def() -> EmulatorDefinition {
    emu(
        "project64",
        "Project64",
        "https://www.pj64-emu.com/",
        vec![prof(
            "Default",
            "\"{ImagePath}\"",
            vec!["nintendo_n64"],
            vec!["z64", "v64", "n64", "rom"],
            "^Project64\\.exe$",
            None,
        )],
    )
}

fn flycast_def() -> EmulatorDefinition {
    emu(
        "flycast",
        "Flycast",
        "https://github.com/flyinghead/flycast",
        vec![prof(
            "Default",
            "\"{ImagePath}\"",
            vec!["sega_dreamcast"],
            vec!["cdi", "gdi", "chd", "cue", "iso"],
            "^flycast.*\\.exe$",
            None,
        )],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_definitions_count() {
        let defs = builtin_emulator_definitions();
        assert!(defs.len() >= 20);
    }

    #[test]
    fn test_retroarch_has_profiles() {
        let ra = retroarch_def();
        assert!(ra.profiles.len() >= 5);
    }

    #[test]
    fn test_scan_roms_empty_dir() {
        let tmp = std::env::temp_dir().join("moe_emu_test_empty");
        std::fs::create_dir_all(&tmp).ok();
        let roms = scan_roms(&tmp.to_string_lossy(), &["iso".into(), "nes".into()], true);
        assert!(roms.is_empty());
        std::fs::remove_dir_all(&tmp).ok();
    }
}
