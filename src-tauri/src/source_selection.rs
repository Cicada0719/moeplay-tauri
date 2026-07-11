//! Source health scoring and selection policy.
//!
//! This module deliberately has no database, Tauri, or provider dependencies.  Callers may
//! persist [`SourceHealth`] in their preferred repository and project it into a source-center
//! DTO.  It is intended to be the single policy used by anime, comic, and external runtimes.

use std::collections::BTreeSet;

/// Persistent health facts for one provider/source.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceHealth {
    pub successful_checks: u64,
    pub failed_checks: u64,
    pub consecutive_failures: u32,
    pub latency_ema_ms: Option<f64>,
    pub circuit: CircuitState,
    pub last_checked_at_ms: Option<i64>,
}

impl Default for SourceHealth {
    fn default() -> Self {
        Self {
            successful_checks: 0,
            failed_checks: 0,
            consecutive_failures: 0,
            latency_ema_ms: None,
            circuit: CircuitState::Closed,
            last_checked_at_ms: None,
        }
    }
}

impl SourceHealth {
    /// Returns the observed success rate, or `None` when no outcome has been recorded yet.
    pub fn success_rate(&self) -> Option<f64> {
        let attempts = self.successful_checks.saturating_add(self.failed_checks);
        (attempts > 0).then(|| self.successful_checks as f64 / attempts as f64)
    }

    /// Whether automatic selection must currently exclude this source.
    pub fn circuit_is_open(&self, now_ms: i64) -> bool {
        matches!(self.circuit, CircuitState::Open { until_ms } if now_ms < until_ms)
    }

    /// Records a successful request/check and closes any expired or half-open circuit.
    pub fn record_success(
        &mut self,
        latency_ms: Option<f64>,
        now_ms: i64,
        config: &CircuitBreakerConfig,
    ) {
        self.successful_checks = self.successful_checks.saturating_add(1);
        self.consecutive_failures = 0;
        self.circuit = CircuitState::Closed;
        self.last_checked_at_ms = Some(now_ms);
        self.update_latency(latency_ms, config);
    }

    /// Records a failed request/check and opens the circuit at the configured threshold.
    pub fn record_failure(
        &mut self,
        latency_ms: Option<f64>,
        now_ms: i64,
        config: &CircuitBreakerConfig,
    ) {
        self.failed_checks = self.failed_checks.saturating_add(1);
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.last_checked_at_ms = Some(now_ms);
        self.update_latency(latency_ms, config);

        if self.consecutive_failures >= config.failure_threshold.max(1) {
            self.circuit = CircuitState::Open {
                until_ms: now_ms.saturating_add(config.open_for_ms.max(0)),
            };
        }
    }

    fn update_latency(&mut self, latency_ms: Option<f64>, config: &CircuitBreakerConfig) {
        let Some(latency_ms) = latency_ms.filter(|value| value.is_finite() && *value >= 0.0) else {
            return;
        };

        let alpha = config.latency_ema_alpha.clamp(0.0, 1.0);
        self.latency_ema_ms = Some(match self.latency_ema_ms {
            Some(previous) => alpha * latency_ms + (1.0 - alpha) * previous,
            None => latency_ms,
        });
    }
}

/// Current circuit state.  An expired `Open` state is eligible for a new automatic attempt;
/// a successful attempt closes it, while another failure opens it for a fresh interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open { until_ms: i64 },
}

/// Tuning values for per-source health recording.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub open_for_ms: i64,
    pub latency_ema_alpha: f64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            open_for_ms: 5 * 60 * 1_000,
            latency_ema_alpha: 0.25,
        }
    }
}

/// Minimal source projection consumed by the selection policy.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceCandidate {
    pub provider_id: String,
    pub enabled: bool,
    /// User-controlled preference in the inclusive range configured by the policy.
    pub priority: i32,
    pub health: SourceHealth,
}

