use moeplay_lib::domain::{
    BackgroundJob, BackgroundJobStatus, ProviderError, ProviderErrorKind, ProviderHealth,
    ProviderHealthState, ResolvedTarget, ResourceKind,
};

#[test]
fn provider_contract_wire_format_is_stable() {
    let error = ProviderError {
        kind: ProviderErrorKind::CaptchaRequired,
        message: "verification required".to_string(),
        retryable: false,
        retry_after_ms: None,
        provider_id: Some("fixture-provider".to_string()),
        operation: Some("search".to_string()),
    };
    let value = serde_json::to_value(&error).unwrap();
    assert_eq!(value["kind"], "captcha_required");
    assert_eq!(value["providerId"], "fixture-provider");
    assert!(value.get("provider_id").is_none());

    let target = ResolvedTarget::ImagePages {
        pages: vec!["https://images.example/1.jpg".to_string()],
        headers: vec![("referer".to_string(), "https://example/".to_string())],
    };
    let target_value = serde_json::to_value(target).unwrap();
    assert_eq!(target_value["mode"], "image_pages");
    assert!(target_value.get("allowedHosts").is_none());
}

#[test]
fn health_and_job_contracts_roundtrip() {
    let health = ProviderHealth {
        provider_id: "fixture-provider".to_string(),
        operation: "resolve".to_string(),
        state: ProviderHealthState::Degraded,
        success_count: 5,
        failure_count: 2,
        consecutive_failures: 1,
        latency_ms_ema: Some(420.5),
        last_success_at: Some("2026-07-10T00:00:00Z".to_string()),
        last_failure_at: None,
        circuit_open_until: None,
        last_error_kind: Some(ProviderErrorKind::Timeout),
    };
    let encoded = serde_json::to_string(&health).unwrap();
    let decoded: ProviderHealth = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, health);

    let job = BackgroundJob {
        id: "job-1".to_string(),
        kind: "provider_probe".to_string(),
        title: "探测来源".to_string(),
        status: BackgroundJobStatus::Running,
        progress: 0.5,
        created_at: "2026-07-10T00:00:00Z".to_string(),
        updated_at: "2026-07-10T00:00:01Z".to_string(),
        error: None,
        metadata: serde_json::json!({ "resourceKind": ResourceKind::Comic }),
    };
    let value = serde_json::to_value(job).unwrap();
    assert_eq!(value["status"], "running");
    assert_eq!(value["metadata"]["resourceKind"], "comic");
}
