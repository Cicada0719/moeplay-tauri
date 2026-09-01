//! FR-08 迁移框架测试（spec §6，共 19 项）。
//!
//! 全部使用 `tempfile` 创建隔离的 app_data_dir，覆盖正常路径、边界/异常路径
//! 与性能路径。迁移逻辑行覆盖率目标 100%。

use super::{commands::ensure_history_available, *};
use crate::db_sqlite::{get_or_create_device_id, HistoryDb, HistoryRepo};
use crate::domain::history::{ContentType, HistoryRecord};
use crate::migration::v1_to_v2::{map_v1_to_v2, V1HistoryEntry};
use rusqlite::params;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use tempfile::TempDir;

fn temp_app_dir() -> TempDir {
    TempDir::new().expect("create temp dir")
}

fn write_v1(dir: &Path, entries: &[Value]) {
    let path = dir.join("history.json");
    std::fs::write(&path, serde_json::to_vec_pretty(entries).unwrap()).unwrap();
}

macro_rules! map {
    () => { serde_json::Map::new() };
    ($($key:expr => $value:expr),* $(,)?) => {{
        let mut map = serde_json::Map::new();
        $(map.insert($key.to_string(), serde_json::json!($value));)*
        map
    }};
}

fn v1(
    content_id: &str,
    content_type: &str,
    title: &str,
    extra: serde_json::Map<String, Value>,
) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("content_id".to_string(), json!(content_id));
    map.insert("content_type".to_string(), json!(content_type));
    map.insert("title".to_string(), json!(title));
    for (key, value) in extra {
        map.insert(key, value);
    }
    Value::Object(map)
}

fn open_migrator(dir: &Path) -> (HistoryDb, Migrator) {
    let db = HistoryDb::open(dir).expect("open history db");
    let migrator = Migrator::new(db.clone(), dir.to_path_buf()).expect("create migrator");
    (db, migrator)
}

fn history_rows(db: &HistoryDb) -> Vec<HistoryRecord> {
    <HistoryDb as HistoryRepo>::list(db, None, None, 100_000, 0).unwrap()
}

/// 断言 `(content_id, source_id, chapter_id)` merge key 全表唯一（FR-08 “无重复记录”）。
fn assert_no_duplicates(db: &HistoryDb) {
    let conn = db.conn();
    let guard = conn.lock().unwrap();
    let dup: i64 = guard
        .query_row(
            "SELECT COUNT(*) FROM (SELECT content_id, source_id, chapter_id, COUNT(*) c \
             FROM history GROUP BY content_id, source_id, chapter_id HAVING c > 1)",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        dup, 0,
        "merge key (content_id, source_id, chapter_id) must be unique"
    );
}

// ---------------------------------------------------------------------------
// 正常路径
// ---------------------------------------------------------------------------

#[test]
fn test_open_creates_schema_and_version() {
    let dir = temp_app_dir();
    let db = HistoryDb::open(dir.path()).unwrap();
    assert_eq!(db.user_version().unwrap(), 2);
    let conn = db.conn();
    let guard = conn.lock().unwrap();
    for table in ["history", "migration_state", "migration_staging"] {
        let count: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                params![table],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "missing table {table}");
    }
    for index in ["idx_history_type_updated", "idx_history_merge_key"] {
        let count: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
                params![index],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "missing index {index}");
    }
}

#[test]
fn test_migrate_basic_records() {
    let dir = temp_app_dir();
    write_v1(
        dir.path(),
        &[
            v1("a1", "anime", "Anime One", map!("position_sec" => 123.5)),
            v1("a2", "bangumi", "Anime Two", map!("position_ms" => 45_000)),
            v1("m1", "manga", "Manga One", map!("page_index" => 12)),
            v1("m2", "comic", "Manga Two", map!("chapter_id" => "ch3")),
            v1("n1", "novel", "Novel One", map!("scroll_pct" => 45.5)),
            v1("n2", "novel", "Novel Two", map!()),
        ],
    );
    let (db, migrator) = open_migrator(dir.path());
    let report = migrator.run().unwrap();
    assert_eq!(report.status, MigrationStatus::Completed);
    assert_eq!(report.total, 6);
    assert_eq!(report.migrated, 6);

    let rows = history_rows(&db);
    assert_eq!(rows.len(), 6);
    let by_id: HashMap<&str, &HistoryRecord> =
        rows.iter().map(|r| (r.content_id.as_str(), r)).collect();
    assert_eq!(by_id["a1"].content_type, ContentType::Anime);
    assert_eq!(by_id["a1"].position_sec, 123.5);
    assert_eq!(by_id["a2"].position_sec, 45.0); // 45_000 ms → 45 s
    assert_eq!(by_id["m1"].page_index, 12);
    assert_eq!(by_id["m2"].content_type, ContentType::Manga);
    assert_eq!(by_id["n1"].content_type, ContentType::Novel);
    assert_eq!(by_id["n1"].scroll_pct, 45.5);
}

#[test]
fn test_backup_created_with_checksum() {
    let dir = temp_app_dir();
    write_v1(dir.path(), &[v1("a1", "anime", "T", map!())]);
    let original = std::fs::read(dir.path().join("history.json")).unwrap();

    let (_, migrator) = open_migrator(dir.path());
    migrator.run().unwrap();

    let backups = migrator.list_backups().unwrap();
    assert_eq!(backups.len(), 1);
    let backup_bytes = std::fs::read(&backups[0]).unwrap();
    assert_eq!(backup_bytes, original, "backup bytes must match source");
    assert!(dir.path().join("history.json.migrated").exists());
    assert!(!dir.path().join("history.json").exists());
}

#[test]
fn test_migration_report_counts() {
    let dir = temp_app_dir();
    let entries: Vec<Value> = (0..3)
        .map(|i| v1(&format!("c{i}"), "anime", &format!("T{i}"), map!()))
        .collect();
    write_v1(dir.path(), &entries);
    let (_, migrator) = open_migrator(dir.path());
    let report = migrator.run().unwrap();
    assert_eq!(report.status, MigrationStatus::Completed);
    assert_eq!(report.total, 3);
    assert_eq!(report.migrated, 3);
    assert!(report.backup_path.is_some());
}

