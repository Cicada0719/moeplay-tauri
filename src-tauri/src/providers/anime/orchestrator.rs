use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::domain::{ProviderErrorKind, ProviderHealth, ProviderHealthState};
use chrono::Utc;

use super::{
    ensure_anime_playback_target, provider_error, AnimeDetail, AnimeEpisode, AnimeResolveRequest,
    AnimeResolveResponse, AnimeSearchQuery, AnimeSearchResponse, AnimeSourceAdapter,
    ProviderResult,
};

#[derive(Debug, Clone, Copy)]
pub struct CircuitBreakerConfig {
    pub consecutive_failure_threshold: u32,
    pub open_for: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            consecutive_failure_threshold: 3,
            open_for: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone)]
struct CircuitRecord {
    health: ProviderHealth,
    opened_until: Option<Instant>,
}

impl CircuitRecord {
    fn new(provider_id: &str, operation: &str) -> Self {
        Self {
            health: ProviderHealth {
                provider_id: provider_id.to_string(),
                operation: operation.to_string(),
                state: ProviderHealthState::Unknown,
                success_count: 0,
                failure_count: 0,
                consecutive_failures: 0,
                latency_ms_ema: None,
                last_success_at: None,
                last_failure_at: None,
                circuit_open_until: None,
                last_error_kind: None,
            },
            opened_until: None,
        }
    }

    fn effective_health(&self) -> ProviderHealth {
        let mut health = self.health.clone();
        if self
            .opened_until
            .is_some_and(|until| until > Instant::now())
        {
            health.state = ProviderHealthState::OpenCircuit;
        } else if health.state == ProviderHealthState::OpenCircuit {
            // The next request is a half-open probe. Preserve the evidence of
            // failures but do not keep the provider indefinitely unavailable.
            health.state = ProviderHealthState::Degraded;
            health.circuit_open_until = None;
        }
        health
    }
}

/// Owns provider selection and resilient provider health bookkeeping. It does
/// not persist health yet; command-layer integration can later feed snapshots
/// to the existing health repository.
pub struct AnimeProviderOrchestrator {
    adapters: Vec<Arc<dyn AnimeSourceAdapter>>,
    circuits: Mutex<HashMap<(String, String), CircuitRecord>>,
    config: CircuitBreakerConfig,
}

impl AnimeProviderOrchestrator {
    pub fn new(adapters: Vec<Arc<dyn AnimeSourceAdapter>>, config: CircuitBreakerConfig) -> Self {
        Self {
            adapters,
            circuits: Mutex::new(HashMap::new()),
            config,
        }
    }

    pub fn manifests(&self) -> Vec<crate::domain::ProviderManifest> {
        self.adapters
            .iter()
            .map(|adapter| adapter.manifest())
            .collect()
    }

    pub fn health(&self) -> Vec<ProviderHealth> {
        let circuits = self
            .circuits
            .lock()
            .expect("anime provider health mutex poisoned");
        circuits
            .values()
            .map(CircuitRecord::effective_health)
            .collect()
    }

    /// Clears every in-memory circuit record for one provider. Persistent health
    /// is cleared separately by Source Center, so a user-requested reset is not
    /// merely cosmetic.
    pub fn reset_provider_health(&self, provider_id: &str) -> bool {
        let mut circuits = self
            .circuits
            .lock()
            .expect("anime provider health mutex poisoned");
        let before = circuits.len();
        circuits.retain(|(record_provider_id, _), _| record_provider_id != provider_id);
        circuits.len() != before
    }

    pub async fn search(&self, query: AnimeSearchQuery) -> AnimeSearchResponse {
        let mut items = Vec::new();
        let mut failures = Vec::new();
        for adapter in &self.adapters {
            let provider_id = adapter.manifest().id;
            if let Err(error) = self.before_call(&provider_id, "search") {
                failures.push(error);
                continue;
            }
            let started = Instant::now();
            match adapter.search(query.clone()).await {
                Ok(mut provider_items) => {
                    self.record_success(&provider_id, "search", started.elapsed());
                    items.append(&mut provider_items);
                }
                Err(error) => {
                    self.record_failure(&provider_id, "search", started.elapsed(), &error);
                    failures.push(error);
                }
            }
        }
        AnimeSearchResponse {
            items,
            failures,
            provider_health: self.health(),
        }
    }

    pub async fn search_provider(
        &self,
        provider_id: &str,
        query: AnimeSearchQuery,
    ) -> AnimeSearchResponse {
        let adapter = match self.adapter(provider_id) {
            Ok(adapter) => adapter,
            Err(error) => {
                return AnimeSearchResponse {
                    items: Vec::new(),
                    failures: vec![error],
                    provider_health: self.health(),
                };
            }
        };
        if let Err(error) = self.before_call(provider_id, "search") {
            return AnimeSearchResponse {
                items: Vec::new(),
                failures: vec![error],
                provider_health: self.health(),
            };
        }
        let started = Instant::now();
        let (items, failures) = match adapter.search(query).await {
            Ok(items) => {
                self.record_success(provider_id, "search", started.elapsed());
                (items, Vec::new())
            }
            Err(error) => {
                self.record_failure(provider_id, "search", started.elapsed(), &error);
                (Vec::new(), vec![error])
            }
        };
        AnimeSearchResponse {
            items,
            failures,
            provider_health: self.health(),
        }
    }

