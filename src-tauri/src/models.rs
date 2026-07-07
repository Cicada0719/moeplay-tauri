use chrono::Utc;
use serde::{Deserialize, Serialize};

// ============================================================================
// 枚举定义
// ============================================================================

/// 游戏完成状态
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum CompletionStatus {
    #[default]
    NotStarted,
    Playing,
    Completed,
    Dropped,
    OnHold,
    PlanToPlay,
    Replaying,
}

/// 标签分类
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TagCategory {
    /// 游戏类型 (RPG, Visual Novel, Action...)
    Genre,
    /// 主题 (奇幻, 校园, 科幻...)
    Theme,
    /// 氛围 (温馨, 黑暗, 搞笑...)
    Mood,
    /// 特性 (多结局, 全语音, 动态CG...)
    Feature,
    /// 内容标签 (纯爱, NTR, 百合...)
    Content,
    /// 用户自定义
    #[default]
    Custom,
}

/// 标签来源
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TagSource {
    Vndb,
    Bangumi,
    Steam,
    Dlsite,
    Ai,
    #[default]
    User,
}

/// 游戏平台
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GamePlatform {
    PC,
    Web,
    Mobile,
    Console,
    Other,
}

// ============================================================================
// 子模型
// ============================================================================

/// 在线商店链接
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct StoreLink {
    pub name: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
}

/// 游戏别名（多语言标题、简称等）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameAlias {
    pub name: String,
    /// 语言代码: "zh", "en", "ja", "ko", "original"...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// 来源: "vndb", "bangumi", "steam", "user"...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// 是否为显示用主标题
    #[serde(default)]
    pub is_primary: bool,
}

/// 增强标签（带分类、颜色、来源）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tag {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default)]
    pub category: TagCategory,
    #[serde(default)]
    pub source: TagSource,
}

/// 游戏元数据（来自刮削或手动填写）
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GameMetadata {
    /// 开发商
    #[serde(skip_serializing_if = "Option::is_none")]
    pub developer: Option<String>,
    /// 发行商
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    /// 平台
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<GamePlatform>,
    /// 游戏引擎 (Unity, Ren'Py, RPG Maker...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// 游戏类型 (RPG, Visual Novel, Action...)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<String>,
    /// 界面语言
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    /// 语音语言
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub voice_languages: Vec<String>,
    /// 游戏版本号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// 游戏原版标题（翻译/汉化游戏的原名）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_name: Option<String>,
    /// 官方网站
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    /// 开发商主页
    #[serde(skip_serializing_if = "Option::is_none")]
    pub developer_homepage: Option<String>,
    /// 在线商店链接
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stores: Vec<StoreLink>,
    /// 年龄分级 (R18, All Ages, R15...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_rating: Option<String>,
    /// 所属系列
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    /// 发行日期 (YYYY-MM-DD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    /// 发行年份
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_year: Option<u32>,
    /// HowLongToBeat 预估通关时长（小时）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_hours: Option<f64>,
    /// VNDB 评分 (1-10)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vndb_rating: Option<f64>,
    /// Bangumi 评分 (1-10)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bangumi_rating: Option<f64>,
    /// VNDB ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vndb_id: Option<String>,
    /// Bangumi ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bangumi_id: Option<String>,
    /// 封面图片路径或 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    /// 背景图片路径或 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
}

/// 单次游玩会话
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaySession {
    /// 会话 ID
    pub id: String,
    /// 开始时间
    pub start_time: String,
    /// 结束时间（None 表示仍在进行中）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    /// 本次游玩时长（秒）
    pub duration_seconds: u64,
    /// 玩家备注
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// 带游戏信息的游玩会话（跨游戏统计用）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaySessionEntry {
    pub game_id: String,
    pub game_name: String,
    pub session: PlaySession,
}

/// 每日游玩时长统计
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DailyPlaytime {
    pub date: String,
    pub seconds: u64,
    pub sessions: u32,
}

/// 每月游玩时长统计
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonthlyPlaytime {
    pub month: String,
    pub seconds: u64,
    pub sessions: u32,
}

/// 单个游戏游玩时长排行
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GamePlaytimeRank {
    pub game_id: String,
    pub game_name: String,
    pub total_seconds: u64,
    pub sessions: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_played: Option<String>,
}

