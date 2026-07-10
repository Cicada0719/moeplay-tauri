use moeplay_lib::db::Database;
use moeplay_lib::models::{AppDatabase, Game};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

struct TestDir(PathBuf);

impl TestDir {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "moeplay_db_recovery_{label}_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn recovery_backups(data_dir: &Path) -> Vec<PathBuf> {
    let mut backups: Vec<_> = std::fs::read_dir(data_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("moegame.db.recovery-") && name.ends_with(".bak")
                })
        })
        .collect();
    backups.sort();
    backups
}

fn open_error(data_dir: &Path) -> String {
    match Database::open_at(data_dir) {
        Ok(_) => panic!("corrupt database must fail closed"),
        Err(error) => error,
    }
}

#[test]
fn failed_open_preserves_primary_and_creates_non_overwriting_backup_set() {
    let data_dir = TestDir::new("backup_set");
    let db_path = data_dir.path().join("moegame.db");
    let wal_path = append_suffix(&db_path, "-wal");
    let shm_path = append_suffix(&db_path, "-shm");
    let primary_bytes = b"not-a-sqlite-database: primary sentinel";
    let wal_bytes = b"wal sentinel";
    let shm_bytes = b"shm sentinel";

    std::fs::write(&db_path, primary_bytes).unwrap();
    std::fs::write(&wal_path, wal_bytes).unwrap();
    std::fs::write(&shm_path, shm_bytes).unwrap();

    let first_error = open_error(data_dir.path());
    assert!(first_error.contains("No writable in-memory fallback was used"));
    assert!(db_path.exists());
    assert_eq!(std::fs::read(&db_path).unwrap(), primary_bytes);

    let first_backups = recovery_backups(data_dir.path());
    assert_eq!(first_backups.len(), 1);
    let first_backup = first_backups[0].clone();
    assert_eq!(std::fs::read(&first_backup).unwrap(), primary_bytes);
    assert_eq!(
        std::fs::read(append_suffix(&first_backup, "-wal")).unwrap(),
        wal_bytes
    );
    assert_eq!(
        std::fs::read(append_suffix(&first_backup, "-shm")).unwrap(),
        shm_bytes
    );

    let second_error = open_error(data_dir.path());
    assert!(second_error.contains("recovery backup:"));
    assert_eq!(std::fs::read(&db_path).unwrap(), primary_bytes);

    let second_backups = recovery_backups(data_dir.path());
    assert_eq!(second_backups.len(), 2);
    assert_ne!(second_backups[0], second_backups[1]);
    assert!(second_backups.contains(&first_backup));
    assert_eq!(std::fs::read(&first_backup).unwrap(), primary_bytes);
    assert!(!data_dir.path().join("moegame-fallback.db").exists());
}

#[test]
fn failed_migration_rolls_back_and_never_returns_an_empty_database() {
    let data_dir = TestDir::new("migration_rollback");
    let db_path = data_dir.path().join("moegame.db");

    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_version (version INTEGER NOT NULL);
             INSERT INTO schema_version(version) VALUES (0);
             CREATE TABLE games (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                data_json TEXT NOT NULL
             );
             INSERT INTO games(id,name,data_json)
             VALUES('sentinel','Sentinel Game','{}');
             CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
             );
             INSERT INTO settings(key,value) VALUES('app_settings','{}');
             CREATE TRIGGER force_migration_failure
             BEFORE UPDATE ON settings
             BEGIN
                SELECT RAISE(ABORT, 'forced migration failure');
             END;",
        )
        .unwrap();
    }

    let error = open_error(data_dir.path());
    assert!(error.contains("forced migration failure"));
    assert!(error.contains("No writable in-memory fallback was used"));
    assert!(db_path.exists());
    assert_eq!(recovery_backups(data_dir.path()).len(), 1);

    let conn = Connection::open(&db_path).unwrap();
    let sentinel: String = conn
        .query_row("SELECT name FROM games WHERE id='sentinel'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(sentinel, "Sentinel Game");

    let game_columns: Vec<String> = conn
        .prepare("PRAGMA table_info(games)")
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(!game_columns.iter().any(|column| column == "sort_name"));

    let settings_columns: Vec<String> = conn
        .prepare("PRAGMA table_info(settings)")
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(!settings_columns.iter().any(|column| column == "value_json"));
}

#[test]
fn legacy_json_auto_migration_still_imports_games() {
    let data_dir = TestDir::new("json_migration");
    let mut legacy = AppDatabase::default();
    let mut game = Game::new(
        "JSON Sentinel".to_string(),
        r"C:\Games\sentinel.exe".to_string(),
    );
    game.id = "json-sentinel".to_string();
    legacy.games.push(game);
    std::fs::write(
        data_dir.path().join("database.json"),
        serde_json::to_vec_pretty(&legacy).unwrap(),
    )
    .unwrap();

    let database = Database::open_at(data_dir.path()).unwrap();
    assert_eq!(database.sqlite().game_count().unwrap(), 1);
    assert_eq!(
        database.sqlite().get_game("json-sentinel").unwrap().name,
        "JSON Sentinel"
    );
}
