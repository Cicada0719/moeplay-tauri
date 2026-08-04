//! 历史记录 WebDAV 同步集成测试（spec §6.3）。
//!
//! 用 wiremock 模拟 WebDAV 服务端、tempfile 磁盘 SQLite（HistoryDb）驱动
//! `run_sync_with`（注入 allow_http 客户端，绕过 https_only 与 keyring 依赖）。

use moeplay_lib::db_sqlite::{HistoryDb, HistoryRepo};
use moeplay_lib::domain::history::{ContentType, HistoryRecord};
use moeplay_lib::sync::webdav::WebDavClient;
use moeplay_lib::sync::{run_sync_with, SyncError, SyncMode, SyncRecord, SyncState, WebDavConfig};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn history_record(
    id: &str,
    content_id: &str,
    title: &str,
    source_id: &str,
    updated_at_ms: i64,
    page_index: i64,
) -> HistoryRecord {
    HistoryRecord {
        id: id.to_string(),
        content_id: content_id.to_string(),
        content_type: ContentType::Manga,
        title: title.to_string(),
        cover: None,
        source_id: source_id.to_string(),
        chapter_id: Some("ch1".to_string()),
        chapter_title: Some("第 1 话".to_string()),
        page_index,
        position_sec: 0.0,
        scroll_pct: 0.0,
        updated_at: updated_at_ms,
        device_id: "device-a".to_string(),
        deleted: false,
    }
}

fn sync_record(
    id: &str,
    content_id: &str,
    title: &str,
    source_id: &str,
    updated_at_s: i64,
    page_index: i64,
    deleted: bool,
) -> SyncRecord {
    SyncRecord {
        id: id.to_string(),
        content_id: content_id.to_string(),
        content_type: "manga".to_string(),
        title: title.to_string(),
        cover: None,
        source_id: source_id.to_string(),
        chapter_id: Some("ch1".to_string()),
        chapter_title: Some("第 1 话".to_string()),
        page_index,
        position_sec: 0.0,
        scroll_pct: 0.0,
        updated_at: updated_at_s,
        device_id: "device-remote".to_string(),
        deleted,
    }
}

fn config_for(server: &MockServer) -> WebDavConfig {
    WebDavConfig {
        base_url: server.uri(),
        username: "alice".to_string(),
        mode: SyncMode::Manual,
        remote_dir: "moeplay-sync".to_string(),
    }
}

fn state_for(db: HistoryDb) -> SyncState {
    SyncState::new(Some(db))
}

async fn mount_ok_webdav(server: &MockServer, remote_body: Option<Vec<u8>>) {
    Mock::given(method("MKCOL"))
        .respond_with(ResponseTemplate::new(201))
        .mount(server)
        .await;
    match remote_body {
        Some(body) => {
            Mock::given(method("GET"))
                .and(path("/moeplay-sync/history.json"))
                .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
                .mount(server)
                .await;
        }
        None => {
            Mock::given(method("GET"))
                .and(path("/moeplay-sync/history.json"))
                .respond_with(ResponseTemplate::new(404))
                .mount(server)
                .await;
        }
    }
    Mock::given(method("PUT"))
        .and(path("/moeplay-sync/history.json"))
        .respond_with(ResponseTemplate::new(204))
        .mount(server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/moeplay-sync/manifest.json"))
        .respond_with(ResponseTemplate::new(204))
        .mount(server)
        .await;
}

async fn put_history_body(server: &MockServer) -> Vec<SyncRecord> {
    let requests = server.received_requests().await.unwrap_or_default();
    for request in requests {
        if request.method.as_str() == "PUT" && request.url.path() == "/moeplay-sync/history.json" {
            return serde_json::from_slice(&request.body).expect("parse history PUT body");
        }
    }
    panic!("no PUT to history.json was recorded");
}

#[tokio::test]
async fn end_to_end_upload_merge_download() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = HistoryDb::open(dir.path()).expect("open history db");
    for (index, cid) in ["c1", "c2", "c3"].iter().enumerate() {
        let id = format!("l{}", index + 1);
        db.upsert(&history_record(
            &id,
            cid,
            cid,
            "s1",
            1000 * (index as i64 + 1),
            index as i64 + 1,
        ))
        .expect("upsert local");
    }
    let remote = vec![
        sync_record("r1", "d1", "d1", "s1", 100, 1, false),
        sync_record("r2", "d2", "d2", "s1", 100, 2, false),
    ];
    let remote_body = serde_json::to_vec(&remote).expect("serialize remote");

    let server = MockServer::start().await;
    mount_ok_webdav(&server, Some(remote_body)).await;

    let state = state_for(db.clone());
    let cfg = config_for(&server);
    let client = WebDavClient::new_allow_http(&server.uri(), "alice", "pw").expect("client");
    let result = run_sync_with(&state, &cfg, client)
        .await
        .expect("sync succeeds");

    assert_eq!(result.uploaded, 3);
    assert_eq!(result.downloaded, 2);
    assert_eq!(result.conflicts, 0);
    assert_eq!(result.tombstones_purged, 0);

    // 本地库新增远端 2 条 → 共 5 条。
    let local = db.list_all_with_deleted().expect("list local");
    assert_eq!(local.len(), 5);

    // 远端文件（PUT body）含 5 条。
    let uploaded = put_history_body(&server).await;
    assert_eq!(uploaded.len(), 5);
}