/// 游玩追踪汇总
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaytimeSummary {
    pub total_seconds: u64,
    pub session_count: u32,
    pub play_days: u32,
    pub average_session_seconds: u64,
    pub daily: Vec<DailyPlaytime>,
    pub monthly: Vec<MonthlyPlaytime>,
    pub recent_sessions: Vec<PlaySessionEntry>,
    pub top_games: Vec<GamePlaytimeRank>,
}

/// 游玩追踪器
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PlayTracker {
    /// 累计游玩时长（秒）
    #[serde(default)]
    pub total_seconds: u64,
    /// 游玩会话历史
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sessions: Vec<PlaySession>,
    /// 完成状态
    #[serde(default)]
    pub completion_status: CompletionStatus,
    /// 最后游玩时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_played: Option<String>,
    /// 首次游玩时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_played: Option<String>,
    /// 用户评分 (1-10)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_rating: Option<f64>,
    /// 用户评测/感想
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review: Option<String>,
    /// 成就总数
    #[serde(default)]
    pub achievements_total: u32,
    /// 已解锁成就数
    #[serde(default)]
    pub achievements_unlocked: u32,
    /// 是否已通关
    #[serde(default)]
    pub finished: bool,
    /// 通关次数
    #[serde(default)]
    pub completion_count: u32,
}

/// 存档备份记录
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SaveBackup {
    /// 备份 ID
    pub id: String,
    /// 备份名称（可自定义）
    pub name: String,
    /// 备份文件路径
    pub path: String,
    /// 文件大小（字节）
    pub size: u64,
    /// 创建时间
    pub created: String,
    /// 游戏内进度描述（第几章、哪条路线等）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_note: Option<String>,
    /// 备份备注
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// 存档数据管理
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SaveData {
    /// 已知的存档目录
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_dir: Option<String>,
    /// 是否启用自动备份
    #[serde(default)]
    pub auto_backup: bool,
    /// 自动备份间隔（分钟）
    #[serde(default)]
    pub backup_interval_minutes: u32,
    /// 最大备份数量
    #[serde(default = "SaveData::default_max_backups")]
    pub max_backups: u32,
    /// 备份列表
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backups: Vec<SaveBackup>,
    /// 是否启用云同步
    #[serde(default)]
    pub cloud_sync: bool,
    /// 云同步服务商 ("webdav", "onedrive"...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud_provider: Option<String>,
}

impl SaveData {
    fn default_max_backups() -> u32 {
        10
    }
}

// ============================================================================
// 核心模型：Game
// ============================================================================

/// 游戏数据模型 —— 核心实体
///
/// 包含游戏的基本信息、元数据、游玩追踪、存档管理、
/// 别名系统、标签系统等完整字段。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Game {
    // ---- 基本标识 ----
    /// 唯一 ID (UUID v4)
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 可执行文件路径
    pub exe_path: String,
    /// 安装目录
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_dir: Option<String>,
    /// 游戏类型标识 (visual_novel, rpg, action...)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_type: Option<String>,
    /// 平台导入来源，如 steam / epic
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_source: Option<String>,
    /// 平台侧唯一 ID，如 Steam appid / Epic AppName
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_id: Option<String>,
    /// 平台协议启动 URI，如 steam://rungameid/{appid}
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_uri: Option<String>,
    /// 最近一次平台导入/同步时间
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_imported_at: Option<String>,
    /// 创建时间
    pub created_at: String,
    /// 最后更新时间
    pub updated_at: String,

    // ---- 描述与媒体 ----
    /// 游戏简介/描述
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 封面图片路径或 URL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    /// 背景图片路径或 URL
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    /// 图标路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// 预览截图列表
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub screenshots: Vec<String>,

    // ---- 用户偏好 ----
    /// 是否收藏
    #[serde(default)]
    pub favorite: bool,
    /// 是否隐藏（不在主列表显示）
    #[serde(default)]
    pub hidden: bool,

    // ---- 简单字符串标签（向后兼容） ----
    /// 简单标签列表 (纯字符串，与 tag_entries 互补)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    // ---- 结构化字段 ----
    /// 游戏元数据（开发商、发行商、引擎等）
    #[serde(default)]
    pub metadata: GameMetadata,
    /// 游玩追踪器（时长、会话、完成状态等）
    #[serde(default)]
    pub play_tracker: PlayTracker,
    /// 存档数据管理（备份、云同步等）
    #[serde(default)]
    pub save_data: SaveData,
    /// 游戏别名（多语言标题等）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<GameAlias>,
    /// 增强标签（带分类、颜色、来源）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tag_entries: Vec<Tag>,

    // ---- 向后兼容·旧版字段（标记为不再直接使用） ----
    /// [已弃用] 使用 metadata.release_year 代替
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_year: Option<u32>,
    /// [已弃用] 使用 play_tracker.user_rating 代替
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rating: Option<f64>,
    /// [已弃用] 使用 play_tracker.last_played 代替
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_played: Option<String>,
    /// [已弃用] 使用 metadata.vndb_id 代替
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vndb_id: Option<String>,
    /// [已弃用] 使用 metadata.bangumi_id 代替
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bangumi_id: Option<String>,
    /// [已弃用] 使用 play_tracker.total_seconds 代替
    #[serde(default)]
    pub play_time_seconds: u64,
    /// [已弃用] 使用 created_at 代替
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add_date: Option<String>,
}

