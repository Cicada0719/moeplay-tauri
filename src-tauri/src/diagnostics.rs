// 诊断服务 + 国际化系统
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sysinfo::System;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsReport {
    pub system_info: SystemInfo,
    pub app_info: AppInfo,
    pub issues: Vec<Issue>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub memory_gb: f64,
    pub disk_free_gb: f64,
    pub locale_emulator_installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub version: String,
    pub database_size_mb: f64,
    pub game_count: u32,
    pub scrape_sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub severity: Severity,
    pub category: String,
    pub message: String,
    pub solution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

/// 运行完整诊断。需要传入已打开的数据目录路径、游戏数、数据库文件大小。
pub fn run_diagnostics(
    data_dir: &std::path::Path,
    game_count: u32,
    db_size_bytes: u64,
) -> DiagnosticsReport {
    let mut sys = System::new_all();
    sys.refresh_all();

    let memory_gb = sys.total_memory() as f64 / 1_073_741_824.0;
    let disk_free_gb = sysinfo::Disks::new_with_refreshed_list()
        .iter()
        .find(|d| data_dir.starts_with(d.mount_point()))
        .map(|d| d.available_space() as f64 / 1_073_741_824.0)
        .unwrap_or(0.0);

    let system_info = SystemInfo {
        os: format!(
            "{} {}",
            std::env::consts::OS,
            System::name().unwrap_or_default()
        ),
        arch: std::env::consts::ARCH.to_string(),
        memory_gb: (memory_gb * 10.0).round() / 10.0,
        disk_free_gb: (disk_free_gb * 10.0).round() / 10.0,
        locale_emulator_installed: is_le_installed(),
    };

    let app_info = AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        database_size_mb: (db_size_bytes as f64 / 1_048_576.0 * 10.0).round() / 10.0,
        game_count,
        scrape_sources: vec![
            "VNDB".into(),
            "Bangumi".into(),
            "DLsite".into(),
            "TouchGAL".into(),
            "ErogameScape".into(),
            "Ymgal".into(),
            "Kungal".into(),
            "Steam".into(),
        ],
    };

    let mut issues = Vec::new();
    let recommendations = vec![
        "确保 Locale Emulator 已安装（用于日区游戏）".into(),
        "建议定期备份游戏存档".into(),
    ];

    if memory_gb < 4.0 {
        issues.push(Issue {
            severity: Severity::Warning,
            category: "system".into(),
            message: format!("系统内存仅 {:.1} GB，推荐 4 GB 以上", memory_gb),
            solution: Some("关闭其他应用以释放内存".into()),
        });
    }
    if disk_free_gb < 10.0 && disk_free_gb > 0.0 {
        issues.push(Issue {
            severity: Severity::Warning,
            category: "storage".into(),
            message: format!("磁盘剩余空间仅 {:.1} GB", disk_free_gb),
            solution: Some("清理磁盘空间，避免存档快照写入失败".into()),
        });
    }
    if game_count == 0 {
        issues.push(Issue {
            severity: Severity::Info,
            category: "library".into(),
            message: "游戏库为空".into(),
            solution: Some("使用导入功能添加游戏".into()),
        });
    }
    if !is_le_installed() {
        issues.push(Issue {
            severity: Severity::Info,
            category: "compatibility".into(),
            message: "未检测到 Locale Emulator".into(),
            solution: Some("安装 LE 以支持日文游戏日区启动".into()),
        });
    }

    DiagnosticsReport {
        system_info,
        app_info,
        issues,
        recommendations,
    }
}

fn is_le_installed() -> bool {
    let paths = [
        r"C:\Program Files\Locale Emulator\LEProc.exe",
        r"C:\Program Files (x86)\Locale Emulator\LEProc.exe",
    ];
    paths.iter().any(|p| std::path::Path::new(p).exists())
}

// ============ 国际化系统 ============

pub struct I18n {
    strings: HashMap<String, HashMap<String, String>>,
    current_language: String,
}

impl Default for I18n {
    fn default() -> Self {
        Self::new()
    }
}

impl I18n {
    pub fn new() -> Self {
        let mut i18n = Self {
            strings: HashMap::new(),
            current_language: "zh_CN".to_string(),
        };
        i18n.load_builtin();
        i18n
    }

