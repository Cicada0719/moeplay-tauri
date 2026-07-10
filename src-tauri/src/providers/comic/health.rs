use super::{provider_error, ComicProviderError};
use crate::domain::{ProviderErrorKind, ProviderHealth, ProviderHealthState};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const FAILURE_THRESHOLD: u32 = 3;
const OPEN_FOR: Duration = Duration::from_secs(30);

#[derive(Debug, Default)]
struct OperationState {
    success_count: u64,
    failure_count: u64,
    consecutive_failures: u32,
    latency_ms_ema: Option<f64>,
    last_success_at: Option<String>,
    last_failure_at: Option<String>,
    last_error_kind: Option<ProviderErrorKind>,
    opened_at: Option<Instant>,
    circuit_open_until: Option<String>,
}

/// In-memory health/circuit state. Persistence is intentionally left to the
/// existing provider_health repository during the lib integration step.
pub struct HealthTracker {
    provider_id: String,
    operations: Mutex<HashMap<String, OperationState>>,
}

impl HealthTracker {
    pub fn new(provider_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            operations: Mutex::new(HashMap::new()),
        }
    }

    pub fn before(&self, operation: &str) -> Result<Instant, ComicProviderError> {
        let mut all = self.operations.lock().expect("health mutex poisoned");
        let state = all.entry(operation.to_string()).or_default();
        if let Some(opened_at) = state.opened_at {
            if opened_at.elapsed() < OPEN_FOR {
                return Err(ComicProviderError::CircuitOpen(format!(
                    "{}:{} circuit open",
                    self.provider_id, operation
                )));
            }
            // Half-open: allow one request and keep the circuit closed while it
            // is being probed. A concurrent caller can still be rejected by the
            // next integration layer if strict single-flight is required.
            state.opened_at = None;
            state.circuit_open_until = None;
        }
        Ok(Instant::now())
    }

    pub fn success(&self, operation: &str, started: Instant) {
        let mut all = self.operations.lock().expect("health mutex poisoned");
        let state = all.entry(operation.to_string()).or_default();
        let latency = started.elapsed().as_secs_f64() * 1000.0;
        state.success_count += 1;
        state.consecutive_failures = 0;
        state.last_success_at = Some(Utc::now().to_rfc3339());
        state.last_error_kind = None;
        state.opened_at = None;
        state.circuit_open_until = None;
        state.latency_ms_ema = Some(match state.latency_ms_ema {
            Some(previous) => previous * 0.8 + latency * 0.2,
            None => latency,
        });
    }

    pub fn failure(&self, operation: &str, error: &ComicProviderError) {
        let mut all = self.operations.lock().expect("health mutex poisoned");
        let state = all.entry(operation.to_string()).or_default();
        let provider_error = error.provider_error();
        state.failure_count += 1;
        state.consecutive_failures += 1;
        state.last_failure_at = Some(Utc::now().to_rfc3339());
        state.last_error_kind = Some(provider_error.kind);
        if state.consecutive_failures >= FAILURE_THRESHOLD {
            state.opened_at = Some(Instant::now());
            state.circuit_open_until =
                Some((Utc::now() + chrono::Duration::from_std(OPEN_FOR).unwrap()).to_rfc3339());
        }
    }

    pub fn snapshot(&self, operation: &str) -> ProviderHealth {
        let mut all = self.operations.lock().expect("health mutex poisoned");
        let state = all.entry(operation.to_string()).or_default();
        let is_open = state
            .opened_at
            .map(|at| at.elapsed() < OPEN_FOR)
            .unwrap_or(false);
        if !is_open {
            state.opened_at = None;
            state.circuit_open_until = None;
        }
        let degraded = state.consecutive_failures > 0;
        ProviderHealth {
            provider_id: self.provider_id.clone(),
            operation: operation.to_string(),
            state: if is_open {
                ProviderHealthState::OpenCircuit
            } else if degraded {
                ProviderHealthState::Degraded
            } else {
                ProviderHealthState::Healthy
            },
            success_count: state.success_count,
            failure_count: state.failure_count,
            consecutive_failures: state.consecutive_failures,
            latency_ms_ema: state.latency_ms_ema,
            last_success_at: state.last_success_at.clone(),
            last_failure_at: state.last_failure_at.clone(),
            circuit_open_until: state.circuit_open_until.clone(),
            last_error_kind: state.last_error_kind,
        }
    }

    pub fn unavailable(&self, operation: &str) -> ComicProviderError {
        provider_error(
            &self.provider_id,
            operation,
            ProviderErrorKind::Network,
            "provider circuit is open",
            true,
        )
    }
}