impl Game {
    /// 创建新游戏
    pub fn new(name: String, exe_path: String) -> Self {
        let now = Utc::now().format("%Y-%m-%d %H:%M").to_string();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            exe_path,
            install_dir: None,
            game_type: None,
            library_source: None,
            library_id: None,
            launch_uri: None,
            last_imported_at: None,
            created_at: now.clone(),
            updated_at: now,
            description: None,
            cover: None,
            background: None,
            icon: None,
            screenshots: vec![],
            favorite: false,
            hidden: false,
            tags: vec![],
            metadata: GameMetadata::default(),
            play_tracker: PlayTracker::default(),
            save_data: SaveData::default(),
            aliases: vec![],
            tag_entries: vec![],
            // 向后兼容旧字段
            release_year: None,
            rating: None,
            last_played: None,
            vndb_id: None,
            bangumi_id: None,
            play_time_seconds: 0,
            add_date: None,
        }
    }

    // ========================================================================
    // 便利方法
    // ========================================================================

    /// 获取格式化后的游玩时长
    pub fn play_time_display(&self) -> String {
        let total = self.play_tracker.total_seconds.max(self.play_time_seconds);
        if total == 0 {
            return "未游玩".to_string();
        }
        let hours = total / 3600;
        let minutes = (total % 3600) / 60;
        if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        }
    }

    /// 获取评分（优先使用新字段）
    pub fn effective_rating(&self) -> Option<f64> {
        self.play_tracker
            .user_rating
            .or(self.metadata.vndb_rating)
            .or(self.metadata.bangumi_rating)
            .or(self.rating)
    }

    /// 获取发行年份（优先使用新字段）
    pub fn effective_release_year(&self) -> Option<u32> {
        self.metadata.release_year.or(self.release_year)
    }

    /// 获取最后游玩时间（优先使用新字段）
    pub fn effective_last_played(&self) -> Option<&str> {
        self.play_tracker
            .last_played
            .as_deref()
            .or(self.last_played.as_deref())
    }

    /// 标记游戏已更新
    pub fn touch_updated(&mut self) {
        self.updated_at = Utc::now().format("%Y-%m-%d %H:%M").to_string();
    }

    /// 开始游玩会话
    pub fn start_session(&mut self) -> String {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().format("%Y-%m-%d %H:%M").to_string();

        let session = PlaySession {
            id: session_id.clone(),
            start_time: now.clone(),
            end_time: None,
            duration_seconds: 0,
            notes: None,
        };

        if self.play_tracker.first_played.is_none() {
            self.play_tracker.first_played = Some(now.clone());
        }
        self.play_tracker.last_played = Some(now);
        self.last_played = self.play_tracker.last_played.clone();

        if self.play_tracker.completion_status == CompletionStatus::NotStarted {
            self.play_tracker.completion_status = CompletionStatus::Playing;
        }

        self.play_tracker.sessions.push(session);
        self.touch_updated();
        session_id
    }

    /// 结束游玩会话
    pub fn end_session(&mut self, session_id: &str, duration_seconds: u64) -> bool {
        if let Some(session) = self
            .play_tracker
            .sessions
            .iter_mut()
            .find(|s| s.id == session_id && s.end_time.is_none())
        {
            session.end_time = Some(Utc::now().format("%Y-%m-%d %H:%M").to_string());
            session.duration_seconds = duration_seconds;
            self.play_tracker.total_seconds += duration_seconds;
            self.play_time_seconds = self.play_tracker.total_seconds;
            self.touch_updated();
            true
        } else {
            false
        }
    }

    /// 添加别名
    pub fn add_alias(&mut self, name: String, language: Option<String>, source: Option<String>) {
        // 避免重复
        if !self.aliases.iter().any(|a| a.name == name) {
            self.aliases.push(GameAlias {
                name,
                language,
                source,
                is_primary: false,
            });
            self.touch_updated();
        }
    }

    /// 添加增强标签
    pub fn add_tag(&mut self, name: String, category: TagCategory, source: TagSource) {
        if !self.tag_entries.iter().any(|t| t.name == name) {
            self.tag_entries.push(Tag {
                name,
                color: None,
                category,
                source,
            });
            self.touch_updated();
        }
    }

    /// 从 GameMetadata 同步数据到 Game 的向后兼容旧字段
    pub fn sync_to_legacy(&mut self) {
        self.release_year = self.metadata.release_year;
        self.vndb_id.clone_from(&self.metadata.vndb_id);
        self.bangumi_id.clone_from(&self.metadata.bangumi_id);
        self.cover.clone_from(&self.metadata.cover);
        self.background.clone_from(&self.metadata.background);
    }

    /// 从 PlayTracker 同步数据到向后兼容旧字段
    pub fn sync_tracker_to_legacy(&mut self) {
        self.play_time_seconds = self.play_tracker.total_seconds;
        self.last_played.clone_from(&self.play_tracker.last_played);
        self.rating = self.play_tracker.user_rating;
    }

    /// Normalize canonical metadata/play tracker fields before persistence.
    pub fn normalize_for_persistence(&mut self) {
        if self.metadata.cover.is_none() {
            self.metadata.cover.clone_from(&self.cover);
        }
        if self.metadata.background.is_none() {
            self.metadata.background.clone_from(&self.background);
        }
        if self.metadata.release_year.is_none() {
            self.metadata.release_year = self.release_year;
        }
        if self.metadata.vndb_id.is_none() {
            self.metadata.vndb_id.clone_from(&self.vndb_id);
        }
        if self.metadata.bangumi_id.is_none() {
            self.metadata.bangumi_id.clone_from(&self.bangumi_id);
        }
        if self.play_tracker.user_rating.is_none() {
            self.play_tracker.user_rating = self.rating;
        }
        if self.play_tracker.last_played.is_none() {
            self.play_tracker.last_played.clone_from(&self.last_played);
        }
        if self.play_tracker.total_seconds == 0 && self.play_time_seconds > 0 {
            self.play_tracker.total_seconds = self.play_time_seconds;
        }

        self.cover.clone_from(&self.metadata.cover);
        self.background.clone_from(&self.metadata.background);
        self.release_year = self.metadata.release_year;
        self.vndb_id.clone_from(&self.metadata.vndb_id);
        self.bangumi_id.clone_from(&self.metadata.bangumi_id);
        self.rating = self.play_tracker.user_rating;
        self.last_played.clone_from(&self.play_tracker.last_played);
        self.play_time_seconds = self.play_tracker.total_seconds;
    }
}

