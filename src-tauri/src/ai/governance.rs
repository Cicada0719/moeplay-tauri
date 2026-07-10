use crate::ai::error::{AiError, AiErrorKind, AiResult};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct CancellationToken {
    state: Arc<CancellationState>,
}

#[derive(Debug)]
struct CancellationState {
    cancelled: AtomicBool,
    generation: AtomicU64,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            state: Arc::new(CancellationState {
                cancelled: AtomicBool::new(false),
                generation: AtomicU64::new(0),
            }),
        }
    }

    pub fn guard(&self) -> CancellationGuard {
        CancellationGuard {
            state: Arc::clone(&self.state),
            generation: self.state.generation.load(Ordering::SeqCst),
        }
    }

    pub fn cancel(&self) {
        self.state.cancelled.store(true, Ordering::SeqCst);
        self.state.generation.fetch_add(1, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.state.cancelled.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone)]
pub struct CancellationGuard {
    state: Arc<CancellationState>,
    generation: u64,
}

impl CancellationGuard {
    /// Must be checked before send and again before accepting a response. The
    /// generation check rejects responses that arrive after cancellation.
    pub fn ensure_active(&self) -> AiResult<()> {
        let cancelled = self.state.cancelled.load(Ordering::SeqCst);
        let generation = self.state.generation.load(Ordering::SeqCst);
        if cancelled || generation != self.generation {
            return Err(AiError::new(
                AiErrorKind::Cancelled,
                "AI task was cancelled",
                false,
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetPolicy {
    pub monthly_hard_limit_tokens: u64,
    pub soft_warning_tokens: u64,
    pub per_task_limit_tokens: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetSnapshot {
    pub committed_tokens: u64,
    pub reserved_tokens: u64,
    pub soft_warning_reached: bool,
}

#[derive(Debug)]
pub struct BudgetLedger {
    policy: BudgetPolicy,
    state: Mutex<BudgetState>,
}

#[derive(Debug, Default)]
struct BudgetState {
    committed: u64,
    reserved: u64,
    next_id: u64,
}

#[derive(Debug)]
pub struct BudgetReservation<'a> {
    ledger: &'a BudgetLedger,
    id: u64,
    reserved: u32,
    settled: bool,
}

impl BudgetLedger {
    pub fn new(policy: BudgetPolicy, committed_tokens: u64) -> Self {
        Self {
            policy,
            state: Mutex::new(BudgetState {
                committed: committed_tokens,
                reserved: 0,
                next_id: 1,
            }),
        }
    }

    pub fn reserve(&self, estimated_tokens: u32) -> AiResult<BudgetReservation<'_>> {
        if estimated_tokens > self.policy.per_task_limit_tokens {
            return Err(AiError::new(
                AiErrorKind::BudgetExceeded,
                "AI task token estimate exceeds the per-task limit",
                false,
            ));
        }

        let mut state = self.state.lock().expect("budget ledger poisoned");
        let projected = state
            .committed
            .saturating_add(state.reserved)
            .saturating_add(u64::from(estimated_tokens));
        if projected > self.policy.monthly_hard_limit_tokens {
            return Err(AiError::new(
                AiErrorKind::BudgetExceeded,
                "AI monthly token budget would be exceeded",
                false,
            ));
        }

        let id = state.next_id;
        state.next_id = state.next_id.saturating_add(1);
        state.reserved = state.reserved.saturating_add(u64::from(estimated_tokens));
        Ok(BudgetReservation {
            ledger: self,
            id,
            reserved: estimated_tokens,
            settled: false,
        })
    }

    pub fn snapshot(&self) -> BudgetSnapshot {
        let state = self.state.lock().expect("budget ledger poisoned");
        BudgetSnapshot {
            committed_tokens: state.committed,
            reserved_tokens: state.reserved,
            soft_warning_reached: state.committed.saturating_add(state.reserved)
                >= self.policy.soft_warning_tokens,
        }
    }
}

impl BudgetReservation<'_> {
    pub fn commit(mut self, actual_tokens: u32) -> AiResult<()> {
        if actual_tokens > self.ledger.policy.per_task_limit_tokens {
            self.release_inner();
            self.settled = true;
            return Err(AiError::new(
                AiErrorKind::BudgetExceeded,
                "actual AI task usage exceeds the per-task limit",
                false,
            ));
        }

        let mut state = self.ledger.state.lock().expect("budget ledger poisoned");
        state.reserved = state.reserved.saturating_sub(u64::from(self.reserved));
        let projected = state.committed.saturating_add(u64::from(actual_tokens));
        if projected.saturating_add(state.reserved) > self.ledger.policy.monthly_hard_limit_tokens {
            self.settled = true;
            return Err(AiError::new(
                AiErrorKind::BudgetExceeded,
                "actual AI usage exceeds the monthly hard limit",
                false,
            ));
        }
        state.committed = projected;
        self.settled = true;
        Ok(())
    }

    pub fn release(mut self) {
        self.release_inner();
        self.settled = true;
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    fn release_inner(&self) {
        let mut state = self.ledger.state.lock().expect("budget ledger poisoned");
        state.reserved = state.reserved.saturating_sub(u64::from(self.reserved));
    }
}

impl Drop for BudgetReservation<'_> {
    fn drop(&mut self) {
        if !self.settled {
            self.release_inner();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitPolicy {
    pub max_requests: u32,
    pub window_ms: u64,
}

#[derive(Debug)]
pub struct FixedWindowRateLimiter {
    policy: RateLimitPolicy,
    state: Mutex<RateWindow>,
}

#[derive(Debug, Default)]
struct RateWindow {
    started_at_ms: u64,
    used: u32,
    initialized: bool,
}

impl FixedWindowRateLimiter {
    pub fn new(policy: RateLimitPolicy) -> Self {
        Self {
            policy,
            state: Mutex::new(RateWindow::default()),
        }
    }

    /// The caller supplies monotonic milliseconds, keeping tests deterministic.
    pub fn check(&self, now_ms: u64) -> AiResult<()> {
        let mut state = self.state.lock().expect("rate limiter poisoned");
        if !state.initialized || now_ms.saturating_sub(state.started_at_ms) >= self.policy.window_ms
        {
            state.started_at_ms = now_ms;
            state.used = 0;
            state.initialized = true;
        }
        if state.used >= self.policy.max_requests {
            let retry_after = self
                .policy
                .window_ms
                .saturating_sub(now_ms.saturating_sub(state.started_at_ms));
            return Err(AiError::new(
                AiErrorKind::RateLimited,
                "AI request rate limit exceeded",
                true,
            )
            .with_retry_after(retry_after));
        }
        state.used = state.used.saturating_add(1);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackAuthorization {
    #[default]
    Disabled,
    SameScopeOnly,
    ExplicitCrossScope,
}

pub fn authorize_provider_fallback(
    from_local: bool,
    to_local: bool,
    authorization: FallbackAuthorization,
) -> AiResult<()> {
    if authorization == FallbackAuthorization::Disabled {
        return Err(AiError::new(
            AiErrorKind::PolicyRejected,
            "automatic cross-provider fallback is disabled",
            false,
        ));
    }
    if from_local != to_local && authorization != FallbackAuthorization::ExplicitCrossScope {
        let message = if from_local && !to_local {
            "local-to-remote fallback requires explicit user authorization"
        } else {
            "cross-scope provider fallback requires explicit user authorization"
        };
        return Err(AiError::new(AiErrorKind::PolicyRejected, message, false));
    }
    Ok(())
}
