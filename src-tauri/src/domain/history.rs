//! 历史数据领域模型 v2（FR-08）
//!
//! 统一番剧/漫画/小说三类历史记录的数据结构，作为存储层
//! （`crate::db_sqlite::HistoryDb` / `HistoryRepo`）、迁移层
//! （`crate::migration`）与下游子任务 5（同步）/ 6（历史列表）的共享契约。
//!
//! 序列化字段名固定为 camelCase（对前端的 wire format），数据库列名固定为
//! snake_case（见 `crate::db_sqlite::SCHEMA_V2_SQL`）。

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// 内容类型。数据库 TEXT 列取值固定为 `anime` / `manga` / `novel`。
///
/// `FromStr` 额外接受 v1 时代的别名（`bangumi`→anime、`comic`→manga 等），
/// 供迁移映射使用；`Display` 只输出规范化值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Anime,
    Manga,
    Novel,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::Anime => "anime",
            ContentType::Manga => "manga",
            ContentType::Novel => "novel",
        }
    }
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ContentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "anime" | "bangumi" | "番剧" => Ok(ContentType::Anime),
            "manga" | "comic" | "漫画" => Ok(ContentType::Manga),
            "novel" | "小说" => Ok(ContentType::Novel),
            other => Err(format!("unsupported content type: {other}")),
        }
    }
}

/// 历史记录（v2 统一模型）。
///
/// FR-08 验收字段对照：`contentId→content_id`、`contentType→content_type`、
/// `title`、`cover`、`chapterId→chapter_id`、`chapterTitle→chapter_title`、
/// `pageIndex→page_index`、`progress`（由 [`HistoryRecord::progress`] 按类型
/// 输出 `position_sec` / `page_index` / `scroll_pct`）、`sourceId→source_id`、
/// `updatedAt→updated_at`、`deviceId→device_id`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    /// uuid v4
    pub id: String,
    /// 条目标识（源内唯一 ID）
    pub content_id: String,
    pub content_type: ContentType,
    pub title: String,
    pub cover: Option<String>,
    pub source_id: String,
    /// 集/话/章 ID
    pub chapter_id: Option<String>,
    pub chapter_title: Option<String>,
    /// 漫画页码（原子单位=单页，与渲染模式无关）；番剧/小说为 0
    pub page_index: i64,
    /// 番剧播放进度秒数
    pub position_sec: f64,
    /// 小说滚动百分比 0.0~100.0
    pub scroll_pct: f64,
    /// Unix 毫秒时间戳
    pub updated_at: i64,
    /// 设备唯一标识（子任务 5 同步合并依赖其稳定）
    pub device_id: String,
    /// 墓碑标记，同步用
    pub deleted: bool,
}

impl HistoryRecord {
    /// 前端展示用 `progress`：按内容类型映射为推进位置。
    ///
    /// - 番剧 → `position_sec`（播放秒数）
    /// - 漫画 → `page_index`（当前单页）
    /// - 小说 → `scroll_pct`（滚动百分比）
    pub fn progress(&self) -> f64 {
        match self.content_type {
            ContentType::Anime => self.position_sec,
            ContentType::Manga => self.page_index as f64,
            ContentType::Novel => self.scroll_pct,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_type_roundtrips_and_accepts_v1_aliases() {
        for (raw, expected) in [
            ("anime", ContentType::Anime),
            ("bangumi", ContentType::Anime),
            ("番剧", ContentType::Anime),
            ("manga", ContentType::Manga),
            ("comic", ContentType::Manga),
            ("novel", ContentType::Novel),
            ("小说", ContentType::Novel),
        ] {
            assert_eq!(ContentType::from_str(raw).unwrap(), expected);
            assert_eq!(expected.as_str(), expected.to_string());
        }
        assert!(ContentType::from_str("game").is_err());
    }

    #[test]
    fn wire_names_are_stable() {
        let record = HistoryRecord {
            id: "id".into(),
            content_id: "cid".into(),
            content_type: ContentType::Anime,
            title: "Title".into(),
            cover: None,
            source_id: "src".into(),
            chapter_id: None,
            chapter_title: None,
            page_index: 0,
            position_sec: 12.5,
            scroll_pct: 0.0,
            updated_at: 1_600_000_000_000,
            device_id: "dev".into(),
            deleted: false,
        };
        let value = serde_json::to_value(&record).unwrap();
        assert_eq!(value["contentId"], "cid");
        assert_eq!(value["contentType"], "anime");
        assert_eq!(value["updatedAt"].as_i64(), Some(1_600_000_000_000));
        assert_eq!(value["positionSec"], 12.5);
        assert_eq!(record.progress(), 12.5);
    }
}