// ============================================================================
// 刮削结果
// ============================================================================

/// 刮削结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScrapeResult {
    pub title: String,
    pub description: Option<String>,
    pub cover: Option<String>,
    pub background: Option<String>,
    pub tags: Vec<String>,
    pub rating: Option<f64>,
    pub release_year: Option<u32>,
    /// 数据来源: "vndb" | "bangumi"
    pub source: String,
    /// 原始 ID
    pub source_id: String,
    /// 增强的元数据（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ScrapeDetail>,
}

/// 单个数据源的刮削状态
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScrapeSourceStatus {
    pub source: String,
    pub ok: bool,
    pub count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScrapeResponse {
    pub results: Vec<ScrapeResult>,
    pub source_status: Vec<ScrapeSourceStatus>,
}

/// 刮削详细信息
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ScrapeDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub developer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub screenshots: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_rating: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub voice_languages: Vec<String>,
}

// ============================================================================
// 存档信息
// ============================================================================

/// 存档信息（用于文件系统扫描）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SaveInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub created: String,
}

// ============================================================================
// 应用设置
// ============================================================================

/// 应用设置
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    /// 主题: "dark" | "light" | "sakura"
    pub theme: String,
    /// 自动监控的目录
    pub watch_dirs: Vec<String>,
    /// 添加游戏时自动刮削
    pub auto_scrape: bool,
    /// 语言: "zh" | "en" | "ja"
    pub language: String,
    /// 最小化到托盘
    pub minimize_to_tray: bool,
    /// 启用 VNDB 刮削
    pub vndb_enabled: bool,
    /// 启用 Bangumi 刮削
    pub bangumi_enabled: bool,
    /// 启用 DLsite 刮削
    #[serde(default = "Settings::default_true")]
    pub dlsite_enabled: bool,
    /// 启用 TouchGAL 刮削
    #[serde(default = "Settings::default_true")]
    pub touchgal_enabled: bool,
    /// 启用 批评空间 (ErogameScape) 刮削
    #[serde(default = "Settings::default_true")]
    pub erogamescape_enabled: bool,
    /// 启用 月幕 Ymgal 刮削
    #[serde(default = "Settings::default_true")]
    pub ymgal_enabled: bool,
    /// 启用 Kungal 刮削
    #[serde(default = "Settings::default_true")]
    pub kungal_enabled: bool,
    /// 启用 Steam 商店刮削
    #[serde(default = "Settings::default_true")]
    pub steam_enabled: bool,
    /// 启用 PCGamingWiki 刮削
    #[serde(default = "Settings::default_true")]
    pub pcgw_enabled: bool,
    /// 刮削 HTTP 代理（空=系统代理，如 http://127.0.0.1:7890）
    #[serde(default)]
    pub scraper_proxy: String,
    /// 启用 AI 增强刮削
    pub ai_enabled: bool,
    /// AI API 地址
    pub ai_api_url: String,
    /// AI API Key
    pub ai_api_key: String,
    /// AI 模型名称
    pub ai_model: String,
    /// NSFW 显示模式: "show" | "blur" | "hide"
    #[serde(default = "Settings::default_nsfw_display_mode")]
    pub nsfw_display_mode: String,
    /// 开机自动启动
    #[serde(default)]
    pub autostart_enabled: bool,
    /// 自启时的启动模式: "dashboard" | "big-picture"
    #[serde(default = "Settings::default_startup_mode")]
    pub startup_mode: String,
    /// 已连接的 SteamID64
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steam_id: Option<String>,
    /// Steam Web API Key，仅保存在本机设置
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steam_api_key: Option<String>,
    /// 首页是否显示看板娘立绘
    #[serde(default = "Settings::default_true")]
    pub home_mascot_enabled: bool,
    /// 首页看板娘自定义图片路径（空=使用默认）
    #[serde(default)]
    pub home_mascot_path: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            watch_dirs: vec![],
            auto_scrape: true,
            language: "zh".to_string(),
            minimize_to_tray: false,
            vndb_enabled: true,
            bangumi_enabled: true,
            dlsite_enabled: true,
            touchgal_enabled: true,
            erogamescape_enabled: true,
            ymgal_enabled: true,
            kungal_enabled: true,
            steam_enabled: true,
            pcgw_enabled: true,
            scraper_proxy: String::new(),
            ai_enabled: false,
            ai_api_url: "https://api.openai.com/v1/chat/completions".to_string(),
            ai_api_key: String::new(),
            ai_model: "gpt-4o-mini".to_string(),
            nsfw_display_mode: Self::default_nsfw_display_mode(),
            autostart_enabled: false,
            startup_mode: Self::default_startup_mode(),
            steam_id: None,
            steam_api_key: None,
            home_mascot_enabled: true,
            home_mascot_path: String::new(),
        }
    }
}

impl Settings {
    fn default_nsfw_display_mode() -> String {
        "blur".to_string()
    }
    fn default_startup_mode() -> String {
        "fullscreen".to_string()
    }
    fn default_true() -> bool {
        true
    }
}

// ============================================================================
// 应用数据库
// ============================================================================

/// 应用数据库
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppDatabase {
    /// Schema 版本号，用于数据迁移
    ///
    /// 旧版 JSON 文件中没有此字段，serde 反序列化时使用 u32 default = 0，
    /// 随后由 migration 模块按序升级到最新版本。
    #[serde(default)]
    pub schema_version: u32,
    pub games: Vec<Game>,
    pub settings: Settings,
}

impl Default for AppDatabase {
    fn default() -> Self {
        Self {
            schema_version: crate::migration::CURRENT_SCHEMA_VERSION,
            games: Vec::new(),
            settings: Settings::default(),
        }
    }
}