impl SourceCandidate {
    pub fn new(provider_id: impl Into<String>, enabled: bool, priority: i32) -> Self {
        Self {
            provider_id: provider_id.into(),
            enabled,
            priority,
            health: SourceHealth::default(),
        }
    }
}

/// Weights used for an automatic ranking.  The values are normalized before use, so callers may
/// tune their relative importance without having to make them sum to one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceScoreWeights {
    pub success_rate: f64,
    pub consecutive_failure_resilience: f64,
    pub latency: f64,
    pub user_priority: f64,
}

impl Default for SourceScoreWeights {
    fn default() -> Self {
        Self {
            success_rate: 0.45,
            consecutive_failure_resilience: 0.25,
            latency: 0.15,
            user_priority: 0.15,
        }
    }
}

/// Security/policy controls that ranking and manual selection must obey.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceSelectionPolicy {
    /// Source IDs prohibited by product, security, runtime, NSFW, or tenant policy.
    pub blocked_provider_ids: BTreeSet<String>,
    pub priority_min: i32,
    pub priority_max: i32,
    /// A latency at or above this value receives a latency component of zero.
    pub latency_reference_ms: f64,
    /// Neutral health used before a source has an observed outcome.
    pub unknown_success_rate: f64,
    /// Neutral latency component used before a latency sample exists.
    pub unknown_latency_score: f64,
    pub weights: SourceScoreWeights,
}

impl Default for SourceSelectionPolicy {
    fn default() -> Self {
        Self {
            blocked_provider_ids: BTreeSet::new(),
            priority_min: -100,
            priority_max: 100,
            latency_reference_ms: 5_000.0,
            unknown_success_rate: 0.5,
            unknown_latency_score: 0.5,
            weights: SourceScoreWeights::default(),
        }
    }
}

impl SourceSelectionPolicy {
    pub fn blocks(&self, provider_id: &str) -> bool {
        self.blocked_provider_ids.contains(provider_id)
    }
}

/// A request for automatic ranking or for one user-selected source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceSelectionRequest {
    Automatic,
    Manual { provider_id: String },
}

/// The individual components make source ordering explainable in the Source Center UI.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceScore {
    pub total: f64,
    pub success_rate: f64,
    pub failure_resilience: f64,
    pub latency: f64,
    pub user_priority: f64,
}

/// Why an otherwise known source cannot be selected for this request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceRejection {
    NotFound,
    PolicyBlocked,
    Disabled,
    CircuitOpen,
    NoEligibleSource,
}

/// An explainable selection result.  A manual selection intentionally bypasses health ranking,
/// but it never bypasses [`SourceSelectionPolicy::blocked_provider_ids`].
#[derive(Debug, Clone, PartialEq)]
pub enum SourceSelectionResult {
    Selected {
        provider_id: String,
        score: Option<SourceScore>,
        manual: bool,
    },
    Rejected(SourceRejection),
}

/// Scores one candidate under the supplied policy.  The return value is always bounded to 0..=1.
pub fn score_source(candidate: &SourceCandidate, policy: &SourceSelectionPolicy) -> SourceScore {
    let success_rate = candidate
        .health
        .success_rate()
        .unwrap_or(policy.unknown_success_rate)
        .clamp(0.0, 1.0);
    let failure_resilience = 1.0 / (1.0 + candidate.health.consecutive_failures as f64);
    let latency = candidate
        .health
        .latency_ema_ms
        .map(|value| {
            let reference = policy.latency_reference_ms.max(1.0);
            (1.0 - value.max(0.0) / reference).clamp(0.0, 1.0)
        })
        .unwrap_or(policy.unknown_latency_score)
        .clamp(0.0, 1.0);
    let priority_span = (policy.priority_max - policy.priority_min).max(1) as f64;
    let user_priority = ((candidate
        .priority
        .clamp(policy.priority_min, policy.priority_max)
        - policy.priority_min) as f64
        / priority_span)
        .clamp(0.0, 1.0);

    let weights = policy.weights;
    let weight_sum = (weights.success_rate.max(0.0)
        + weights.consecutive_failure_resilience.max(0.0)
        + weights.latency.max(0.0)
        + weights.user_priority.max(0.0))
    .max(f64::EPSILON);
    let total = (success_rate * weights.success_rate.max(0.0)
        + failure_resilience * weights.consecutive_failure_resilience.max(0.0)
        + latency * weights.latency.max(0.0)
        + user_priority * weights.user_priority.max(0.0))
        / weight_sum;

    SourceScore {
        total: total.clamp(0.0, 1.0),
        success_rate,
        failure_resilience,
        latency,
        user_priority,
    }
}