#[tokio::test]
async fn two_devices_converge_to_newest_progress() {
    // 设备 A：看到第 8 页（较新 updated_at）。
    let dir_a = tempfile::tempdir().expect("tempdir a");
    let db_a = HistoryDb::open(dir_a.path()).expect("open a");
    db_a.upsert(&history_record(
        "a8", "anime1", "My Anime", "s1", 2_000_000, 8,
    ))
    .expect("upsert a");
    let server_a = MockServer::start().await;
    mount_ok_webdav(&server_a, None).await; // 远端为空
    let state_a = state_for(db_a.clone());
    let cfg_a = config_for(&server_a);
    let client_a = WebDavClient::new_allow_http(&server_a.uri(), "alice", "pw").expect("client a");
    let result_a = run_sync_with(&state_a, &cfg_a, client_a)
        .await
        .expect("a sync ok");
    assert_eq!(result_a.uploaded, 1);
    let remote_after_a = put_history_body(&server_a).await;
    assert_eq!(remote_after_a.len(), 1);
    assert_eq!(remote_after_a[0].page_index, 8);

    // 设备 B：本地还是第 5 页（较旧）；远端已是设备 A 上传的第 8 页。
    let dir_b = tempfile::tempdir().expect("tempdir b");
    let db_b = HistoryDb::open(dir_b.path()).expect("open b");
    db_b.upsert(&history_record(
        "b5", "anime1", "My Anime", "s1", 1_000_000, 5,
    ))
    .expect("upsert b");
    let server_b = MockServer::start().await;
    let remote_body = serde_json::to_vec(&remote_after_a).expect("serialize remote");
    mount_ok_webdav(&server_b, Some(remote_body)).await;
    let state_b = state_for(db_b.clone());
    let cfg_b = config_for(&server_b);
    let client_b = WebDavClient::new_allow_http(&server_b.uri(), "alice", "pw").expect("client b");
    let result_b = run_sync_with(&state_b, &cfg_b, client_b)
        .await
        .expect("b sync ok");
    assert_eq!(result_b.downloaded, 1);
    assert_eq!(result_b.conflicts, 1);

    // B 本地收敛为第 8 页（同键 updated_at 更新者胜）。
    let b_local = db_b.list_all_with_deleted().expect("list b");
    assert_eq!(b_local.len(), 1);
    assert_eq!(b_local[0].page_index, 8);
}

