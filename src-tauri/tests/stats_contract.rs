use chrono::{TimeZone, Utc};
use moeplay_lib::models::{Game, PlaySession};
use moeplay_lib::stats::{build_monthly_heatmap_at, DashboardData};

const DASHBOARD_FIXTURE: &str = include_str!("fixtures/stats_dashboard.json");

#[test]
fn dashboard_json_fixture_matches_the_rust_dto() {
    let expected: serde_json::Value = serde_json::from_str(DASHBOARD_FIXTURE).unwrap();
    let dashboard: DashboardData = serde_json::from_value(expected.clone()).unwrap();
    let serialized = serde_json::to_value(dashboard).unwrap();

    assert_eq!(serialized, expected);
    assert!(serialized.get("playtime_hours").is_some());
    assert!(serialized.get("total_playtime_hours").is_none());
    assert!(serialized.get("completion_distribution").is_some());
    assert!(serialized.get("status_distribution").is_none());
}

#[test]
fn monthly_aggregation_accepts_legacy_and_rfc3339_without_double_counting() {
    let mut legacy = Game::new("Legacy".to_string(), "legacy.exe".to_string());
    legacy.id = "legacy-game".to_string();
    legacy.play_tracker.last_played = Some("2026-07-10 12:45".to_string());
    let duplicate = PlaySession {
        id: "session-legacy".to_string(),
        start_time: "2026-07-10 11:45".to_string(),
        end_time: Some("2026-07-10 12:45".to_string()),
        duration_seconds: 3_600,
        notes: None,
    };
    legacy.play_tracker.sessions = vec![duplicate.clone(), duplicate];

    let mut rfc3339 = Game::new("RFC3339".to_string(), "rfc3339.exe".to_string());
    rfc3339.id = "rfc3339-game".to_string();
    rfc3339.play_tracker.sessions = vec![PlaySession {
        id: "session-rfc3339".to_string(),
        start_time: "2026-06-30T23:00:00.000-02:00".to_string(),
        end_time: Some("2026-06-30T23:30:00.000-02:00".to_string()),
        duration_seconds: 1_800,
        notes: None,
    }];

    let mut last_played_only = Game::new("Fallback".to_string(), "fallback.exe".to_string());
    last_played_only.id = "fallback-game".to_string();
    last_played_only.last_played = Some("2026-07-05 09:30".to_string());

    let now = Utc.with_ymd_and_hms(2026, 7, 15, 0, 0, 0).unwrap();
    let heatmap = build_monthly_heatmap_at(&[legacy, rfc3339, last_played_only], now);
    let july = heatmap
        .iter()
        .find(|activity| activity.month == "2026-07")
        .unwrap();

    assert_eq!(heatmap.len(), 12);
    assert_eq!(july.sessions, 3);
    assert_eq!(july.hours, 1.5);
}
