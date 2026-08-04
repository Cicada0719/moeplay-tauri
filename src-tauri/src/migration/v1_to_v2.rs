//! v1 JSON → v2 SQLite 的具体迁移实现（FR-08）。
//!
//! `V1HistoryEntry` 是 v1 历史记录的容错反序列化结构：所有字段可选
//! （`#[serde(default)]`），同时接受 spec §4.1 的通用 v1 字段与仓库早期
//! `anime.rs::AnimeHistory` 的字段名。`map_v1_to_v2` 是纯函数，单独抽出
//! 以便 100% 单测；非法记录记入 skipped 而非中断迁移。
//!
//! > 接口假设：仓库当前不存在真实落盘的 v1 JSON 历史文件（番剧/漫画/小说历史
//! > 存于前端 localStorage）。因此 v1 契约以本模块的 `V1HistoryEntry` 为准，
//! > 同时兼容 `AnimeHistory` 形状，便于旧数据文件按 spec §4.1 映射。

use crate::db_sqlite::DbError;
use crate::domain::history::{ContentType, HistoryRecord};
use rusqlite::OptionalExtension;
use serde::Deserialize;
use std::fmt;
use std::str::FromStr;

/// v1 原始记录（容错反序列化：字段缺失/未知均不导致整体解析失败）。
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct V1HistoryEntry {
    // ---- spec §4.1 通用 v1 字段 ----
    pub content_id: Option<String>,
    pub content_type: Option<String>,
    pub title: Option<String>,
    pub cover: Option<String>,
    pub source_id: Option<String>,
    pub chapter_id: Option<String>,
    pub chapter_title: Option<String>,
    pub page_index: Option<i64>,
    pub position_sec: Option<f64>,
    pub position_ms: Option<i64>,
    pub scroll_pct: Option<f64>,
    pub updated_at: Option<serde_json::Value>,
    pub deleted: Option<bool>,
    // ---- 早期 anime.rs::AnimeHistory 形状 ----
    pub key: Option<String>,
    pub name: Option<String>,
    pub name_cn: Option<String>,
    pub image: Option<String>,
    pub rule_name: Option<String>,
    pub last_episode: Option<i64>,
    pub last_episode_name: Option<String>,
    pub last_road: Option<i64>,
    pub progress_ms: Option<i64>,
    pub last_src: Option<String>,
}

/// 单条 v1 → v2 映射失败原因（非法记录，不计入迁移）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapError {
    MissingContentId,
    MissingTitle,
    UnknownContentType(String),
}

impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MapError::MissingContentId => write!(f, "missing content_id"),
            MapError::MissingTitle => write!(f, "missing title"),
            MapError::UnknownContentType(ty) => write!(f, "unsupported content_type: {ty}"),
        }
    }
}

impl std::error::Error for MapError {}

/// 幂等写入结果。`Inserted` 携带新写入行的 `id`；`Replaced` 携带被覆盖行的
/// `id` 与**更新前快照**（迁移框架据此登记 staging 表，回滚时还原原行）。
#[derive(Debug, Clone, PartialEq)]
pub enum UpsertOutcome {
    Inserted(String),
    Replaced(String, HistoryRecord),
    Skipped,
}

fn first_non_empty<'a>(candidates: &[Option<&'a str>]) -> Option<&'a str> {
    candidates
        .iter()
        .flatten()
        .copied()
        .find(|value| !value.trim().is_empty())
}

