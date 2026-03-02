//! Spending controls and budget enforcement for billing.
//!
//! Provides placeholder structs for tracking, limiting, and circuit-breaking
//! payment activity. These work alongside [`BillingConfig::spending`](super::config::SpendingConfig)
//! to enforce per-user/per-tenant budgets.
//!
//! # Status
//!
//! This module is **placeholder scaffolding**. All methods return
//! `anyhow::bail!` or safe defaults. Real implementation will follow
//! once the billing strategy is finalized.

use super::traits::{Currency, Money};
use serde::{Deserialize, Serialize};

// ── Spending Limit ───────────────────────────────────────────────

/// A spending cap applied to a user, tenant, or global scope.
///
/// TODO: Implement storage-backed limit checking (e.g. SQLite or
/// the existing cost tracker infrastructure).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingLimit {
    /// Opaque identifier for the limit scope (e.g. user ID, tenant ID, "global").
    pub scope_id: String,
    /// Maximum spend in minor currency units per period.
    pub max_amount_minor: u64,
    /// Currency for this limit.
    pub currency: Currency,
    /// Period over which the limit applies.
    pub period: LimitPeriod,
}

/// Time period for a spending limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LimitPeriod {
    /// Per-transaction limit.
    Transaction,
    /// Rolling 24-hour window.
    Daily,
    /// Calendar month.
    Monthly,
}

impl SpendingLimit {
    /// Create a new spending limit.
    pub fn new(
        scope_id: impl Into<String>,
        max_amount_minor: u64,
        currency: Currency,
        period: LimitPeriod,
    ) -> Self {
        Self {
            scope_id: scope_id.into(),
            max_amount_minor,
            currency,
            period,
        }
    }

    /// Check whether a proposed charge would exceed this limit.
    ///
    /// TODO: Query actual spend-to-date from storage before comparing.
    /// Currently always returns `Ok(true)` (allowed) as a safe placeholder.
    pub fn check(&self, _proposed: &Money) -> anyhow::Result<bool> {
        // TODO: Look up current period spend from storage and compare
        // against self.max_amount_minor.
        Ok(true)
    }
}

// ── Usage Tracker ────────────────────────────────────────────────

/// Tracks current spend against configured limits.
///
/// TODO: Back with persistent storage (SQLite or the existing cost
/// tracker). Current implementation is in-memory only and will not
/// survive restarts.
#[derive(Debug)]
pub struct UsageTracker {
    /// Accumulated spend in minor units, keyed by scope ID.
    totals: std::collections::HashMap<String, u64>,
}

impl UsageTracker {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self {
            totals: std::collections::HashMap::new(),
        }
    }

    /// Record a charge against a scope.
    ///
    /// TODO: Persist to storage.
    pub fn record(&mut self, scope_id: &str, amount_minor: u64) {
        *self.totals.entry(scope_id.to_string()).or_insert(0) += amount_minor;
    }

    /// Get current accumulated spend for a scope.
    pub fn current_spend(&self, scope_id: &str) -> u64 {
        self.totals.get(scope_id).copied().unwrap_or(0)
    }

    /// Check whether a proposed charge would exceed any of the given limits.
    ///
    /// Returns `Ok(true)` if all limits pass, `Ok(false)` if any limit
    /// would be exceeded.
    pub fn check_limits(
        &self,
        scope_id: &str,
        proposed_minor: u64,
        limits: &[SpendingLimit],
    ) -> anyhow::Result<bool> {
        for limit in limits {
            if limit.scope_id != scope_id {
                continue;
            }
            let current = self.current_spend(scope_id);
            if current.saturating_add(proposed_minor) > limit.max_amount_minor {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

// ── Circuit Breaker ──────────────────────────────────────────────

/// Auto-halts payment processing when consecutive failures exceed a threshold.
///
/// TODO: Add time-based recovery (half-open state) and persistent state
/// so the breaker survives restarts.
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Number of consecutive failures before the breaker trips.
    threshold: u32,
    /// Current consecutive failure count.
    failure_count: u32,
    /// Whether the breaker is currently open (tripped).
    open: bool,
}

/// Current state of the circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation — payments can proceed.
    Closed,
    /// Breaker tripped — payments are halted.
    Open,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given failure threshold.
    pub fn new(threshold: u32) -> Self {
        Self {
            threshold,
            failure_count: 0,
            open: false,
        }
    }

    /// Current state of the breaker.
    pub fn state(&self) -> CircuitState {
        if self.open {
            CircuitState::Open
        } else {
            CircuitState::Closed
        }
    }

    /// Record a successful payment — resets the failure counter.
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.open = false;
    }

    /// Record a failed payment — increments counter and trips if threshold hit.
    pub fn record_failure(&mut self) {
        self.failure_count = self.failure_count.saturating_add(1);
        if self.failure_count >= self.threshold {
            self.open = true;
        }
    }

    /// Whether a payment attempt is currently allowed.
    pub fn is_allowed(&self) -> bool {
        !self.open
    }

    /// Manually reset the breaker (e.g. operator override).
    pub fn reset(&mut self) {
        self.failure_count = 0;
        self.open = false;
    }
}

// ── Prepayment Balance ───────────────────────────────────────────

/// Tracks pre-paid balance for a user or tenant.
///
/// TODO: Back with persistent storage. Current implementation is
/// in-memory only and will not survive restarts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepaymentBalance {
    /// Opaque identifier for the balance owner (e.g. tenant ID).
    pub owner_id: String,
    /// Current balance in minor currency units.
    pub balance_minor: u64,
    /// Currency of the balance.
    pub currency: Currency,
}

