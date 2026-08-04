//! 同步记录合并算法（FR-09 / spec §3.3）——纯函数、不依赖 DB 与网络。
//!
//! 合并规则（严格按 PRD §5.4 / spec §3.3）：
//! 1. 匹配键 = `(content_id, source_id)`；
//! 2. 同键冲突：`updated_at` 新者胜（**秒**粒度，单位与 `history.json` 传输格式一致）；
//! 3. `updated_at` 相同（同秒）：按内容类型取进度更大者
//!    —— 漫画 `page_index`、番剧 `position_sec`、小说 `scroll_pct`；
//! 4. 任一方 `deleted == true` 且其 `updated_at` 不旧于另一方 → 墓碑胜（删除传播）；
//! 5. 墓碑保留 90 天：`purge_tombstones(now, 90)` 物理清理过期墓碑。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 墓碑保留天数（spec §3.3 rule 5）。
pub const TOMBSTONE_RETENTION_DAYS: i64 = 90;

/// 与 DB 行一一对应的可序列化快照（`history.json` 传输结构）。
///
/// `updated_at` 为 **Unix 秒**（与 `SyncResult::synced_at` 单位一致）；本地
/// `HistoryRecord.updated_at` 为毫秒，转换时 `/1000` 取整（合并按秒粒度，符合
/// "同秒冲突取进度更大者"规则）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncRecord {
    pub id: String,
    pub content_id: String,
    /// anime | manga | novel
    pub content_type: String,
    pub title: String,
    pub cover: Option<String>,
    pub source_id: String,
    pub chapter_id: Option<String>,
    pub chapter_title: Option<String>,
    pub page_index: i64,
    pub position_sec: f64,
    pub scroll_pct: f64,
    pub updated_at: i64,
    pub device_id: String,
    pub deleted: bool,
}

impl SyncRecord {
    /// 按内容类型映射"进度"推进量（同秒冲突时比较用）。
    pub fn progress(&self) -> f64 {
        match self.content_type.as_str() {
            "anime" => self.position_sec,
            "manga" => self.page_index as f64,
            "novel" => self.scroll_pct,
            // 未知类型兜底：取番剧进度（不会参与合法数据的冲突判定）。
            _ => self.position_sec,
        }
    }
}

/// 合并结果。
#[derive(Debug, Clone, PartialEq)]
pub struct MergeOutcome {
    /// 最终一致集（双方都应落为这个集合）。
    pub merged: Vec<SyncRecord>,
    /// local 有而 remote 缺失/过旧的条数。
    pub uploaded: u32,
    /// remote 有而 local 缺失/过旧的条数。
    pub downloaded: u32,
    /// 合并键冲突次数（同键双方都有记录）。
    pub conflicts: u32,
}

/// 合并入口（纯函数，必须可无 IO 单测）。
///
/// 幂等：对同一输入反复调用产生相同结果；同步循环中"本地不变、远端已是上次合并结果"
/// 时 `uploaded == 0`（见 §6.1 幂等性测试）。
pub fn merge_records(local: Vec<SyncRecord>, remote: Vec<SyncRecord>) -> MergeOutcome {
    let mut by_key: HashMap<(String, String), (Option<SyncRecord>, Option<SyncRecord>)> =
        HashMap::new();
    for record in local {
        let key = (record.content_id.clone(), record.source_id.clone());
        by_key.entry(key).or_default().0 = Some(record);
    }
    for record in remote {
        let key = (record.content_id.clone(), record.source_id.clone());
        by_key.entry(key).or_default().1 = Some(record);
    }

    let mut merged = Vec::with_capacity(by_key.len());
    let mut uploaded = 0u32;
    let mut downloaded = 0u32;
    let mut conflicts = 0u32;

    for (_, (local_record, remote_record)) in by_key {
        match (local_record, remote_record) {
            (Some(local_record), None) => {
                merged.push(local_record);
                uploaded += 1;
            }
            (None, Some(remote_record)) => {
                merged.push(remote_record);
                downloaded += 1;
            }
            (Some(local_record), Some(remote_record)) => {
                conflicts += 1;
                if local_record == remote_record {
                    // 同一版本（幂等重合并）：没有需要上传/下载的变化。
                    merged.push(local_record);
                    continue;
                }
                let (winner, local_wins) = pick_winner(&local_record, &remote_record);
                if local_wins {
                    uploaded += 1;
                } else {
                    downloaded += 1;
                }
                merged.push(winner.clone());
            }
            (None, None) => unreachable!("merge key always has at least one candidate"),
        }
    }

    MergeOutcome {
        merged,
        uploaded,
        downloaded,
        conflicts,
    }
}