/// 纯函数：单条 v1 → v2 映射。
///
/// - 生成 uuid；`device_id` 取全局设备标识；
/// - 缺失字段按 §4.1 填默认值；
/// - 非法记录（缺 `content_id`/`title`、类型无法识别）返回 `Err(MapError)`。
pub fn map_v1_to_v2(entry: &V1HistoryEntry, device_id: &str) -> Result<HistoryRecord, MapError> {
    let content_id = first_non_empty(&[entry.content_id.as_deref(), entry.key.as_deref()])
        .ok_or(MapError::MissingContentId)?
        .trim()
        .to_string();
    let title = first_non_empty(&[entry.title.as_deref(), entry.name_cn.as_deref(), entry.name.as_deref()])
        .ok_or(MapError::MissingTitle)?
        .trim()
        .to_string();

    let content_type = match entry.content_type.as_deref() {
        Some(raw) => ContentType::from_str(raw)
            .map_err(|_| MapError::UnknownContentType(raw.to_string()))?,
        // 早期番剧历史形状没有 content_type，但带番剧专属字段 → 视为 anime
        None if entry.last_episode.is_some() || entry.progress_ms.is_some() || entry.rule_name.is_some() => {
            ContentType::Anime
        }
        None => return Err(MapError::UnknownContentType("missing content_type".to_string())),
    };

    let source_id = first_non_empty(&[
        entry.source_id.as_deref(),
        entry.rule_name.as_deref(),
        entry.last_src.as_deref(),
    ])
    .unwrap_or("unknown")
    .trim()
    .to_string();
    let cover = first_non_empty(&[entry.cover.as_deref(), entry.image.as_deref()])
        .map(|value| value.trim().to_string());
    let chapter_id = entry
        .chapter_id
        .clone()
        .or_else(|| entry.last_episode.map(|episode| format!("episode-{episode}")))
        .filter(|value| !value.trim().is_empty());
    let chapter_title = entry
        .chapter_title
        .clone()
        .or_else(|| entry.last_episode_name.clone())
        .filter(|value| !value.trim().is_empty());

    let page_index = entry.page_index.unwrap_or(0);
    let position_sec = entry
        .position_sec
        .or_else(|| entry.position_ms.map(|ms| ms as f64 / 1000.0))
        .or_else(|| entry.progress_ms.map(|ms| ms as f64 / 1000.0))
        .unwrap_or(0.0);
    let scroll_pct = entry.scroll_pct.unwrap_or(0.0);
    let updated_at = normalize_updated_at(entry.updated_at.as_ref())
        .unwrap_or_else(now_ms);
    let deleted = entry.deleted.unwrap_or(false);

    Ok(HistoryRecord {
        id: uuid::Uuid::new_v4().to_string(),
        content_id,
        content_type,
        title,
        cover,
        source_id,
        chapter_id,
        chapter_title,
        page_index,
        position_sec,
        scroll_pct,
        updated_at,
        device_id: device_id.to_string(),
        deleted,
    })
}

/// 归一化时间戳到 Unix 毫秒：
/// - 秒级（< 1e12）→ ×1000；
/// - 毫秒级（≥ 1e12）→ 原值（不重复放大）；
/// - RFC3339 字符串 → 毫秒。
fn normalize_updated_at(value: Option<&serde_json::Value>) -> Option<i64> {
    let raw: i128 = match value? {
        serde_json::Value::Number(number) => number
            .as_i64()
            .map(i128::from)
            .or_else(|| number.as_u64().map(i128::from))
            .or_else(|| number.as_f64().map(|f| f as i128))?,
        serde_json::Value::String(text) => {
            if let Ok(parsed) = text.parse::<i64>() {
                return Some(normalize_ts_unit(parsed as i128));
            }
            if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(text) {
                return Some(datetime.timestamp_millis());
            }
            return None;
        }
        _ => return None,
    };
    Some(normalize_ts_unit(raw))
}