impl PrepaymentBalance {
    /// Create a new prepayment balance.
    pub fn new(
        owner_id: impl Into<String>,
        initial_balance_minor: u64,
        currency: Currency,
    ) -> Self {
        Self {
            owner_id: owner_id.into(),
            balance_minor: initial_balance_minor,
            currency,
        }
    }

    /// Add funds to the balance.
    ///
    /// TODO: Persist to storage.
    pub fn credit(&mut self, amount_minor: u64) {
        self.balance_minor = self.balance_minor.saturating_add(amount_minor);
    }

    /// Attempt to debit funds from the balance.
    ///
    /// Returns `true` if the debit succeeded (sufficient funds),
    /// `false` if insufficient balance.
    ///
    /// TODO: Persist to storage.
    pub fn debit(&mut self, amount_minor: u64) -> bool {
        if self.balance_minor >= amount_minor {
            self.balance_minor -= amount_minor;
            true
        } else {
            false
        }
    }

    /// Current balance in minor units.
    pub fn balance(&self) -> u64 {
        self.balance_minor
    }

    /// Whether the balance can cover a proposed charge.
    pub fn can_cover(&self, amount_minor: u64) -> bool {
        self.balance_minor >= amount_minor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SpendingLimit ────────────────────────────────────────────

    #[test]
    fn spending_limit_creation() {
        let limit = SpendingLimit::new("tenant_001", 100_000, Currency::Usd, LimitPeriod::Daily);
        assert_eq!(limit.scope_id, "tenant_001");
        assert_eq!(limit.max_amount_minor, 100_000);
        assert_eq!(limit.period, LimitPeriod::Daily);
    }

    #[test]
    fn spending_limit_check_placeholder_allows() {
        let limit = SpendingLimit::new("tenant_001", 100_000, Currency::Usd, LimitPeriod::Daily);
        let proposed = Money {
            amount_minor: 5000,
            currency: Currency::Usd,
        };
        assert!(limit.check(&proposed).unwrap());
    }

    // ── UsageTracker ─────────────────────────────────────────────

    #[test]
    fn usage_tracker_records_and_reports_spend() {
        let mut tracker = UsageTracker::new();
        assert_eq!(tracker.current_spend("tenant_001"), 0);

        tracker.record("tenant_001", 1000);
        tracker.record("tenant_001", 2500);
        assert_eq!(tracker.current_spend("tenant_001"), 3500);
        assert_eq!(tracker.current_spend("tenant_002"), 0);
    }

    #[test]
    fn usage_tracker_check_limits_passes_within_budget() {
        let mut tracker = UsageTracker::new();
        tracker.record("tenant_001", 5000);

        let limits = vec![SpendingLimit::new(
            "tenant_001",
            10_000,
            Currency::Usd,
            LimitPeriod::Daily,
        )];

        assert!(tracker.check_limits("tenant_001", 3000, &limits).unwrap());
    }

    #[test]
    fn usage_tracker_check_limits_rejects_over_budget() {
        let mut tracker = UsageTracker::new();
        tracker.record("tenant_001", 8000);

        let limits = vec![SpendingLimit::new(
            "tenant_001",
            10_000,
            Currency::Usd,
            LimitPeriod::Daily,
        )];

        assert!(!tracker.check_limits("tenant_001", 3000, &limits).unwrap());
    }

    // ── CircuitBreaker ───────────────────────────────────────────

    #[test]
    fn circuit_breaker_starts_closed() {
        let breaker = CircuitBreaker::new(3);
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.is_allowed());
    }

    #[test]
    fn circuit_breaker_trips_at_threshold() {
        let mut breaker = CircuitBreaker::new(3);
        breaker.record_failure();
        breaker.record_failure();
        assert!(breaker.is_allowed());

        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(!breaker.is_allowed());
    }

    #[test]
    fn circuit_breaker_resets_on_success() {
        let mut breaker = CircuitBreaker::new(2);
        breaker.record_failure();
        breaker.record_failure();
        assert!(!breaker.is_allowed());

        breaker.record_success();
        assert!(breaker.is_allowed());
        assert_eq!(breaker.state(), CircuitState::Closed);
    }

    #[test]
    fn circuit_breaker_manual_reset() {
        let mut breaker = CircuitBreaker::new(1);
        breaker.record_failure();
        assert!(!breaker.is_allowed());

        breaker.reset();
        assert!(breaker.is_allowed());
    }

    // ── PrepaymentBalance ────────────────────────────────────────

    #[test]
    fn prepayment_balance_credit_and_debit() {
        let mut balance = PrepaymentBalance::new("tenant_001", 10_000, Currency::Usd);
        assert_eq!(balance.balance(), 10_000);

        balance.credit(5_000);
        assert_eq!(balance.balance(), 15_000);

        assert!(balance.debit(3_000));
        assert_eq!(balance.balance(), 12_000);
    }

    #[test]
    fn prepayment_balance_debit_rejects_insufficient_funds() {
        let mut balance = PrepaymentBalance::new("tenant_001", 1_000, Currency::Usd);
        assert!(!balance.debit(2_000));
        assert_eq!(balance.balance(), 1_000);
    }

    #[test]
    fn prepayment_balance_can_cover() {
        let balance = PrepaymentBalance::new("tenant_001", 5_000, Currency::Usd);
        assert!(balance.can_cover(5_000));
        assert!(balance.can_cover(4_999));
        assert!(!balance.can_cover(5_001));
    }

    #[test]
    fn prepayment_balance_serde_roundtrip() {
        let balance = PrepaymentBalance::new("tenant_001", 7_500, Currency::Eur);
        let json = serde_json::to_string(&balance).unwrap();
        let parsed: PrepaymentBalance = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.owner_id, "tenant_001");
        assert_eq!(parsed.balance_minor, 7_500);
        assert_eq!(parsed.currency, Currency::Eur);
    }
}