    pub async fn detail(&self, provider_id: &str, item_id: &str) -> ProviderResult<AnimeDetail> {
        let adapter = self.adapter(provider_id)?;
        self.before_call(provider_id, "detail")?;
        let started = Instant::now();
        match adapter.detail(item_id).await {
            Ok(detail) => {
                self.record_success(provider_id, "detail", started.elapsed());
                Ok(detail)
            }
            Err(error) => {
                self.record_failure(provider_id, "detail", started.elapsed(), &error);
                Err(error)
            }
        }
    }

    pub async fn episodes(
        &self,
        provider_id: &str,
        series_id: &str,
    ) -> ProviderResult<Vec<AnimeEpisode>> {
        let adapter = self.adapter(provider_id)?;
        self.before_call(provider_id, "episodes")?;
        let started = Instant::now();
        match adapter.episodes(series_id).await {
            Ok(episodes) => {
                self.record_success(provider_id, "episodes", started.elapsed());
                Ok(episodes)
            }
            Err(error) => {
                self.record_failure(provider_id, "episodes", started.elapsed(), &error);
                Err(error)
            }
        }
    }

    pub async fn resolve(
        &self,
        request: AnimeResolveRequest,
    ) -> ProviderResult<AnimeResolveResponse> {
        let provider_id = request.episode.provider_id.clone();
        let adapter = self.adapter(&provider_id)?;
        self.before_call(&provider_id, "resolve")?;
        let started = Instant::now();
        match adapter.resolve(request).await {
            Ok(response) => match ensure_anime_playback_target(&provider_id, response.target) {
                Ok(target) => {
                    self.record_success(&provider_id, "resolve", started.elapsed());
                    Ok(AnimeResolveResponse { target, ..response })
                }
                Err(error) => {
                    self.record_failure(&provider_id, "resolve", started.elapsed(), &error);
                    Err(error)
                }
            },
            Err(error) => {
                self.record_failure(&provider_id, "resolve", started.elapsed(), &error);
                Err(error)
            }
        }
    }

    fn adapter(&self, provider_id: &str) -> ProviderResult<&Arc<dyn AnimeSourceAdapter>> {
        self.adapters
            .iter()
            .find(|adapter| adapter.manifest().id == provider_id)
            .ok_or_else(|| {
                provider_error(
                    provider_id,
                    "select_provider",
                    ProviderErrorKind::Unsupported,
                    "anime provider is not registered",
                    false,
                )
            })
    }

    fn before_call(&self, provider_id: &str, operation: &str) -> ProviderResult<()> {
        let mut circuits = self
            .circuits
            .lock()
            .expect("anime provider health mutex poisoned");
        let record = circuits
            .entry((provider_id.to_string(), operation.to_string()))
            .or_insert_with(|| CircuitRecord::new(provider_id, operation));
        if record
            .opened_until
            .is_some_and(|until| until > Instant::now())
        {
            let retry_after_ms = record
                .opened_until
                .and_then(|until| until.checked_duration_since(Instant::now()))
                .map(|duration| duration.as_millis() as u64);
            let mut error = provider_error(
                provider_id,
                operation,
                ProviderErrorKind::Network,
                "provider circuit is open; request was not dispatched",
                true,
            );
            error.retry_after_ms = retry_after_ms;
            return Err(error);
        }
        if record.opened_until.take().is_some() {
            record.health.state = ProviderHealthState::Degraded;
            record.health.circuit_open_until = None;
        }
        Ok(())
    }

    fn record_success(&self, provider_id: &str, operation: &str, elapsed: Duration) {
        let mut circuits = self
            .circuits
            .lock()
            .expect("anime provider health mutex poisoned");
        let record = circuits
            .entry((provider_id.to_string(), operation.to_string()))
            .or_insert_with(|| CircuitRecord::new(provider_id, operation));
        record.health.success_count += 1;
        record.health.consecutive_failures = 0;
        record.health.state = ProviderHealthState::Healthy;
        record.health.last_success_at = Some(Utc::now().to_rfc3339());
        record.health.latency_ms_ema = Some(update_latency(record.health.latency_ms_ema, elapsed));
        record.opened_until = None;
        record.health.circuit_open_until = None;
    }

    fn record_failure(
        &self,
        provider_id: &str,
        operation: &str,
        elapsed: Duration,
        error: &crate::domain::ProviderError,
    ) {
        if error.kind == ProviderErrorKind::Cancelled {
            return;
        }
        let mut circuits = self
            .circuits
            .lock()
            .expect("anime provider health mutex poisoned");
        let record = circuits
            .entry((provider_id.to_string(), operation.to_string()))
            .or_insert_with(|| CircuitRecord::new(provider_id, operation));
        record.health.failure_count += 1;
        record.health.consecutive_failures += 1;
        record.health.last_failure_at = Some(Utc::now().to_rfc3339());
        record.health.last_error_kind = Some(error.kind);
        record.health.latency_ms_ema = Some(update_latency(record.health.latency_ms_ema, elapsed));
        if record.health.consecutive_failures >= self.config.consecutive_failure_threshold.max(1) {
            let until = Instant::now() + self.config.open_for;
            record.opened_until = Some(until);
            record.health.state = ProviderHealthState::OpenCircuit;
            record.health.circuit_open_until = Some(
                (Utc::now() + chrono::Duration::from_std(self.config.open_for).unwrap_or_default())
                    .to_rfc3339(),
            );
        } else {
            record.health.state = ProviderHealthState::Degraded;
        }
    }
}

fn update_latency(previous: Option<f64>, elapsed: Duration) -> f64 {
    let current = elapsed.as_secs_f64() * 1_000.0;
    previous.map_or(current, |ema| ema * 0.8 + current * 0.2)
}