fn normalize_ts_unit(raw: i128) -> i64 {
    if raw < 1_000_000_000_000 {
        (raw * 1000) as i64
    } else {
        raw as i64
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

/// 幂等写入：以 `(content_id, source_id, chapter_id)` 判定重复。
/// 目标行已存在且 `updated_at >= 待写入值` → `Skipped`；
/// 已存在但更旧 → `UPDATE`（保留原 `id`）并返回 `Replaced(原 id, 原行快照)`；
/// 不存在 → `INSERT` 并返回 `Inserted(新 id)`。
///
/// `chapter_id = NULL` 的匹配语义边界（merge-key 去重）：匹配查询用 `chapter_id IS ?3`
/// （NULL-safe 等值），因此 `NULL` 与任何具体值（如 `'ch1'`）分属不同 merge key、各自成行；
/// 同一 merge key 的多条 v1 记录按 `updated_at` 收敛——较旧者被覆盖或跳过，保证 v2 表中
/// 同一 merge key 至多一行（FR-08 “无重复记录”）。v1 数据缺 `chapter_id` 时，同
/// `content_id + source_id` 的多集/多话会被合并为最新一条（这是设计语义，见 spec §4.1 默认值）。
pub fn upsert_idempotent(
    tx: &rusqlite::Transaction<'_>,
    rec: &HistoryRecord,
) -> Result<UpsertOutcome, DbError> {
    let existing: Option<(String, i64)> = tx
        .query_row(
            "SELECT id, updated_at FROM history \
             WHERE content_id = ?1 AND source_id = ?2 AND chapter_id IS ?3",
            rusqlite::params![rec.content_id, rec.source_id, rec.chapter_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    if let Some((existing_id, existing_updated_at)) = existing {
        if existing_updated_at >= rec.updated_at {
            return Ok(UpsertOutcome::Skipped);
        }
        // 先取被覆盖行的完整快照，供迁移回滚时还原原行。
        let snapshot = load_snapshot(tx, &existing_id)?;
        tx.execute(
            "UPDATE history SET
                content_type=?1, title=?2, cover=?3, source_id=?4, chapter_id=?5,
                chapter_title=?6, page_index=?7, position_sec=?8, scroll_pct=?9,
                updated_at=?10, device_id=?11, deleted=?12
             WHERE id=?13",
            rusqlite::params![
                rec.content_type.as_str(),
                rec.title,
                rec.cover,
                rec.source_id,
                rec.chapter_id,
                rec.chapter_title,
                rec.page_index,
                rec.position_sec,
                rec.scroll_pct,
                rec.updated_at,
                rec.device_id,
                i64::from(rec.deleted),
                existing_id,
            ],
        )?;
        return Ok(UpsertOutcome::Replaced(existing_id, snapshot));
    }

    tx.execute(
        "INSERT INTO history
            (id, content_id, content_type, title, cover, source_id, chapter_id, chapter_title,
             page_index, position_sec, scroll_pct, updated_at, device_id, deleted)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        rusqlite::params![
            rec.id,
            rec.content_id,
            rec.content_type.as_str(),
            rec.title,
            rec.cover,
            rec.source_id,
            rec.chapter_id,
            rec.chapter_title,
            rec.page_index,
            rec.position_sec,
            rec.scroll_pct,
            rec.updated_at,
            rec.device_id,
            i64::from(rec.deleted),
        ],
    )?;
    Ok(UpsertOutcome::Inserted(rec.id.clone()))
}

/// 读取一行完整 `history` 记录作为回滚快照。
fn load_snapshot(tx: &rusqlite::Transaction<'_>, id: &str) -> Result<HistoryRecord, DbError> {
    let record = tx.query_row(
        "SELECT id, content_id, content_type, title, cover, source_id, chapter_id,
                chapter_title, page_index, position_sec, scroll_pct, updated_at, device_id, deleted
         FROM history WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            let content_type_raw: String = row.get(2)?;
            let content_type = ContentType::from_str(&content_type_raw).map_err(|message| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        message,
                    )),
                )
            })?;
            Ok(HistoryRecord {
                id: row.get(0)?,
                content_id: row.get(1)?,
                content_type,
                title: row.get(3)?,
                cover: row.get(4)?,
                source_id: row.get(5)?,
                chapter_id: row.get(6)?,
                chapter_title: row.get(7)?,
                page_index: row.get(8)?,
                position_sec: row.get(9)?,
                scroll_pct: row.get(10)?,
                updated_at: row.get(11)?,
                device_id: row.get(12)?,
                deleted: row.get::<_, i64>(13)? != 0,
            })
        },
    )?;
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_json(fields: &[(&str, serde_json::Value)]) -> V1HistoryEntry {
        let mut map = serde_json::Map::new();
        for (key, value) in fields {
            map.insert((*key).to_string(), value.clone());
        }
        serde_json::from_value(serde_json::Value::Object(map)).unwrap()
    }

    #[test]
    fn maps_generic_v1_fields_per_spec() {
        let entry = entry_json(&[
            ("content_id", "c1".into()),
            ("content_type", "manga".into()),
            ("title", "漫画".into()),
            ("cover", "cover.jpg".into()),
            ("source_id", "src1".into()),
            ("chapter_id", "ch2".into()),
            ("chapter_title", "第二话".into()),
            ("page_index", 7.into()),
            ("updated_at", 1_600_000_000.into()),
        ]);
        let rec = map_v1_to_v2(&entry, "dev-1").unwrap();
        assert_eq!(rec.content_id, "c1");
        assert_eq!(rec.content_type, ContentType::Manga);
        assert_eq!(rec.title, "漫画");
        assert_eq!(rec.cover.as_deref(), Some("cover.jpg"));
        assert_eq!(rec.source_id, "src1");
        assert_eq!(rec.chapter_id.as_deref(), Some("ch2"));
        assert_eq!(rec.chapter_title.as_deref(), Some("第二话"));
        assert_eq!(rec.page_index, 7);
        assert_eq!(rec.updated_at, 1_600_000_000_000);
        assert_eq!(rec.device_id, "dev-1");
        assert!(!rec.deleted);
        assert_eq!(rec.progress(), 7.0);
    }

    #[test]
    fn maps_legacy_anime_history_shape() {
        let entry = entry_json(&[
            ("key", "a1".into()),
            ("name_cn", "进击的巨人".into()),
            ("image", "img.jpg".into()),
            ("rule_name", "zzzfun".into()),
            ("last_episode", 5.into()),
            ("last_episode_name", "第5话".into()),
            ("progress_ms", 750_000.into()),
            ("last_src", "src-x".into()),
        ]);
        let rec = map_v1_to_v2(&entry, "dev-1").unwrap();
        assert_eq!(rec.content_id, "a1");
        assert_eq!(rec.content_type, ContentType::Anime);
        assert_eq!(rec.title, "进击的巨人");
        assert_eq!(rec.cover.as_deref(), Some("img.jpg"));
        assert_eq!(rec.source_id, "zzzfun");
        assert_eq!(rec.chapter_id.as_deref(), Some("episode-5"));
        assert_eq!(rec.chapter_title.as_deref(), Some("第5话"));
        assert_eq!(rec.position_sec, 750.0);
    }

    #[test]
    fn defaults_and_skips() {
        let rec = map_v1_to_v2(&entry_json(&[("content_id", "c1".into()), ("content_type", "novel".into()), ("title", "T".into())]), "d").unwrap();
        assert_eq!(rec.source_id, "unknown");
        assert_eq!(rec.cover, None);
        assert_eq!(rec.chapter_id, None);
        assert_eq!(rec.position_sec, 0.0);
        assert_eq!(rec.scroll_pct, 0.0);
        assert_eq!(rec.page_index, 0);
        assert!(!rec.deleted);

        assert_eq!(
            map_v1_to_v2(&entry_json(&[("content_type", "anime".into()), ("title", "T".into())]), "d"),
            Err(MapError::MissingContentId)
        );
        assert_eq!(
            map_v1_to_v2(&entry_json(&[("content_id", "c1".into()), ("content_type", "anime".into())]), "d"),
            Err(MapError::MissingTitle)
        );
        assert_eq!(
            map_v1_to_v2(&entry_json(&[("content_id", "c1".into()), ("content_type", "game".into()), ("title", "T".into())]), "d"),
            Err(MapError::UnknownContentType("game".into()))
        );
    }

    #[test]
    fn timestamp_units_normalized_once() {
        let seconds = map_v1_to_v2(
            &entry_json(&[("content_id", "c1".into()), ("content_type", "anime".into()), ("title", "T".into()), ("updated_at", 1_600_000_000.into())]),
            "d",
        )
        .unwrap();
        assert_eq!(seconds.updated_at, 1_600_000_000_000);

        let millis = map_v1_to_v2(
            &entry_json(&[("content_id", "c1".into()), ("content_type", "anime".into()), ("title", "T".into()), ("updated_at", 1_600_000_000_000_i64.into())]),
            "d",
        )
        .unwrap();
        assert_eq!(millis.updated_at, 1_600_000_000_000);
    }
}