#[test]
fn test_idempotent_rerun() {
    let dir = temp_app_dir();
    write_v1(dir.path(), &[v1("c1", "anime", "T", map!())]);
    let (db, migrator) = open_migrator(dir.path());
    let first = migrator.run().unwrap();
    assert_eq!(first.status, MigrationStatus::Completed);

    let second = migrator.run().unwrap();
    assert_eq!(second.status, MigrationStatus::NotNeeded);
    assert_eq!(history_rows(&db).len(), 1);

    let conn = db.conn();
    let guard = conn.lock().unwrap();
    let dup: i64 = guard
        .query_row(
            "SELECT COUNT(*) FROM (SELECT content_id, source_id, chapter_id, COUNT(*) c \
             FROM history GROUP BY content_id, source_id, chapter_id HAVING c > 1)",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(dup, 0);
}

#[test]
fn test_history_repo_list_filter_search_pagination() {
    let dir = temp_app_dir();
    let db = HistoryDb::open(dir.path()).unwrap();
    for i in 0..10 {
        let content_type = match i % 3 {
            0 => ContentType::Anime,
            1 => ContentType::Manga,
            _ => ContentType::Novel,
        };
        db.upsert(&HistoryRecord {
            id: format!("id{i}"),
            content_id: format!("c{i}"),
            content_type,
            title: format!("Title {i}"),
            cover: None,
            source_id: "src".into(),
            chapter_id: None,
            chapter_title: None,
            page_index: 0,
            position_sec: i as f64,
            scroll_pct: 0.0,
            updated_at: 1_600_000_000_000 + i,
            device_id: "dev".into(),
            deleted: false,
        })
        .unwrap();
    }

    let manga =
        <HistoryDb as HistoryRepo>::list(&db, Some(ContentType::Manga), None, 100, 0).unwrap();
    assert!(!manga.is_empty());
    assert!(manga.iter().all(|r| r.content_type == ContentType::Manga));

    let kw = <HistoryDb as HistoryRepo>::list(&db, None, Some("Title 1"), 100, 0).unwrap();
    assert!(!kw.is_empty());
    assert!(kw.iter().all(|r| r.title.contains("Title 1")));

    let page = <HistoryDb as HistoryRepo>::list(&db, None, None, 3, 2).unwrap();
    assert_eq!(page.len(), 3);
    assert!(page.windows(2).all(|w| w[0].updated_at > w[1].updated_at));

    let total = <HistoryDb as HistoryRepo>::count(&db, None).unwrap();
    assert_eq!(total, 10);
}

#[test]
fn test_tombstone_and_list_exclusion() {
    let dir = temp_app_dir();
    let db = HistoryDb::open(dir.path()).unwrap();
    db.upsert(&HistoryRecord {
        id: "r1".into(),
        content_id: "c1".into(),
        content_type: ContentType::Anime,
        title: "T".into(),
        cover: None,
        source_id: "src".into(),
        chapter_id: None,
        chapter_title: None,
        page_index: 0,
        position_sec: 0.0,
        scroll_pct: 0.0,
        updated_at: 1_000,
        device_id: "dev".into(),
        deleted: false,
    })
    .unwrap();
    assert_eq!(history_rows(&db).len(), 1);

    db.tombstone("r1").unwrap();
    assert_eq!(
        history_rows(&db).len(),
        0,
        "tombstoned row must be hidden from list"
    );

    let tombstones = <HistoryDb as HistoryRepo>::list_tombstones_since(&db, 0).unwrap();
    assert_eq!(tombstones.len(), 1);
    assert!(tombstones[0].deleted);
    assert!(
        tombstones[0].updated_at > 1_000,
        "updated_at must be refreshed on tombstone"
    );

    let fetched = db.get("r1").unwrap().unwrap();
    assert!(fetched.deleted);
}

// ---------------------------------------------------------------------------
// 边界 / 异常路径
// ---------------------------------------------------------------------------

#[test]
fn test_crash_resume_no_duplicates() {
    let dir = temp_app_dir();
    // 3 批：1200 条（500 + 500 + 200）
    let entries: Vec<Value> = (0..1200)
        .map(|i| {
            v1(
                &format!("c{i}"),
                if i % 2 == 0 { "anime" } else { "manga" },
                &format!("T{i}"),
                map!("updated_at" => 1_600_000_000_i64 + i),
            )
        })
        .collect();
    write_v1(dir.path(), &entries);

    let db = HistoryDb::open(dir.path()).unwrap();
    let mut migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    migrator.set_batch_hook(Some(Arc::new(|batch| {
        if batch == 2 {
            panic!("injected crash after batch 2");
        }
        Ok(())
    })));
    let handle = std::thread::spawn(move || migrator.run());
    assert!(handle.join().is_err(), "run should have panicked");

    // 重建 Migrator 断点续迁
    let migrator2 = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    let report = migrator2.run().unwrap();
    assert_eq!(report.status, MigrationStatus::Completed);
    assert_eq!(report.total, 1200);

    let rows = history_rows(&db);
    assert_eq!(rows.len(), 1200);
    let conn = db.conn();
    let guard = conn.lock().unwrap();
    let dup: i64 = guard
        .query_row(
            "SELECT COUNT(*) FROM (SELECT content_id, source_id, chapter_id, COUNT(*) c \
             FROM history GROUP BY content_id, source_id, chapter_id HAVING c > 1)",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(dup, 0);
}

#[test]
fn test_migrate_failure_rollback() {
    let dir = temp_app_dir();
    let entries: Vec<Value> = (0..1200)
        .map(|i| v1(&format!("c{i}"), "anime", &format!("T{i}"), map!()))
        .collect();
    write_v1(dir.path(), &entries);

    let db = HistoryDb::open(dir.path()).unwrap();
    let mut migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    migrator.set_batch_hook(Some(Arc::new(|batch| {
        if batch == 3 {
            Err(MigrationError::Migration("injected failure".into()))
        } else {
            Ok(())
        }
    })));
    let error = migrator.run().unwrap_err();
    assert!(
        error.to_string().contains("injected failure"),
        "got: {error}"
    );

    let conn = db.conn();
    let guard = conn.lock().unwrap();
    let status: String = guard
        .query_row("SELECT status FROM migration_state WHERE id=1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(status, "rolled_back");
    let history_count: i64 = guard
        .query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
        .unwrap();
    assert_eq!(history_count, 0, "all this-run writes must be cleared");
    let staging: i64 = guard
        .query_row("SELECT COUNT(*) FROM migration_staging", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(staging, 0);
    drop(guard);

    // 回滚后 v1 源文件不得被重命名（只有 status=completed 之后才执行重命名）。
    assert!(dir.path().join("history.json").exists());
    assert!(!dir.path().join("history.json.migrated").exists());

    let backups = migrator.list_backups().unwrap();
    assert_eq!(
        backups.len(),
        1,
        "backup file must be retained after rollback"
    );
}

#[test]
fn test_rollback_restores_replaced_rows() {
    let dir = temp_app_dir();
    let db = HistoryDb::open(dir.path()).unwrap();

    // 预置两条历史，后续迁移会以更新的 updated_at 覆盖它们（UPDATE / Replaced）。
    let seed = |id: &str, content_id: &str, title: &str, updated_at: i64| HistoryRecord {
        id: id.into(),
        content_id: content_id.into(),
        content_type: ContentType::Anime,
        title: title.into(),
        cover: None,
        source_id: "src".into(),
        chapter_id: None,
        chapter_title: None,
        page_index: 0,
        position_sec: 0.0,
        scroll_pct: 0.0,
        updated_at,
        device_id: "dev".into(),
        deleted: false,
    };
    db.upsert(&seed("pre-1", "c1", "Old Title 1", 1_000))
        .unwrap();
    db.upsert(&seed("pre-2", "c2", "Old Title 2", 1_000))
        .unwrap();

    // v1 数据：同一 merge key，updated_at 更新 → 迁移会触发 UPDATE。
    write_v1(
        dir.path(),
        &[
            v1(
                "c1",
                "anime",
                "New Title 1",
                map!("source_id" => "src", "updated_at" => 2_000),
            ),
            v1(
                "c2",
                "anime",
                "New Title 2",
                map!("source_id" => "src", "updated_at" => 2_000),
            ),
        ],
    );

    let mut migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    migrator.set_batch_hook(Some(Arc::new(|batch| {
        if batch == 1 {
            Err(MigrationError::Migration("injected failure".into()))
        } else {
            Ok(())
        }
    })));
    let error = migrator.run().unwrap_err();
    assert!(
        error.to_string().contains("injected failure"),
        "got: {error}"
    );

    // 回滚后：被 UPDATE 覆盖的原行必须按快照还原（id 不变、旧值恢复）。
    let rows = history_rows(&db);
    assert_eq!(rows.len(), 2, "no extra rows may remain after rollback");
    let by_id: HashMap<&str, &HistoryRecord> = rows.iter().map(|r| (r.id.as_str(), r)).collect();
    let restored1 = by_id["pre-1"];
    assert_eq!(restored1.content_id, "c1");
    assert_eq!(restored1.title, "Old Title 1");
    assert_eq!(restored1.updated_at, 1_000);
    let restored2 = by_id["pre-2"];
    assert_eq!(restored2.title, "Old Title 2");
    assert_eq!(restored2.updated_at, 1_000);

    let conn = db.conn();
    let guard = conn.lock().unwrap();
    let staging: i64 = guard
        .query_row("SELECT COUNT(*) FROM migration_staging", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(staging, 0, "staging must be cleared after rollback");
    drop(guard);
}

#[test]
fn test_restore_from_backup() {
    let dir = temp_app_dir();
    let entries: Vec<Value> = (0..10)
        .map(|i| v1(&format!("c{i}"), "anime", &format!("T{i}"), map!()))
        .collect();
    write_v1(dir.path(), &entries);

    let db = HistoryDb::open(dir.path()).unwrap();
    let mut migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    migrator.set_batch_hook(Some(Arc::new(|batch| {
        if batch == 1 {
            Err(MigrationError::Migration("boom".into()))
        } else {
            Ok(())
        }
    })));
    assert!(migrator.run().is_err());
    assert_eq!(history_rows(&db).len(), 0);

    let backups = migrator.list_backups().unwrap();
    assert_eq!(backups.len(), 1);

    let migrator2 = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    let report = migrator2.restore_from_backup(&backups[0]).unwrap();
    assert_eq!(report.status, MigrationStatus::Completed);
    assert_eq!(report.total, 10);
    assert_eq!(history_rows(&db).len(), 10);
    // restore 复用备份文件作为 backup_path，不应生成第二个备份。
    assert_eq!(
        migrator2.list_backups().unwrap().len(),
        1,
        "restore must not create a second backup"
    );
    // 恢复成功后备份内容写回 v1 路径：用户能看到数据"回来了"，
    // 避免误以为恢复失败。
    assert!(
        dir.path().join("history.json").exists(),
        "restore must write backup content back to history.json"
    );
    assert_eq!(
        std::fs::read(dir.path().join("history.json")).unwrap(),
        std::fs::read(&backups[0]).unwrap(),
        "history.json must equal the restored backup content"
    );
}

#[test]
fn test_corrupted_v1_json() {
    let dir = temp_app_dir();
    std::fs::write(dir.path().join("history.json"), "{ this is not valid json").unwrap();

    let db = HistoryDb::open(dir.path()).unwrap();
    let migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    let error = migrator.run().unwrap_err();
    assert!(
        error.to_string().contains("parse") || error.to_string().contains("expected"),
        "error should mention parse failure: {error}"
    );

    let backups = migrator.list_backups().unwrap();
    assert_eq!(
        backups.len(),
        1,
        "backup must be generated even for corrupted v1"
    );

    let conn = db.conn();
    let guard = conn.lock().unwrap();
    let status: String = guard
        .query_row("SELECT status FROM migration_state WHERE id=1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(status, "rolled_back");
    drop(guard);
    // 解析失败 → 回滚，v1 文件不得被重命名。
    assert!(dir.path().join("history.json").exists());
    assert!(!dir.path().join("history.json.migrated").exists());
}

#[test]
fn test_partial_invalid_entries_skipped() {
    let dir = temp_app_dir();
    let mut entries = Vec::new();
    for i in 0..10 {
        if i == 3 || i == 7 {
            // 缺 content_id → 跳过
            entries.push(v1("", "anime", &format!("Bad {i}"), map!()));
        } else {
            entries.push(v1(&format!("c{i}"), "anime", &format!("Title {i}"), map!()));
        }
    }
    write_v1(dir.path(), &entries);
    let (db, migrator) = open_migrator(dir.path());
    let report = migrator.run().unwrap();
    assert_eq!(report.status, MigrationStatus::Completed);
    assert_eq!(report.total, 8);
    assert_eq!(history_rows(&db).len(), 8);
}

#[test]
fn test_empty_v1() {
    let dir = temp_app_dir();
    write_v1(dir.path(), &[]);
    let (db, migrator) = open_migrator(dir.path());
    let report = migrator.run().unwrap();
    assert_eq!(report.status, MigrationStatus::Completed);
    assert_eq!(report.total, 0);
    assert_eq!(history_rows(&db).len(), 0);
    // 行为锁定：空备份仍生成，源文件重命名
    assert_eq!(migrator.list_backups().unwrap().len(), 1);
    assert!(dir.path().join("history.json.migrated").exists());
    // 空库场景不残留 migration_staging 记录。
    let conn = db.conn();
    let guard = conn.lock().unwrap();
    let staging: i64 = guard
        .query_row("SELECT COUNT(*) FROM migration_staging", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(staging, 0, "empty migration must clear staging rows");
    drop(guard);
}

#[test]
fn test_no_v1_data() {
    let dir = temp_app_dir();
    let (_, migrator) = open_migrator(dir.path());
    assert_eq!(migrator.check().unwrap(), MigrationStatus::NotNeeded);
    let report = migrator.run().unwrap();
    assert_eq!(report.status, MigrationStatus::NotNeeded);
    assert_eq!(migrator.list_backups().unwrap().len(), 0);
}

#[test]
fn test_missing_fields_defaults() {
    let dir = temp_app_dir();
    write_v1(
        dir.path(),
        &[v1(
            "c1",
            "anime",
            "Title",
            map!("updated_at" => 1_600_000_000_i64),
        )],
    );
    let (db, migrator) = open_migrator(dir.path());
    migrator.run().unwrap();
    let rows = history_rows(&db);
    assert_eq!(rows.len(), 1);
    let rec = &rows[0];
    assert_eq!(rec.cover, None);
    assert_eq!(rec.chapter_id, None);
    assert_eq!(rec.chapter_title, None);
    assert_eq!(rec.position_sec, 0.0);
    assert_eq!(rec.scroll_pct, 0.0);
    assert_eq!(rec.page_index, 0);
    assert_eq!(rec.source_id, "unknown");
    assert_eq!(rec.updated_at, 1_600_000_000_000);
    assert!(!rec.deleted);
}

#[test]
fn test_device_id_stable() {
    let dir = temp_app_dir();
    let first = get_or_create_device_id(dir.path()).unwrap();
    let second = get_or_create_device_id(dir.path()).unwrap();
    assert_eq!(first, second);
    std::fs::remove_file(dir.path().join("device.id")).unwrap();
    let third = get_or_create_device_id(dir.path()).unwrap();
    assert_ne!(first, third);
}

#[test]
fn test_updated_at_unit_normalization() {
    // 秒级 → 毫秒
    let entry_seconds = v1("s1", "anime", "S", map!("updated_at" => 1_600_000_000_i64));
    let rec = map_v1_to_v2(
        &serde_json::from_value::<V1HistoryEntry>(entry_seconds).unwrap(),
        "dev",
    )
    .unwrap();
    assert_eq!(rec.updated_at, 1_600_000_000_000);

    // 毫秒级 → 不重复放大
    let entry_millis = v1(
        "m1",
        "anime",
        "M",
        map!("updated_at" => 1_600_000_000_000_i64),
    );
    let rec2 = map_v1_to_v2(
        &serde_json::from_value::<V1HistoryEntry>(entry_millis).unwrap(),
        "dev",
    )
    .unwrap();
    assert_eq!(rec2.updated_at, 1_600_000_000_000);
}

#[test]
fn test_command_gating() {
    assert!(ensure_history_available(&MigrationStatus::Completed).is_ok());
    assert!(ensure_history_available(&MigrationStatus::NotNeeded).is_ok());
    let pending = ensure_history_available(&MigrationStatus::InProgress).unwrap_err();
    assert_eq!(pending, "MIGRATION_PENDING");
    assert!(ensure_history_available(&MigrationStatus::Failed("x".into())).is_err());
    assert!(ensure_history_available(&MigrationStatus::RolledBack).is_err());
    assert!(ensure_history_available(&MigrationStatus::Pending).is_err());
}

// ---------------------------------------------------------------------------
// 迁移回滚回归测试
// ---------------------------------------------------------------------------

#[test]
fn test_progress_event_name_matches_spec() {
    // spec §4.2 步骤 8：事件名必须是 `migration://progress`（前端订阅方 wire 契约）。
    // 步骤 7 的示例文本写作 `migration-progress`（连字符）属笔误；实现以步骤 8 为准。
    // spec 属"禁止修改清单"，实现口径由常量注释与 PR 说明表达。
    assert_eq!(MIGRATION_PROGRESS_EVENT, "migration://progress");
}

#[test]
fn test_final_progress_sink_emits_completed() {
    // 600 条 → 2 批；最后一个批次提交后也必须触发 progress_sink 并发出 Completed 事件，
    // 前端进度页据此从 InProgress 收敛到终态，而不是永远停在 InProgress。
    let dir = temp_app_dir();
    let entries: Vec<Value> = (0..600)
        .map(|i| v1(&format!("c{i}"), "anime", &format!("T{i}"), map!()))
        .collect();
    write_v1(dir.path(), &entries);

    let db = HistoryDb::open(dir.path()).unwrap();
    let mut migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    let reports: Arc<std::sync::Mutex<Vec<MigrationReport>>> = Arc::new(Default::default());
    let captured = Arc::clone(&reports);
    migrator.set_progress_sink(Some(Arc::new(move |report: &MigrationReport| {
        captured.lock().unwrap().push(report.clone());
    })));

    let report = migrator.run().unwrap();
    assert_eq!(report.status, MigrationStatus::Completed);

    let all = reports.lock().unwrap();
    assert!(!all.is_empty(), "progress sink must fire at least once");
    assert_eq!(
        all.last().unwrap().status,
        MigrationStatus::Completed,
        "last progress event must be Completed, got {:?}",
        all.last().unwrap().status
    );
    assert!(all.iter().all(|r| matches!(
        r.status,
        MigrationStatus::InProgress | MigrationStatus::Completed
    )));
}

#[test]
fn test_merge_key_null_chapter_semantics() {
    // 同 content_id + source_id、无 chapter_id → 合并为单行（最新 updated_at 胜出），无重复。
    let dir = temp_app_dir();
    write_v1(
        dir.path(),
        &[
            v1(
                "c1",
                "anime",
                "Old",
                map!("source_id" => "src", "updated_at" => 1_600_000_000_000_i64),
            ),
            v1(
                "c1",
                "anime",
                "New",
                map!("source_id" => "src", "updated_at" => 1_600_000_000_001_i64),
            ),
        ],
    );
    let (db, migrator) = open_migrator(dir.path());
    let report = migrator.run().unwrap();
    assert_eq!(report.status, MigrationStatus::Completed);
    let rows = history_rows(&db);
    assert_eq!(rows.len(), 1, "无 chapter_id 的同 merge key 必须合并为一行");
    assert_eq!(rows[0].title, "New");
    assert_eq!(rows[0].updated_at, 1_600_000_000_001);
    assert_no_duplicates(&db);

    // NULL 与 'ch1' 分属不同 merge key → 两条独立行（各自保留），不能互相覆盖/合并。
    let dir2 = temp_app_dir();
    write_v1(
        dir2.path(),
        &[
            v1("c2", "anime", "Null chapter", map!("source_id" => "src")),
            v1(
                "c2",
                "anime",
                "With chapter",
                map!("source_id" => "src", "chapter_id" => "ch1"),
            ),
        ],
    );
    let (db2, migrator2) = open_migrator(dir2.path());
    let report2 = migrator2.run().unwrap();
    assert_eq!(report2.status, MigrationStatus::Completed);
    let rows2 = history_rows(&db2);
    assert_eq!(
        rows2.len(),
        2,
        "chapter_id NULL 与具体值必须是不同 merge key"
    );
    assert_no_duplicates(&db2);
}

#[test]
fn test_rollback_deletes_row_inserted_then_replaced() {
    // 同一 merge key 两条 v1 记录：先 INSERT 再被更新的记录 REPLACE。
    // 回滚时必须按“首次登记 origin”处理：该行由迁移创建（inserted），
    // 即便同批次内又被 Replace，也必须 DELETE，而不是用中间态快照还原成残留行。
    let dir = temp_app_dir();
    write_v1(
        dir.path(),
        &[
            v1(
                "c1",
                "anime",
                "Old",
                map!("source_id" => "src", "updated_at" => 1_000),
            ),
            v1(
                "c1",
                "anime",
                "New",
                map!("source_id" => "src", "updated_at" => 2_000),
            ),
        ],
    );
    let db = HistoryDb::open(dir.path()).unwrap();
    let mut migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    migrator.set_batch_hook(Some(Arc::new(|_batch| {
        Err(MigrationError::Migration("injected failure".into()))
    })));
    let error = migrator.run().unwrap_err();
    assert!(
        error.to_string().contains("injected failure"),
        "got: {error}"
    );
    assert_eq!(
        history_rows(&db).len(),
        0,
        "inserted-then-replaced row must be deleted on rollback, not restored"
    );
    let conn = db.conn();
    let guard = conn.lock().unwrap();
    let staging: i64 = guard
        .query_row("SELECT COUNT(*) FROM migration_staging", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(staging, 0);
    drop(guard);
}

#[test]
fn test_crash_resume_keeps_original_replaced_snapshot() {
    // 崩溃→断点续迁场景：batch 1 REPLACE 预置行并崩溃，batch 2 再次 REPLACE 同一行。
    // staging 必须保留最初 pre-migration 快照（而非 batch 1 的中间态），
    // 否则续迁失败回滚只能还原到中间值，违背“回滚彻底”。
    let dir = temp_app_dir();
    let db = HistoryDb::open(dir.path()).unwrap();
    db.upsert(&HistoryRecord {
        id: "pre-1".into(),
        content_id: "c1".into(),
        content_type: ContentType::Anime,
        title: "Original".into(),
        cover: None,
        source_id: "src".into(),
        chapter_id: None,
        chapter_title: None,
        page_index: 0,
        position_sec: 0.0,
        scroll_pct: 0.0,
        updated_at: 1_000,
        device_id: "dev".into(),
        deleted: false,
    })
    .unwrap();

    // v1: 第 1 条 REPLACE pre-1（batch 1），第 502 条会在 batch 2 再次 REPLACE 同一 merge key。
    let mut entries = vec![v1(
        "c1",
        "anime",
        "New1",
        map!("source_id" => "src", "updated_at" => 2_000),
    )];
    for i in 1..501 {
        entries.push(v1(
            &format!("f{i}"),
            "anime",
            &format!("F{i}"),
            map!("updated_at" => 1_600_000_000_i64 + i),
        ));
    }
    entries.push(v1(
        "c1",
        "anime",
        "New2",
        map!("source_id" => "src", "updated_at" => 3_000),
    ));
    assert_eq!(
        entries.len(),
        502,
        "batch 1 = 500 条，New2 必须落在 batch 2"
    );
    write_v1(dir.path(), &entries);

    // 第一次运行：batch 1 提交后崩溃（panic）。
    let mut migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    migrator.set_batch_hook(Some(Arc::new(|batch| {
        if batch == 1 {
            panic!("injected crash after batch 1");
        }
        Ok(())
    })));
    let handle = std::thread::spawn(move || migrator.run());
    assert!(handle.join().is_err(), "run should have panicked");

    // 断点续迁：batch 2 再次 REPLACE c1，随后注入失败触发回滚。
    let mut migrator2 = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    migrator2.set_batch_hook(Some(Arc::new(|_batch| {
        Err(MigrationError::Migration("injected failure".into()))
    })));
    let error = migrator2.run().unwrap_err();
    assert!(
        error.to_string().contains("injected failure"),
        "got: {error}"
    );

    // 回滚必须还原到 pre-migration 原始值（title=Original, updated_at=1000），
    // 而非 batch 1 的中间态（New1 @ 2000）。
    let rows = history_rows(&db);
    assert_eq!(
        rows.len(),
        1,
        "only the pre-existing row may survive rollback"
    );
    assert_eq!(rows[0].id, "pre-1");
    assert_eq!(rows[0].title, "Original");
    assert_eq!(rows[0].updated_at, 1_000);
}

#[test]
fn test_history_gate_refresh_after_completion() {
    // 启动迁移在 spawn_blocking 完成后，内存门控可能仍停留在 InProgress；
    // history_* 命令入口通过 gate_history 的非阻塞落盘核对刷新门控，避免误报 MIGRATION_PENDING。
    let dir = temp_app_dir();
    write_v1(dir.path(), &[v1("c1", "anime", "T", map!())]);
    let db = HistoryDb::open(dir.path()).unwrap();
    let migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    migrator.run().unwrap();

    let state = super::commands::AppState {
        migrator: Some(migrator.clone()),
        history: Some(db),
        migration_status: Arc::new(RwLock::new(MigrationStatus::InProgress)),
    };
    assert!(ensure_history_available(&state.migration_status.read().unwrap()).is_err());

    let status = super::commands::gate_history(&state).unwrap();
    assert_eq!(status, MigrationStatus::NotNeeded);
    assert_eq!(
        *state.migration_status.read().unwrap(),
        MigrationStatus::NotNeeded
    );
    assert!(ensure_history_available(&status).is_ok());
}

// ---------------------------------------------------------------------------
// 迁移条数回归测试
// ---------------------------------------------------------------------------

#[test]
fn test_count_validation_with_pre_existing_rows() {
    // 条数校验口径（item 3）：只统计“本次迁移写入的新行”（staging `kind='inserted'`）
    // + 迁移前基线，不含迁移前已存在的其他来源/设备记录（spec §4.2.d“含本批次前
    // 已有数据需换算”）。旧实现 `COUNT(*) WHERE device_id=?` 会把同设备手动 upsert /
    // 他设备记录误算，导致合法迁移被误判失败；此处锁定新口径行为。
    let dir = temp_app_dir();
    let db = HistoryDb::open(dir.path()).unwrap();
    let seed = |id: &str, content_id: &str, device_id: &str, updated_at: i64| HistoryRecord {
        id: id.into(),
        content_id: content_id.into(),
        content_type: ContentType::Anime,
        title: "Pre".into(),
        cover: None,
        source_id: "src".into(),
        chapter_id: None,
        chapter_title: None,
        page_index: 0,
        position_sec: 0.0,
        scroll_pct: 0.0,
        updated_at,
        device_id: device_id.into(),
        deleted: false,
    };
    // 迁移前已存在的行：
    //  - manual-1：同设备手动 upsert、merge key 不在 v1 中；
    //  - other-dev：他设备、merge key 与 v1 的 c1 相同（会被 REPLACE 覆盖）；
    //  - pre-1：同设备、merge key 不在 v1 中。
    db.upsert(&seed("manual-1", "manual", "dev", 100)).unwrap();
    db.upsert(&seed("other-dev", "c1", "other-device", 100))
        .unwrap();
    db.upsert(&seed("pre-1", "pre1", "dev", 1_000)).unwrap();

    write_v1(
        dir.path(),
        &[
            v1(
                "c1",
                "anime",
                "New",
                map!("source_id" => "src", "updated_at" => 2_000),
            ),
            v1(
                "c2",
                "anime",
                "Fresh",
                map!("source_id" => "src", "updated_at" => 3_000),
            ),
        ],
    );
    let migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    let report = migrator.run().unwrap();
    assert_eq!(report.status, MigrationStatus::Completed);
    // 最终 4 行：manual-1、pre-1 未动，other-dev 被 c1 REPLACE（merge key 匹配不看
    // device），c2 新插入。基线 3 + 本次插入 1 = 4。
    assert_eq!(history_rows(&db).len(), 4);
    assert_no_duplicates(&db);
}

#[test]
fn test_rollback_insert_then_replace_across_batches() {
    // 回滚边界（item 1）：同一批次内“先 INSERT 后又被 UPDATE”的链，跨批次
    // （崩溃→续迁）也必须以首次登记的 origin（'inserted'）为准——回滚 DELETE 该行，
    // 而不是用中间态快照还原成残留行。
    let dir = temp_app_dir();
    // entries[0]=c1 在 batch 1 INSERT；entries[500]=c1 在 batch 2 再次覆盖（REPLACE）。
    let mut entries = vec![v1(
        "c1",
        "anime",
        "Old",
        map!("source_id" => "src", "updated_at" => 1_000),
    )];
    for i in 1..500 {
        entries.push(v1(
            &format!("f{i}"),
            "anime",
            &format!("F{i}"),
            map!("source_id" => "src", "updated_at" => 1_600_000_000_i64 + i),
        ));
    }
    entries.push(v1(
        "c1",
        "anime",
        "New",
        map!("source_id" => "src", "updated_at" => 2_000),
    ));
    assert_eq!(
        entries.len(),
        501,
        "batch 1 = 500 条，c1@New 必须落在 batch 2"
    );
    write_v1(dir.path(), &entries);

    let db = HistoryDb::open(dir.path()).unwrap();
    // 第一次运行：batch 1 提交后崩溃（c1 已 INSERT 并登记 staging 'inserted'）。
    let mut migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    migrator.set_batch_hook(Some(Arc::new(|batch| {
        if batch == 1 {
            panic!("injected crash after batch 1");
        }
        Ok(())
    })));
    let handle = std::thread::spawn(move || migrator.run());
    assert!(handle.join().is_err(), "run should have panicked");

    // 断点续迁：batch 2 覆盖 c1（staging 保持 'inserted' origin），随后注入失败触发回滚。
    let mut migrator2 = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    migrator2.set_batch_hook(Some(Arc::new(|batch| {
        if batch == 1 {
            Err(MigrationError::Migration("injected failure".into()))
        } else {
            Ok(())
        }
    })));
    let error = migrator2.run().unwrap_err();
    assert!(
        error.to_string().contains("injected failure"),
        "got: {error}"
    );

    // 回滚：所有行都是迁移创建（'inserted'），全部删除，不能残留 c1 的中间态快照行。
    assert_eq!(
        history_rows(&db).len(),
        0,
        "inserted-then-replaced row must be deleted on rollback, not restored"
    );
    let conn = db.conn();
    let guard = conn.lock().unwrap();
    let staging: i64 = guard
        .query_row("SELECT COUNT(*) FROM migration_staging", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(staging, 0);
    drop(guard);
}

#[test]
fn test_concurrent_runs_serialized() {
    // 并发防护（item 2）：setup 的 spawn_blocking 迁移与 migration_run 命令共享同一把
    // `run_lock`（Migrator 克隆共享 Arc）。第一个 run 持有锁期间，第二个 run 必须阻塞
    // 等待，不能两个 run 并发交错写库 / 更新 migration_state / 推送进度。
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::sync::{Condvar, Mutex};
    use std::time::Duration;

    let dir = temp_app_dir();
    let entries: Vec<Value> = (0..600)
        .map(|i| v1(&format!("c{i}"), "anime", &format!("T{i}"), map!()))
        .collect();
    write_v1(dir.path(), &entries);

    let db = HistoryDb::open(dir.path()).unwrap();
    let migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();

    // run1 在 batch 1 提交后通过 hook 阻塞，模拟“迁移仍在进行中且持有 run_lock”。
    let started = Arc::new(AtomicUsize::new(0));
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let mut m1 = migrator.clone();
    let started_hook = Arc::clone(&started);
    let release_hook = Arc::clone(&release);
    m1.set_batch_hook(Some(Arc::new(move |batch| {
        if batch == 1 {
            started_hook.store(1, Ordering::SeqCst);
            let (lock, cvar) = &*release_hook;
            let mut released = lock.lock().unwrap();
            while !*released {
                released = cvar.wait(released).unwrap();
            }
        }
        Ok(())
    })));

    let m2 = migrator.clone();
    let (run2_done_tx, run2_done_rx) = mpsc::channel::<()>();
    let handle1 = std::thread::spawn(move || m1.run());
    // 等 run1 进入批处理（batch 1 已提交、hook 阻塞中、持有 run_lock）。
    let mut spins = 0;
    while started.load(Ordering::SeqCst) == 0 {
        std::thread::sleep(Duration::from_millis(5));
        spins += 1;
        assert!(spins < 1000, "run1 should reach the batch hook");
    }
    let handle2 = std::thread::spawn(move || {
        let result = m2.run();
        let _ = run2_done_tx.send(());
        result
    });

    // run2 与 run1 共享 run_lock；run1 未放行前 run2 必须阻塞（不能提前完成）。
    assert!(
        run2_done_rx
            .recv_timeout(Duration::from_millis(150))
            .is_err(),
        "concurrent run must block until the first run finishes"
    );

    // 放行 run1，两个 run 依次完成，最终状态一致、无重复。
    {
        let (lock, cvar) = &*release;
        *lock.lock().unwrap() = true;
        cvar.notify_all();
    }
    let r1 = handle1.join().unwrap().unwrap();
    assert_eq!(r1.status, MigrationStatus::Completed);
    assert_eq!(r1.total, 600);

    run2_done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("second run completes after first releases the lock");
    let r2 = handle2.join().unwrap().unwrap();
    // 第一个 run 已完成 → 第二个 run 幂等返回 NotNeeded。
    assert_eq!(r2.status, MigrationStatus::NotNeeded);
    assert_eq!(history_rows(&db).len(), 600);
    assert_no_duplicates(&db);
}

// ---------------------------------------------------------------------------
// 迁移状态回归测试
// ---------------------------------------------------------------------------

#[test]
fn test_gate_history_returns_pending_when_lock_held() {
    // item 2：后台迁移持有数据库连接锁时，`history_*` 命令入口必须立即返回
    // `MIGRATION_PENDING`，绝不阻塞等待锁释放。
    let dir = temp_app_dir();
    write_v1(dir.path(), &[v1("c1", "anime", "T", map!())]);
    let db = HistoryDb::open(dir.path()).unwrap();
    let migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    let state = super::commands::AppState {
        migrator: Some(migrator),
        history: Some(db.clone()),
        // 内存门控停在 InProgress（后台迁移进行中），与磁盘锁无关。
        migration_status: Arc::new(RwLock::new(MigrationStatus::InProgress)),
    };
    // 模拟后台迁移正在写库：本线程持有连接锁，try_check 的 try_lock 将 WouldBlock。
    let conn_arc = db.conn();
    let lock_guard = conn_arc.lock().unwrap();
    let result = super::commands::gate_history(&state);
    drop(lock_guard);
    assert_eq!(
        result.unwrap_err(),
        "MIGRATION_PENDING",
        "gate must return MIGRATION_PENDING immediately when the DB lock is held"
    );
}

#[test]
fn test_gate_history_refreshes_from_disk_when_unlocked() {
    // item 2：内存门控陈旧（InProgress）但落盘已完成时，`gate_history` 用非阻塞
    // try_lock 核对落盘状态并放行，避免误报 MIGRATION_PENDING。
    let dir = temp_app_dir();
    write_v1(dir.path(), &[v1("c1", "anime", "T", map!())]);
    let db = HistoryDb::open(dir.path()).unwrap();
    let migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    migrator.run().unwrap(); // 磁盘已完成（NotNeeded）

    let state = super::commands::AppState {
        migrator: Some(migrator.clone()),
        history: Some(db),
        // 陈旧的内存门控：后台迁移任务可能刚结束但尚未写回 status_arc。
        migration_status: Arc::new(RwLock::new(MigrationStatus::InProgress)),
    };
    let status = super::commands::gate_history(&state).unwrap();
    assert_eq!(status, MigrationStatus::NotNeeded);
    assert_eq!(
        *state.migration_status.read().unwrap(),
        MigrationStatus::NotNeeded,
        "gate must refresh the in-memory gate after a non-blocking disk check"
    );
}

#[test]
fn test_rollback_preserves_backup_path() {
    // item 5：失败回滚后 `migration_state.backup_path` 必须保留（引用真实存在的
    // 备份文件），备份不因失败而"孤儿化"。
    let dir = temp_app_dir();
    let entries: Vec<Value> = (0..600)
        .map(|i| v1(&format!("c{i}"), "anime", &format!("T{i}"), map!()))
        .collect();
    write_v1(dir.path(), &entries);

    let db = HistoryDb::open(dir.path()).unwrap();
    let mut migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    migrator.set_batch_hook(Some(Arc::new(|batch| {
        if batch == 1 {
            Err(MigrationError::Migration("injected".into()))
        } else {
            Ok(())
        }
    })));
    assert!(migrator.run().is_err());

    let conn = db.conn();
    let guard = conn.lock().unwrap();
    let status: String = guard
        .query_row("SELECT status FROM migration_state WHERE id=1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(status, "rolled_back");
    let backup_path: Option<String> = guard
        .query_row(
            "SELECT backup_path FROM migration_state WHERE id=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(guard);
    assert!(
        backup_path.is_some(),
        "rolled_back state must keep backup_path, got {backup_path:?}"
    );
    assert!(
        Path::new(backup_path.as_deref().unwrap()).exists(),
        "preserved backup_path must reference an existing file"
    );
}

#[test]
fn test_check_after_rollback_pending_and_retry_reuses_backup() {
    // item 5：迁移失败（rolled_back）后 check() 返回 Pending（从头重试），
    // 且重试 run() 复用已保留的备份路径，不生成第二份孤儿备份。
    let dir = temp_app_dir();
    let entries: Vec<Value> = (0..600)
        .map(|i| v1(&format!("c{i}"), "anime", &format!("T{i}"), map!()))
        .collect();
    write_v1(dir.path(), &entries);

    let db = HistoryDb::open(dir.path()).unwrap();
    let mut migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    migrator.set_batch_hook(Some(Arc::new(|batch| {
        if batch == 1 {
            Err(MigrationError::Migration("injected".into()))
        } else {
            Ok(())
        }
    })));
    assert!(migrator.run().is_err());
    // 边界行为核对：rolled_back → Pending（spec §3.3.c）。
    assert_eq!(
        migrator.check().unwrap(),
        MigrationStatus::Pending,
        "check() must return Pending after rollback"
    );
    assert_eq!(migrator.list_backups().unwrap().len(), 1);

    // 重试（无注入）→ 成功；复用既有备份，备份数仍为 1。
    let retry = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    let report = retry.run().unwrap();
    assert_eq!(report.status, MigrationStatus::Completed);
    assert_eq!(history_rows(&db).len(), 600);
    assert_eq!(
        retry.list_backups().unwrap().len(),
        1,
        "retry after rollback must reuse the preserved backup, not create a second one"
    );
}

// ---------------------------------------------------------------------------
// 迁移状态边界回归测试
// ---------------------------------------------------------------------------

#[test]
fn test_source_missing_after_interruption_unblocks_gate() {
    // item 2：迁移中断（migration_state='in_progress'）后 v1 文件被外部删除。
    // run() 必须把陈旧状态收敛为 NotNeeded，避免 check() 恒返回 InProgress、
    // 门控永久卡死且 history_* 命令永远 MIGRATION_PENDING（无恢复入口）。
    let dir = temp_app_dir();
    let entries: Vec<Value> = (0..600)
        .map(|i| v1(&format!("c{i}"), "anime", &format!("T{i}"), map!()))
        .collect();
    write_v1(dir.path(), &entries);

    let db = HistoryDb::open(dir.path()).unwrap();
    let mut migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    migrator.set_batch_hook(Some(Arc::new(|batch| {
        if batch == 1 {
            panic!("injected crash after batch 1");
        }
        Ok(())
    })));
    let handle = std::thread::spawn(move || migrator.run());
    assert!(handle.join().is_err(), "run should have panicked");

    // 落盘状态停在 in_progress（断点续迁待命）。
    let migrator2 = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    assert_eq!(
        migrator2.check().unwrap(),
        MigrationStatus::InProgress,
        "interrupted migration must report InProgress before source removal"
    );

    // 外部删除 v1 源文件（模拟用户/同步工具移动或删除）。
    std::fs::remove_file(dir.path().join("history.json")).unwrap();

    // run() 收敛陈旧状态：不再卡 InProgress，返回 NotNeeded。
    let report = migrator2.run().unwrap();
    assert_eq!(report.status, MigrationStatus::NotNeeded);
    assert_eq!(
        migrator2.check().unwrap(),
        MigrationStatus::NotNeeded,
        "check() must not stay InProgress after source vanishes"
    );

    // 崩溃前已提交的批次数据保留（不因放弃迁移而丢失）；staging 清空。
    assert_eq!(
        history_rows(&db).len(),
        500,
        "committed batch must be retained"
    );
    let conn = db.conn();
    let guard = conn.lock().unwrap();
    let status: String = guard
        .query_row("SELECT status FROM migration_state WHERE id=1", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(status, "completed");
    let staging: i64 = guard
        .query_row("SELECT COUNT(*) FROM migration_staging", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(staging, 0, "staging must be cleared when abandoning");
    let backup_path: Option<String> = guard
        .query_row(
            "SELECT backup_path FROM migration_state WHERE id=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(guard);
    assert!(
        backup_path.is_some(),
        "backup must be preserved as a recovery entry, got {backup_path:?}"
    );
    assert!(Path::new(backup_path.as_deref().unwrap()).exists());

    // 门控放行：check() 结果可直接喂给 history 命令的门控。
    let status = migrator2.check().unwrap();
    assert!(
        ensure_history_available(&status).is_ok(),
        "gate must be unblocked after abandoning a missing-source migration"
    );
}

#[test]
fn test_source_missing_after_rollback_unblocks_gate() {
    // item 2 同类边界：迁移失败（rolled_back）后 v1 文件被外部删除。
    // run() 同样要把状态收敛为 NotNeeded，避免 check() 恒返回 Pending 卡死门控。
    let dir = temp_app_dir();
    let entries: Vec<Value> = (0..600)
        .map(|i| v1(&format!("c{i}"), "anime", &format!("T{i}"), map!()))
        .collect();
    write_v1(dir.path(), &entries);

    let db = HistoryDb::open(dir.path()).unwrap();
    let mut migrator = Migrator::new(db.clone(), dir.path().to_path_buf()).unwrap();
    migrator.set_batch_hook(Some(Arc::new(|batch| {
        if batch == 1 {
            Err(MigrationError::Migration("injected".into()))
        } else {
            Ok(())
        }
    })));
    assert!(migrator.run().is_err());
    assert_eq!(
        migrator.check().unwrap(),
        MigrationStatus::Pending,
        "rolled_back must report Pending before source removal"
    );

    std::fs::remove_file(dir.path().join("history.json")).unwrap();

    let report = migrator.run().unwrap();
    assert_eq!(report.status, MigrationStatus::NotNeeded);
    assert_eq!(
        migrator.check().unwrap(),
        MigrationStatus::NotNeeded,
        "check() must not stay Pending after source vanishes"
    );
    assert!(
        ensure_history_available(&migrator.check().unwrap()).is_ok(),
        "gate must be unblocked"
    );
}

// ---------------------------------------------------------------------------
// 性能路径（`cargo test -- --ignored` 单独跑）
// ---------------------------------------------------------------------------

#[test]
#[ignore = "performance gate: run with --ignored"]
fn test_migrate_10k_records_perf() {
    use std::time::{Duration, Instant};

    let dir = temp_app_dir();
    let entries: Vec<Value> = (0..10_000)
        .map(|i| {
            let ct = match i % 3 {
                0 => "anime",
                1 => "manga",
                _ => "novel",
            };
            v1(
                &format!("c{i}"),
                ct,
                &format!("Title {i}"),
                map!("updated_at" => 1_600_000_000_i64 + i),
            )
        })
        .collect();
    write_v1(dir.path(), &entries);

    let (db, migrator) = open_migrator(dir.path());
    let started = Instant::now();
    let report = migrator.run().unwrap();
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(10),
        "migration took {elapsed:?}"
    );
    assert_eq!(report.total, 10_000);
    assert_eq!(
        <HistoryDb as HistoryRepo>::count(&db, None).unwrap(),
        10_000
    );

    let t0 = Instant::now();
    let page = <HistoryDb as HistoryRepo>::list(&db, None, None, 50, 0).unwrap();
    assert_eq!(page.len(), 50);
    assert!(
        t0.elapsed() < Duration::from_millis(50),
        "list took {:?}",
        t0.elapsed()
    );
}
