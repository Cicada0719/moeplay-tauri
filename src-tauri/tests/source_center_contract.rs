use moeplay_lib::{
    db_sqlite::{SourcePreferenceRepository, SourcePreferenceUpsert, SqliteDb},
    domain::{ProviderErrorKind, ProviderHealth, ProviderHealthState},
    source_center::health_summary_for,
    source_selection::{
        select_source, CircuitBreakerConfig, SourceCandidate, SourceRejection,
        SourceSelectionPolicy, SourceSelectionRequest, SourceSelectionResult,
    },
};

#[test]
fn schema_v7_persists_media_scoped_source_preferences_without_sensitive_fields() {
    let database = SqliteDb::open_in_memory().expect("initialize schema v7");
    let repository = SourcePreferenceRepository::new(&database);

    let saved = repository
        .upsert(SourcePreferenceUpsert {
            provider_id: "catalog-a".to_owned(),
            media_type: "comic".to_owned(),
            enabled: true,
            priority: 42,
        })
        .expect("save source preference");

    assert_eq!(saved.provider_id, "catalog-a");
    assert_eq!(saved.media_type, "comic");
    assert!(saved.enabled);
    assert_eq!(saved.priority, 42);
    assert!(!saved.updated_at.is_empty());
    assert_eq!(
        repository
            .get("catalog-a", "comic")
            .expect("load preference"),
        Some(saved)
    );
    assert!(repository
        .upsert(SourcePreferenceUpsert {
            provider_id: "Authorization: Bearer secret".to_owned(),
            media_type: "comic".to_owned(),
            enabled: true,
            priority: 0,
        })
        .is_err());
}

#[test]
fn automatic_selection_respects_health_and_manual_selection_never_bypasses_policy() {
    let mut unavailable = SourceCandidate::new("unavailable", false, 100);
    let mut failing = SourceCandidate::new("failing", true, 90);
    let healthy = SourceCandidate::new("healthy", true, 0);
    let breaker = CircuitBreakerConfig {
        failure_threshold: 1,
        open_for_ms: 5_000,
        ..Default::default()
    };
    unavailable.health.record_success(Some(10.0), 100, &breaker);
    failing.health.record_failure(Some(20.0), 100, &breaker);

    let automatic = select_source(
        &[unavailable, failing, healthy.clone()],
        &SourceSelectionRequest::Automatic,
        &SourceSelectionPolicy::default(),
        101,
    );
    assert!(matches!(
        automatic,
        SourceSelectionResult::Selected { provider_id, manual: false, .. } if provider_id == "healthy"
    ));

    let mut policy = SourceSelectionPolicy::default();
    policy.blocked_provider_ids.insert("healthy".to_owned());
    assert_eq!(
        select_source(
            &[healthy],
            &SourceSelectionRequest::Manual {
                provider_id: "healthy".to_owned()
            },
            &policy,
            101,
        ),
        SourceSelectionResult::Rejected(SourceRejection::PolicyBlocked)
    );
}

#[test]
fn source_health_projection_exposes_only_a_redacted_latest_failure() {
    let summary = health_summary_for(
        "catalog-a",
        &[ProviderHealth {
            provider_id: "catalog-a".to_owned(),
            operation: "probe".to_owned(),
            state: ProviderHealthState::Degraded,
            success_count: 1,
            failure_count: 1,
            consecutive_failures: 1,
            latency_ms_ema: Some(25.0),
            last_success_at: None,
            last_failure_at: Some("2026-07-11T00:00:00Z".to_owned()),
            circuit_open_until: None,
            last_error_kind: Some(ProviderErrorKind::Timeout),
        }],
    );

    let failure = summary.last_failure.expect("latest failure projection");
    assert_eq!(failure.code, ProviderErrorKind::Timeout);
    assert_eq!(failure.occurred_at, "2026-07-11T00:00:00Z");
    assert_eq!(failure.message, "source operation failed");
}