#[tokio::test]
async fn network_drop_is_idempotent_and_preserves_local() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = HistoryDb::open(dir.path()).expect("open history db");
    for (index, cid) in ["c1", "c2", "c3"].iter().enumerate() {
        let id = format!("l{}", index + 1);
        db.upsert(&history_record(
            &id,
            cid,
            cid,
            "s1",
            1000 * (index as i64 + 1),
            index as i64 + 1,
        ))
        .expect("upsert local");
    }
    let remote = vec![
        sync_record("r1", "d1", "d1", "s1", 100, 1, false),
        sync_record("r2", "d2", "d2", "s1", 100, 2, false),
    ];
    let remote_body = serde_json::to_vec(&remote).expect("serialize remote");

    let server = MockServer::start().await;
    Mock::given(method("MKCOL"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/moeplay-sync/history.json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(remote_body, "application/json"))
        .mount(&server)
        .await;
    // 首次 PUT history.json 断网（500）；后续恢复（204）。
    // with_priority(1)（默认 5）保证 500 mock 优先匹配；up_to_n_times(1) 消费后放行 204。
    Mock::given(method("PUT"))
        .and(path("/moeplay-sync/history.json"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/moeplay-sync/history.json"))
        .respond_with(ResponseTemplate::new(500))
        .with_priority(1)
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/moeplay-sync/manifest.json"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let state = state_for(db.clone());
    let cfg = config_for(&server);
    let client = WebDavClient::new_allow_http(&server.uri(), "alice", "pw").expect("client");
    let first = run_sync_with(&state, &cfg, client).await;
    assert!(
        matches!(first, Err(SyncError::Server(_))),
        "first PUT 500 should fail the sync"
    );
    assert_eq!(
        db.list_all_with_deleted().expect("list").len(),
        3,
        "local data must not be cleared on failure"
    );

    // 恢复后重跑 → 最终一致、无重复记录。
    let client2 = WebDavClient::new_allow_http(&server.uri(), "alice", "pw").expect("client2");
    let second = run_sync_with(&state, &cfg, client2)
        .await
        .expect("second sync ok");
    assert_eq!(second.uploaded, 3);
    assert_eq!(second.downloaded, 2);

    let local = db.list_all_with_deleted().expect("list");
    assert_eq!(local.len(), 5);
    let mut ids: Vec<&str> = local.iter().map(|r| r.id.as_str()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 5, "no duplicate records after recovery");
}

#[tokio::test]
async fn auth_failure_preserves_local() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = HistoryDb::open(dir.path()).expect("open history db");
    db.upsert(&history_record("l1", "c1", "c1", "s1", 1000, 1))
        .expect("upsert local");

    let server = MockServer::start().await;
    Mock::given(method("MKCOL"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let state = state_for(db.clone());
    let cfg = config_for(&server);
    let client = WebDavClient::new_allow_http(&server.uri(), "alice", "pw").expect("client");
    let result = run_sync_with(&state, &cfg, client).await;
    assert!(matches!(result, Err(SyncError::Auth)));

    let local = db.list_all_with_deleted().expect("list");
    assert_eq!(local.len(), 1, "auth failure must not touch local data");
    assert_eq!(local[0].page_index, 1);
}

#[tokio::test]
async fn remote_corrupt_json_returns_parse_and_zero_writes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = HistoryDb::open(dir.path()).expect("open history db");
    db.upsert(&history_record("l1", "c1", "c1", "s1", 1000, 1))
        .expect("upsert local");

    let server = MockServer::start().await;
    Mock::given(method("MKCOL"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/moeplay-sync/history.json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(b"not-json{{{", "application/json"))
        .mount(&server)
        .await;

    let state = state_for(db.clone());
    let cfg = config_for(&server);
    let client = WebDavClient::new_allow_http(&server.uri(), "alice", "pw").expect("client");
    let result = run_sync_with(&state, &cfg, client).await;
    assert!(matches!(result, Err(SyncError::Parse)));

    assert_eq!(
        db.list_all_with_deleted().expect("list").len(),
        1,
        "corrupt remote must cause zero local writes"
    );
}

#[tokio::test]
async fn concurrent_sync_second_is_busy() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = HistoryDb::open(dir.path()).expect("open history db");
    db.upsert(&history_record("l1", "c1", "c1", "s1", 1000, 1))
        .expect("upsert local");

    let server = MockServer::start().await;
    Mock::given(method("MKCOL"))
        .respond_with(ResponseTemplate::new(201).set_delay(std::time::Duration::from_millis(400)))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/moeplay-sync/history.json"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let state = std::sync::Arc::new(state_for(db.clone()));
    let cfg = config_for(&server);

    let state1 = std::sync::Arc::clone(&state);
    let cfg1 = cfg.clone();
    let client1 = WebDavClient::new_allow_http(&server.uri(), "alice", "pw").expect("client1");
    let first = tokio::spawn(async move { run_sync_with(&state1, &cfg1, client1).await });
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;

    let client2 = WebDavClient::new_allow_http(&server.uri(), "alice", "pw").expect("client2");
    let second = run_sync_with(&state, &cfg, client2).await;
    assert!(
        matches!(second, Err(SyncError::Busy)),
        "second concurrent sync must be Busy"
    );

    let first_result = first.await.expect("join").expect("first sync ok");
    assert_eq!(first_result.uploaded, 1);
}
