#![allow(dead_code, unused_imports)]

use moeplay_lib::domain as domain_types;

// Compile the provider module in isolation as well as through the library so this
// contract suite catches accidental dependencies on Tauri integration state.
mod domain {
    pub use super::domain_types::*;
}

#[path = "../src/providers/comic/mod.rs"]
mod comic;

use comic::{
    AuthConfig, ComicProviderError, ComicSourceAdapter, HealthTracker, KavitaConnector,
    KomgaConnector, LocalComicAdapter, ResolveRequest, SearchRequest,
};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use url::Url;

fn fixture(path: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/comic")
        .join(path);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn komga_fixture_contract_maps_series_chapters_and_pages() {
    let libraries = KomgaConnector::parse_libraries(&fixture("komga/libraries.json"));
    assert_eq!(libraries[0].id, "library-1");
    let series = KomgaConnector::parse_series(&fixture("komga/series.json"));
    assert_eq!(series[0].title, "Fixture Series");
    let chapters = KomgaConnector::parse_chapters("series-1", &fixture("komga/books.json"));
    assert_eq!(chapters[0].identity.stable_key, "komga:series-1:book-1");
    assert_eq!(chapters[0].sort.chapter_number, Some(1.0));
    assert_eq!(chapters[0].language.as_deref(), Some("zh-CN"));

    let http = comic::ComicHttpClient::new(comic::ComicHttpConfig {
        base_url: "http://127.0.0.1:25678".into(),
        auth: AuthConfig::Bearer("secret".into()),
    })
    .unwrap();
    let pages =
        KomgaConnector::parse_page_urls(&http, "book-1", &fixture("komga/pages.json")).unwrap();
    assert_eq!(
        pages[0],
        "http://127.0.0.1:25678/api/v1/books/book-1/pages/0"
    );
}

#[test]
fn kavita_fixture_contract_maps_library_series_volume_chapter_and_page_count() {
    let libraries = KavitaConnector::parse_libraries(&fixture("kavita/libraries.json"));
    assert_eq!(libraries[0].id, "7");
    let series = KavitaConnector::parse_series(&fixture("kavita/series.json"));
    assert_eq!(series[0].id, "42");
    let chapters = KavitaConnector::parse_chapters("42", &fixture("kavita/volumes.json"));
    assert_eq!(chapters[0].identity.stable_key, "kavita:42:99");
    assert_eq!(chapters[0].sort.volume_number, Some(1.0));
    assert_eq!(
        KavitaConnector::parse_page_count(&fixture("kavita/dimensions.json")),
        Some(4)
    );
}

#[test]
fn base_url_and_auth_contract_blocks_ssrf_and_cross_origin_credentials() {
    assert!(comic::validate_base_url("http://127.0.0.1:1234").is_ok());
    assert!(comic::validate_base_url("http://example.com").is_err());
    assert!(comic::validate_base_url("https://127.0.0.1:1234").is_err());
    assert!(comic::validate_base_url("https://example.com").is_ok());
    let client = comic::ComicHttpClient::new(comic::ComicHttpConfig {
        base_url: "https://example.com".into(),
        auth: AuthConfig::ApiKey("secret".into()),
    })
    .unwrap();
    let same = Url::parse("https://example.com/api/Reader/image").unwrap();
    let other = Url::parse("https://cdn.example.com/page.jpg").unwrap();
    assert_eq!(client.auth_headers_for(&same).len(), 1);
    assert!(client.auth_headers_for(&other).is_empty());
}

#[test]
fn health_circuit_opens_after_three_consecutive_failures() {
    let health = HealthTracker::new("fixture");
    let failure = ComicProviderError::InvalidConfig("bad fixture".into());
    for _ in 0..3 {
        health.failure("resolve", &failure);
    }
    assert!(health.before("resolve").is_err());
    assert_eq!(
        health.snapshot("resolve").state,
        domain_types::ProviderHealthState::OpenCircuit
    );
}

#[tokio::test]
async fn local_manifest_and_safe_resolve_never_extracts_cbz() {
    let root = std::env::temp_dir().join(format!("moeplay-comic-contract-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("pages")).unwrap();
    fs::write(root.join("pages/002.jpg"), b"jpg").unwrap();
    fs::write(root.join("pages/001.jpg"), b"jpg").unwrap();
    fs::write(root.join("chapter.cbz"), b"not-extracted").unwrap();
    fs::write(root.join(".moeplay-comic.json"), r#"{"version":1,"libraryName":"Fixture Local","series":[{"id":"s","title":"Local","path":".","language":"ja","chapters":[{"id":"dir","title":"Pages","path":"pages"},{"id":"cbz","title":"Archive","path":"chapter.cbz"}]}]}"#).unwrap();
    let adapter = LocalComicAdapter::new(&root).unwrap();
    let pages = adapter
        .resolve(ResolveRequest {
            series_id: "s".into(),
            chapter_id: "dir".into(),
        })
        .await
        .unwrap();
    match pages {
        domain_types::ResolvedTarget::ImagePages { pages, headers } => {
            assert_eq!(pages.len(), 2);
            assert!(headers.is_empty());
            assert!(pages[0].contains("001.jpg"));
        }
        other => panic!("unexpected target: {other:?}"),
    }
    let cbz = adapter
        .resolve(ResolveRequest {
            series_id: "s".into(),
            chapter_id: "cbz".into(),
        })
        .await
        .unwrap();
    assert!(matches!(
        cbz,
        domain_types::ResolvedTarget::Unsupported { .. }
    ));
    assert!(adapter.safe_join("../chapter.cbz").is_err());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn unified_request_wire_shape_is_stable() {
    let request = SearchRequest {
        query: "猫".into(),
        library_id: Some("7".into()),
        page: 2,
        page_size: 20,
    };
    let value = serde_json::to_value(request).unwrap();
    assert_eq!(value["libraryId"], "7");
    assert_eq!(value["pageSize"], 20);
}