/// 判定同键两条记录谁是胜者。返回 `(胜者引用, 是否本地方胜出)`。
fn pick_winner<'a>(local: &'a SyncRecord, remote: &'a SyncRecord) -> (&'a SyncRecord, bool) {
    // 墓碑传播（rule 4）：墓碑 `updated_at` 不旧于对方 → 墓碑胜；对方更新 → "复活"。
    match (local.deleted, remote.deleted) {
        (true, true) => {}
        (true, false) => {
            if local.updated_at >= remote.updated_at {
                return (local, true);
            }
            return (remote, false);
        }
        (false, true) => {
            if remote.updated_at >= local.updated_at {
                return (remote, false);
            }
            return (local, true);
        }
        (false, false) => {}
    }

    // LWW（rule 2/3）：更新者胜；同秒（秒粒度相等）取进度更大者。
    if local.updated_at > remote.updated_at {
        (local, true)
    } else if remote.updated_at > local.updated_at {
        (remote, false)
    } else if local.progress() >= remote.progress() {
        (local, true)
    } else {
        (remote, false)
    }
}

/// 物理清理超过保留期的墓碑（rule 5）。`now` 为 Unix 秒。
pub fn purge_tombstones(
    records: Vec<SyncRecord>,
    now: i64,
    retention_days: i64,
) -> Vec<SyncRecord> {
    let cutoff = now - retention_days * 86_400;
    records
        .into_iter()
        .filter(|record| !(record.deleted && record.updated_at < cutoff))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        id: &str,
        content_id: &str,
        source_id: &str,
        content_type: &str,
        updated_at: i64,
        page_index: i64,
        position_sec: f64,
        scroll_pct: f64,
        deleted: bool,
    ) -> SyncRecord {
        SyncRecord {
            id: id.to_string(),
            content_id: content_id.to_string(),
            content_type: content_type.to_string(),
            title: format!("title-{id}"),
            cover: None,
            source_id: source_id.to_string(),
            chapter_id: None,
            chapter_title: None,
            page_index,
            position_sec,
            scroll_pct,
            updated_at,
            device_id: "dev-a".to_string(),
            deleted,
        }
    }

    #[test]
    fn normal_path_disjoint_sets_counts() {
        let local = vec![
            record("l1", "c1", "s1", "anime", 100, 0, 10.0, 0.0, false),
            record("l2", "c2", "s1", "anime", 100, 0, 20.0, 0.0, false),
            record("l3", "c3", "s1", "anime", 100, 0, 30.0, 0.0, false),
        ];
        let remote = vec![
            record("r1", "d1", "s1", "anime", 100, 0, 5.0, 0.0, false),
            record("r2", "d2", "s1", "anime", 100, 0, 6.0, 0.0, false),
        ];
        let outcome = merge_records(local, remote);
        assert_eq!(outcome.merged.len(), 5);
        assert_eq!(outcome.uploaded, 3);
        assert_eq!(outcome.downloaded, 2);
        assert_eq!(outcome.conflicts, 0);
    }

    #[test]
    fn lww_remote_newer_wins() {
        let local = vec![record("l1", "c1", "s1", "anime", 100, 0, 10.0, 0.0, false)];
        let remote = vec![record("r1", "c1", "s1", "anime", 200, 0, 1.0, 0.0, false)];
        let outcome = merge_records(local, remote);
        assert_eq!(outcome.merged.len(), 1);
        assert_eq!(outcome.merged[0].id, "r1");
        assert_eq!(outcome.merged[0].updated_at, 200);
        assert_eq!(outcome.uploaded, 0);
        assert_eq!(outcome.downloaded, 1);
        assert_eq!(outcome.conflicts, 1);
    }

    #[test]
    fn lww_local_newer_wins() {
        let local = vec![record("l1", "c1", "s1", "anime", 300, 0, 10.0, 0.0, false)];
        let remote = vec![record("r1", "c1", "s1", "anime", 200, 0, 99.0, 0.0, false)];
        let outcome = merge_records(local, remote);
        assert_eq!(outcome.merged[0].id, "l1");
        assert_eq!(outcome.uploaded, 1);
        assert_eq!(outcome.downloaded, 0);
        assert_eq!(outcome.conflicts, 1);
    }

    #[test]
    fn same_second_conflict_takes_larger_progress_per_type() {
        // 漫画：取 page_index 大者。
        let local_manga = vec![record("lm", "c1", "s1", "manga", 500, 5, 0.0, 0.0, false)];
        let remote_manga = vec![record("rm", "c1", "s1", "manga", 500, 8, 0.0, 0.0, false)];
        let outcome = merge_records(local_manga, remote_manga);
        assert_eq!(outcome.merged[0].id, "rm");
        assert_eq!(outcome.merged[0].page_index, 8);

        // 番剧：取 position_sec 大者。
        let local_anime = vec![record("la", "c2", "s1", "anime", 500, 0, 300.0, 0.0, false)];
        let remote_anime = vec![record("ra", "c2", "s1", "anime", 500, 0, 900.0, 0.0, false)];
        let outcome = merge_records(local_anime, remote_anime);
        assert_eq!(outcome.merged[0].id, "ra");
        assert_eq!(outcome.merged[0].position_sec, 900.0);

        // 小说：取 scroll_pct 大者。
        let local_novel = vec![record("ln", "c3", "s1", "novel", 500, 0, 0.0, 45.0, false)];
        let remote_novel = vec![record("rn", "c3", "s1", "novel", 500, 0, 0.0, 72.0, false)];
        let outcome = merge_records(local_novel, remote_novel);
        assert_eq!(outcome.merged[0].id, "rn");
        assert_eq!(outcome.merged[0].scroll_pct, 72.0);
    }

    #[test]
    fn tombstone_propagates_when_remote_deleted_is_newer() {
        let local = vec![record("l1", "c1", "s1", "anime", 100, 0, 10.0, 0.0, false)];
        let remote = vec![record("r1", "c1", "s1", "anime", 200, 0, 10.0, 0.0, true)];
        let outcome = merge_records(local, remote);
        assert_eq!(outcome.merged[0].id, "r1");
        assert!(outcome.merged[0].deleted);
        assert_eq!(outcome.downloaded, 1);
    }

    #[test]
    fn newer_normal_record_resurrects_over_tombstone() {
        let local = vec![record("l1", "c1", "s1", "anime", 300, 0, 10.0, 0.0, false)];
        let remote = vec![record("r1", "c1", "s1", "anime", 200, 0, 10.0, 0.0, true)];
        let outcome = merge_records(local, remote);
        assert_eq!(outcome.merged[0].id, "l1");
        assert!(!outcome.merged[0].deleted);
        assert_eq!(outcome.uploaded, 1);
    }

    #[test]
    fn purge_tombstones_removes_only_expired_ones() {
        let now = 1_000_000_000i64;
        let expired = record(
            "exp",
            "c1",
            "s1",
            "anime",
            now - 91 * 86_400,
            0,
            1.0,
            0.0,
            true,
        );
        let retained = record(
            "keep",
            "c2",
            "s1",
            "anime",
            now - 89 * 86_400,
            0,
            1.0,
            0.0,
            true,
        );
        let normal = record("norm", "c3", "s1", "anime", now, 0, 1.0, 0.0, false);
        let purged = purge_tombstones(
            vec![expired, retained, normal],
            now,
            TOMBSTONE_RETENTION_DAYS,
        );
        let ids: Vec<&str> = purged.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["keep", "norm"]);
    }

    #[test]
    fn idempotent_remerge_same_result_and_zero_uploads_on_second_sync() {
        let local = vec![
            record("l1", "c1", "s1", "anime", 100, 0, 10.0, 0.0, false),
            record("l2", "c2", "s1", "anime", 100, 0, 20.0, 0.0, false),
            record("l3", "c3", "s1", "anime", 100, 0, 30.0, 0.0, false),
        ];
        let remote = vec![
            record("r1", "d1", "s1", "anime", 100, 0, 5.0, 0.0, false),
            record("r2", "d2", "s1", "anime", 100, 0, 6.0, 0.0, false),
        ];

        let first = merge_records(local.clone(), remote.clone());
        assert_eq!(first.merged.len(), 5);
        assert_eq!(first.uploaded, 3);
        assert_eq!(first.downloaded, 2);

        // 合并结果顺序依赖 HashMap 迭代序（非确定性），比较"集合"而非 Vec 顺序。
        let sorted = |records: &[SyncRecord]| -> Vec<String> {
            let mut ids: Vec<String> = records.iter().map(|r| r.id.clone()).collect();
            ids.sort_unstable();
            ids
        };

        // 字面幂等：把合并结果再与旧远端合并，结果集合不变。
        let relit = merge_records(first.merged.clone(), remote.clone());
        assert_eq!(sorted(&relit.merged), sorted(&first.merged));

        // 同步循环幂等：第一次同步后远端文件变为 first.merged；第二次用**相同本地**
        // 重新拉取合并 → 无新上传、结果集稳定（spec §6.1 幂等性）。
        let second = merge_records(local.clone(), first.merged.clone());
        assert_eq!(sorted(&second.merged), sorted(&first.merged));
        assert_eq!(second.uploaded, 0);
        assert_eq!(second.downloaded, 2);

        // 第三次（本地也已收敛）→ 全零。
        let third = merge_records(second.merged.clone(), first.merged.clone());
        assert_eq!(sorted(&third.merged), sorted(&first.merged));
        assert_eq!(third.uploaded, 0);
        assert_eq!(third.downloaded, 0);
        assert_eq!(third.conflicts, 5);
    }

    #[test]
    fn tombstone_vs_tombstone_newer_wins() {
        let local = vec![record("l1", "c1", "s1", "anime", 100, 0, 0.0, 0.0, true)];
        let remote = vec![record("r1", "c1", "s1", "anime", 200, 0, 0.0, 0.0, true)];
        let outcome = merge_records(local, remote);
        assert_eq!(outcome.merged[0].id, "r1");
        assert!(outcome.merged[0].deleted);
    }

    #[test]
    fn merge_keys_are_content_id_plus_source_id() {
        // 相同 content_id、不同 source_id → 视为不同键，不冲突。
        let local = vec![record("l1", "c1", "sA", "anime", 100, 0, 1.0, 0.0, false)];
        let remote = vec![record("r1", "c1", "sB", "anime", 100, 0, 1.0, 0.0, false)];
        let outcome = merge_records(local, remote);
        assert_eq!(outcome.merged.len(), 2);
        assert_eq!(outcome.conflicts, 0);
    }
}