    fn load_builtin(&mut self) {
        let mut zh = HashMap::new();
        zh.insert("app.name".into(), "萌游 MoeGame".into());
        zh.insert("app.title".into(), "萌游 - 二次元游戏管理器".into());
        zh.insert("menu.games".into(), "游戏库".into());
        zh.insert("menu.scraper".into(), "AI 刮削".into());
        zh.insert("menu.downloads".into(), "资源下载".into());
        zh.insert("menu.backup".into(), "存档管理".into());
        zh.insert("menu.stats".into(), "统计".into());
        zh.insert("menu.settings".into(), "设置".into());
        zh.insert("menu.diagnostics".into(), "诊断".into());
        zh.insert("game.launch".into(), "启动游戏".into());
        zh.insert("game.scrape".into(), "AI 刮削".into());
        zh.insert("game.backup".into(), "存档备份".into());
        zh.insert("game.delete".into(), "删除游戏".into());
        zh.insert("game.favorite".into(), "收藏".into());
        zh.insert("search.placeholder".into(), "搜索游戏...".into());
        zh.insert("status.playing".into(), "游玩中".into());
        zh.insert("status.completed".into(), "已通关".into());
        zh.insert("status.planning".into(), "计划玩".into());
        zh.insert("status.dropped".into(), "已弃".into());
        zh.insert("empty.games".into(), "还没有游戏，点击 + 添加游戏吧".into());
        zh.insert("loading".into(), "加载中...".into());
        zh.insert("confirm.delete".into(), "确定要删除这个游戏吗？".into());
        zh.insert("success.import".into(), "导入成功".into());
        zh.insert("success.scrape".into(), "刮削完成".into());
        zh.insert("error.network".into(), "网络连接失败".into());
        zh.insert("error.unknown".into(), "未知错误".into());
        self.strings.insert("zh_CN".into(), zh);

        let mut en = HashMap::new();
        en.insert("app.name".into(), "MoeGame".into());
        en.insert("app.title".into(), "MoeGame - Anime Game Manager".into());
        en.insert("menu.games".into(), "Games".into());
        en.insert("menu.scraper".into(), "AI Scraper".into());
        en.insert("menu.downloads".into(), "Downloads".into());
        en.insert("menu.backup".into(), "Saves".into());
        en.insert("menu.stats".into(), "Stats".into());
        en.insert("menu.settings".into(), "Settings".into());
        en.insert("menu.diagnostics".into(), "Diagnostics".into());
        en.insert("game.launch".into(), "Launch Game".into());
        en.insert("game.scrape".into(), "AI Scrape".into());
        en.insert("game.backup".into(), "Backup Save".into());
        en.insert("game.delete".into(), "Delete Game".into());
        en.insert("game.favorite".into(), "Favorite".into());
        en.insert("search.placeholder".into(), "Search games...".into());
        en.insert("status.playing".into(), "Playing".into());
        en.insert("status.completed".into(), "Completed".into());
        en.insert("status.planning".into(), "Plan to Play".into());
        en.insert("status.dropped".into(), "Dropped".into());
        en.insert("empty.games".into(), "No games yet. Click + to add".into());
        en.insert("loading".into(), "Loading...".into());
        en.insert(
            "confirm.delete".into(),
            "Are you sure you want to delete this game?".into(),
        );
        en.insert("success.import".into(), "Import successful".into());
        en.insert("success.scrape".into(), "Scrape complete".into());
        en.insert("error.network".into(), "Network connection failed".into());
        en.insert("error.unknown".into(), "Unknown error".into());
        self.strings.insert("en_US".into(), en);

        let mut ja = HashMap::new();
        ja.insert("app.name".into(), "萌遊 MoeGame".into());
        ja.insert("app.title".into(), "萌遊 - 二次元ゲームマネージャー".into());
        ja.insert("menu.games".into(), "ゲーム".into());
        ja.insert("menu.scraper".into(), "AI スクレイピング".into());
        ja.insert("menu.downloads".into(), "ダウンロード".into());
        ja.insert("menu.backup".into(), "セーブ".into());
        ja.insert("menu.stats".into(), "統計".into());
        ja.insert("menu.settings".into(), "設定".into());
        ja.insert("search.placeholder".into(), "ゲームを検索...".into());
        self.strings.insert("ja_JP".into(), ja);
    }

    pub fn t(&self, key: &str) -> String {
        self.strings
            .get(&self.current_language)
            .and_then(|lang| lang.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    pub fn set_language(&mut self, lang: &str) {
        if self.strings.contains_key(lang) {
            self.current_language = lang.to_string();
        }
    }

    pub fn get_languages(&self) -> Vec<String> {
        self.strings.keys().cloned().collect()
    }
}

/// 收集诊断数据并导出为 ZIP 文件。
pub fn export_diagnostics_zip(
    output_path: &std::path::Path,
    data_dir: &std::path::Path,
    game_count: u32,
    db_size_bytes: u64,
) -> Result<String, String> {
    use std::io::Write;

    let tmp = std::env::temp_dir().join("moegame_diag");
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;

    let report = run_diagnostics(data_dir, game_count, db_size_bytes);
    let report_json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    std::fs::write(tmp.join("diagnostics.json"), &report_json).map_err(|e| e.to_string())?;

    let logs = crate::logging::collect_recent_logs(200);
    std::fs::write(tmp.join("recent.log"), logs.join("\n")).map_err(|e| e.to_string())?;

    let stats = serde_json::json!({
        "game_count": game_count,
        "db_size_mb": format!("{:.2}", db_size_bytes as f64 / 1_048_576.0),
    });
    std::fs::write(
        tmp.join("stats.json"),
        serde_json::to_string_pretty(&stats).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let zip_file = std::fs::File::create(output_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(zip_file);
    let options = zip::write::FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for entry in &["diagnostics.json", "recent.log", "stats.json"] {
        let path = tmp.join(entry);
        if path.exists() {
            zip.start_file(*entry, options).map_err(|e| e.to_string())?;
            let data = std::fs::read(&path).map_err(|e| e.to_string())?;
            zip.write_all(&data).map_err(|e| e.to_string())?;
        }
    }
    zip.finish().map_err(|e| e.to_string())?;
    std::fs::remove_dir_all(&tmp).ok();
    Ok(output_path.to_string_lossy().to_string())
}