/// Selects a source without any provider-specific behaviour.
///
/// Automatic selection excludes disabled and circuit-open sources.  A manual selection is a
/// deliberate user override of that ranking/health exclusion, but a policy block remains final.
pub fn select_source(
    candidates: &[SourceCandidate],
    request: &SourceSelectionRequest,
    policy: &SourceSelectionPolicy,
    now_ms: i64,
) -> SourceSelectionResult {
    match request {
        SourceSelectionRequest::Manual { provider_id } => {
            let Some(candidate) = candidates
                .iter()
                .find(|item| item.provider_id == *provider_id)
            else {
                return SourceSelectionResult::Rejected(SourceRejection::NotFound);
            };
            if policy.blocks(&candidate.provider_id) {
                return SourceSelectionResult::Rejected(SourceRejection::PolicyBlocked);
            }
            SourceSelectionResult::Selected {
                provider_id: candidate.provider_id.clone(),
                score: None,
                manual: true,
            }
        }
        SourceSelectionRequest::Automatic => {
            let mut best: Option<(&SourceCandidate, SourceScore)> = None;
            let mut saw_candidate = false;
            let mut saw_policy_block = false;
            let mut saw_disabled = false;
            let mut saw_circuit_open = false;

            for candidate in candidates {
                saw_candidate = true;
                if policy.blocks(&candidate.provider_id) {
                    saw_policy_block = true;
                    continue;
                }
                if !candidate.enabled {
                    saw_disabled = true;
                    continue;
                }
                if candidate.health.circuit_is_open(now_ms) {
                    saw_circuit_open = true;
                    continue;
                }

                let score = score_source(candidate, policy);
                let replace = best.as_ref().is_none_or(|(current, current_score)| {
                    score.total > current_score.total
                        || (score.total == current_score.total
                            && candidate.provider_id < current.provider_id)
                });
                if replace {
                    best = Some((candidate, score));
                }
            }

            if let Some((candidate, score)) = best {
                SourceSelectionResult::Selected {
                    provider_id: candidate.provider_id.clone(),
                    score: Some(score),
                    manual: false,
                }
            } else if !saw_candidate {
                SourceSelectionResult::Rejected(SourceRejection::NoEligibleSource)
            } else if saw_policy_block && !saw_disabled && !saw_circuit_open {
                SourceSelectionResult::Rejected(SourceRejection::PolicyBlocked)
            } else if saw_disabled && !saw_circuit_open {
                SourceSelectionResult::Rejected(SourceRejection::Disabled)
            } else if saw_circuit_open {
                SourceSelectionResult::Rejected(SourceRejection::CircuitOpen)
            } else {
                SourceSelectionResult::Rejected(SourceRejection::NoEligibleSource)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, priority: i32) -> SourceCandidate {
        SourceCandidate::new(id, true, priority)
    }

    #[test]
    fn automatic_selection_excludes_disabled_and_open_circuits() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            open_for_ms: 1_000,
            ..Default::default()
        };
        let mut disabled = candidate("disabled", 100);
        disabled.enabled = false;
        let mut circuit_open = candidate("open", 100);
        circuit_open.health.record_failure(None, 100, &config);
        circuit_open.health.record_failure(None, 101, &config);
        let mut healthy = candidate("healthy", 0);
        healthy.health.record_success(Some(300.0), 100, &config);

        let result = select_source(
            &[disabled, circuit_open, healthy],
            &SourceSelectionRequest::Automatic,
            &SourceSelectionPolicy::default(),
            102,
        );

        assert!(matches!(
            result,
            SourceSelectionResult::Selected { provider_id, manual: false, .. } if provider_id == "healthy"
        ));
    }

    #[test]
    fn score_combines_observed_health_latency_and_user_priority() {
        let config = CircuitBreakerConfig::default();
        let mut reliable = candidate("reliable", 10);
        for _ in 0..10 {
            reliable.health.record_success(Some(150.0), 100, &config);
        }

        let mut preferred_but_unhealthy = candidate("preferred", 100);
        preferred_but_unhealthy
            .health
            .record_success(Some(4_500.0), 100, &config);
        for _ in 0..5 {
            preferred_but_unhealthy
                .health
                .record_failure(Some(4_500.0), 100, &config);
        }

        let policy = SourceSelectionPolicy::default();
        let reliable_score = score_source(&reliable, &policy);
        let preferred_score = score_source(&preferred_but_unhealthy, &policy);
        assert!(reliable_score.total > preferred_score.total);

        let result = select_source(
            &[preferred_but_unhealthy, reliable],
            &SourceSelectionRequest::Automatic,
            &policy,
            100,
        );
        assert!(matches!(
            result,
            SourceSelectionResult::Selected { provider_id, .. } if provider_id == "reliable"
        ));
    }

    #[test]
    fn manual_selection_bypasses_ranking_but_never_policy_blocks() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            open_for_ms: 1_000,
            ..Default::default()
        };
        let mut unhealthy = candidate("manual", -100);
        unhealthy.enabled = false;
        unhealthy.health.record_failure(None, 10, &config);
        let healthy = candidate("auto", 100);
        let request = SourceSelectionRequest::Manual {
            provider_id: "manual".to_owned(),
        };

        assert!(matches!(
            select_source(&[unhealthy.clone(), healthy.clone()], &request, &SourceSelectionPolicy::default(), 11),
            SourceSelectionResult::Selected { provider_id, manual: true, score: None } if provider_id == "manual"
        ));

        let mut policy = SourceSelectionPolicy::default();
        policy.blocked_provider_ids.insert("manual".to_owned());
        assert_eq!(
            select_source(&[unhealthy, healthy], &request, &policy, 11),
            SourceSelectionResult::Rejected(SourceRejection::PolicyBlocked)
        );
    }

    #[test]
    fn circuit_opens_after_threshold_and_reopens_for_a_new_attempt_after_cooldown() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            open_for_ms: 500,
            latency_ema_alpha: 0.5,
        };
        let mut health = SourceHealth::default();
        health.record_failure(Some(100.0), 1_000, &config);
        assert!(!health.circuit_is_open(1_001));
        health.record_failure(Some(300.0), 1_100, &config);
        assert_eq!(health.circuit, CircuitState::Open { until_ms: 1_600 });
        assert!(health.circuit_is_open(1_599));
        assert!(!health.circuit_is_open(1_600));
        assert_eq!(health.latency_ema_ms, Some(200.0));

        health.record_success(Some(400.0), 1_600, &config);
        assert_eq!(health.circuit, CircuitState::Closed);
        assert_eq!(health.consecutive_failures, 0);
        assert_eq!(health.latency_ema_ms, Some(300.0));
    }

    #[test]
    fn automatic_selection_is_deterministic_for_equal_scores() {
        let first = candidate("alpha", 0);
        let second = candidate("beta", 0);
        let result = select_source(
            &[second, first],
            &SourceSelectionRequest::Automatic,
            &SourceSelectionPolicy::default(),
            0,
        );
        assert!(matches!(
            result,
            SourceSelectionResult::Selected { provider_id, .. } if provider_id == "alpha"
        ));
    }
}
