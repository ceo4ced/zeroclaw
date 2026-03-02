//! Spending controls and budget enforcement for billing.
//!
//! Provides structures for tracking usage, enforcing daily limits,
//! managing prepaid balances with overdraft, and circuit-breaking
//! payment failures.
//!
//! # MVP Model
//!
//! - Flat $0.10/task deducted from prepaid balance.
//! - $5/day default spending limit (soft cap: finishes current task).
//! - $2 overdraft allowed before new tasks are blocked.
//! - Warnings at $2 and $1 remaining balance.

use serde::{Deserialize, Serialize};

// ── Warning Level ───────────────────────────────────────────────

/// Low-balance warning severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningLevel {
    /// No warning — balance is healthy.
    None,
    /// Balance at or below $2.00 — first alert.
    Low,
    /// Balance at or below $1.00 — urgent alert.
    Critical,
}

impl std::fmt::Display for WarningLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Low => write!(f, "low"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

// ── Spending Limit ──────────────────────────────────────────────

/// A spending cap applied to a user scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingLimit {
    /// Opaque identifier for the limit scope (e.g. user ID).
    pub scope_id: String,
    /// Maximum spend in cents per period.
    pub max_amount_cents: u32,
    /// Period over which the limit applies.
    pub period: LimitPeriod,
}

/// Time period for a spending limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LimitPeriod {
    /// Rolling 24-hour window.
    Daily,
}

impl SpendingLimit {
    /// Create a new daily spending limit.
    pub fn daily(scope_id: impl Into<String>, max_amount_cents: u32) -> Self {
        Self {
            scope_id: scope_id.into(),
            max_amount_cents,
            period: LimitPeriod::Daily,
        }
    }
}

// ── Usage Tracker ───────────────────────────────────────────────

/// Tracks daily spend against configured limits.
///
/// In-memory only for MVP. TODO: persist to storage.
#[derive(Debug)]
pub struct UsageTracker {
    /// Accumulated spend in cents for the current day, keyed by scope ID.
    daily_totals: std::collections::HashMap<String, u32>,
    /// Number of tasks completed today, keyed by scope ID.
    daily_task_counts: std::collections::HashMap<String, u32>,
}

impl UsageTracker {
    pub fn new() -> Self {
        Self {
            daily_totals: std::collections::HashMap::new(),
            daily_task_counts: std::collections::HashMap::new(),
        }
    }

    /// Record a completed task charge.
    pub fn record_task(&mut self, scope_id: &str, cost_cents: u32) {
        *self.daily_totals.entry(scope_id.to_string()).or_insert(0) += cost_cents;
        *self
            .daily_task_counts
            .entry(scope_id.to_string())
            .or_insert(0) += 1;
    }

    /// Get current daily spend in cents for a scope.
    pub fn daily_spend_cents(&self, scope_id: &str) -> u32 {
        self.daily_totals.get(scope_id).copied().unwrap_or(0)
    }

    /// Get number of tasks completed today for a scope.
    pub fn daily_task_count(&self, scope_id: &str) -> u32 {
        self.daily_task_counts.get(scope_id).copied().unwrap_or(0)
    }

    /// Check if a proposed task charge would exceed the daily limit.
    ///
    /// This is a soft cap check: returns `false` if the limit would be
    /// exceeded, but the caller should still allow the current in-flight
    /// task to complete before blocking new tasks.
    pub fn would_exceed_daily_limit(
        &self,
        scope_id: &str,
        proposed_cents: u32,
        limit: &SpendingLimit,
    ) -> bool {
        if limit.scope_id != scope_id {
            return false;
        }
        let current = self.daily_spend_cents(scope_id);
        current.saturating_add(proposed_cents) > limit.max_amount_cents
    }

    /// Reset daily totals (call at day boundary).
    pub fn reset_daily(&mut self) {
        self.daily_totals.clear();
        self.daily_task_counts.clear();
    }
}

// ── Circuit Breaker ─────────────────────────────────────────────

/// Auto-halts payment processing when consecutive failures exceed a threshold.
#[derive(Debug)]
pub struct CircuitBreaker {
    threshold: u32,
    failure_count: u32,
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
    pub fn new(threshold: u32) -> Self {
        Self {
            threshold,
            failure_count: 0,
            open: false,
        }
    }

    pub fn state(&self) -> CircuitState {
        if self.open {
            CircuitState::Open
        } else {
            CircuitState::Closed
        }
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        self.open = false;
    }

    pub fn record_failure(&mut self) {
        self.failure_count = self.failure_count.saturating_add(1);
        if self.failure_count >= self.threshold {
            self.open = true;
        }
    }

    pub fn is_allowed(&self) -> bool {
        !self.open
    }

    pub fn reset(&mut self) {
        self.failure_count = 0;
        self.open = false;
    }
}

// ── Prepayment Balance ──────────────────────────────────────────

/// Tracks pre-paid balance for a user.
///
/// Uses i64 for balance to support negative overdraft (up to the
/// configured overdraft limit). All amounts in cents.
///
/// In-memory only for MVP. TODO: persist to storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepaymentBalance {
    /// Opaque identifier for the balance owner.
    pub owner_id: String,
    /// Current balance in cents. Can go negative up to overdraft limit.
    pub balance_cents: i64,
    /// Maximum allowed overdraft in cents (e.g. 200 = $2.00).
    #[serde(default = "default_overdraft")]
    pub max_overdraft_cents: u32,
}

fn default_overdraft() -> u32 {
    200
}

impl PrepaymentBalance {
    /// Create a new prepayment balance with zero funds.
    pub fn new(owner_id: impl Into<String>) -> Self {
        Self {
            owner_id: owner_id.into(),
            balance_cents: 0,
            max_overdraft_cents: default_overdraft(),
        }
    }

    /// Create with an initial balance.
    pub fn with_balance(owner_id: impl Into<String>, initial_cents: i64) -> Self {
        Self {
            owner_id: owner_id.into(),
            balance_cents: initial_cents,
            max_overdraft_cents: default_overdraft(),
        }
    }

    /// Add funds (top-up). Returns error if amount is below minimum.
    pub fn top_up(&mut self, amount_cents: u32, min_topup_cents: u32) -> anyhow::Result<()> {
        if amount_cents < min_topup_cents {
            anyhow::bail!(
                "Minimum top-up is ${:.2}, got ${:.2}",
                f64::from(min_topup_cents) / 100.0,
                f64::from(amount_cents) / 100.0
            );
        }
        self.balance_cents = self.balance_cents.saturating_add(i64::from(amount_cents));
        Ok(())
    }

    /// Debit a task charge from the balance.
    ///
    /// Allows the balance to go negative up to the overdraft limit
    /// (soft cap behavior — the current task is allowed to complete).
    /// Returns `true` if the debit succeeded, `false` if it would
    /// exceed the overdraft limit.
    pub fn debit_task(&mut self, cost_cents: u32) -> bool {
        let new_balance = self.balance_cents - i64::from(cost_cents);
        let floor = -(i64::from(self.max_overdraft_cents));
        if new_balance >= floor {
            self.balance_cents = new_balance;
            true
        } else {
            false
        }
    }

    /// Current balance in cents (can be negative).
    pub fn balance(&self) -> i64 {
        self.balance_cents
    }

    /// Format balance as a dollar string (e.g. "$4.50" or "-$1.20").
    pub fn balance_display(&self) -> String {
        format_cents(self.balance_cents)
    }

    /// Whether a new task can be started (balance above overdraft floor).
    pub fn can_start_task(&self, task_cost_cents: u32) -> bool {
        let projected = self.balance_cents - i64::from(task_cost_cents);
        let floor = -(i64::from(self.max_overdraft_cents));
        projected >= floor
    }

    /// Check warning level based on current balance.
    pub fn warning_level(&self, thresholds: &[u32]) -> WarningLevel {
        if thresholds.len() >= 2 {
            let low_threshold = i64::from(thresholds[0]);
            let critical_threshold = i64::from(thresholds[1]);
            if self.balance_cents <= critical_threshold {
                return WarningLevel::Critical;
            }
            if self.balance_cents <= low_threshold {
                return WarningLevel::Low;
            }
        } else if !thresholds.is_empty() {
            let threshold = i64::from(thresholds[0]);
            if self.balance_cents <= threshold {
                return WarningLevel::Low;
            }
        }
        WarningLevel::None
    }
}

/// Format a cent amount as a dollar display string.
pub fn format_cents(cents: i64) -> String {
    if cents < 0 {
        let abs_cents = cents.unsigned_abs();
        format!("-${:.2}", abs_cents as f64 / 100.0)
    } else {
        format!("${:.2}", cents as f64 / 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── WarningLevel ────────────────────────────────────────────

    #[test]
    fn warning_level_display() {
        assert_eq!(WarningLevel::None.to_string(), "none");
        assert_eq!(WarningLevel::Low.to_string(), "low");
        assert_eq!(WarningLevel::Critical.to_string(), "critical");
    }

    // ── SpendingLimit ───────────────────────────────────────────

    #[test]
    fn spending_limit_daily() {
        let limit = SpendingLimit::daily("user_a", 500);
        assert_eq!(limit.scope_id, "user_a");
        assert_eq!(limit.max_amount_cents, 500);
        assert_eq!(limit.period, LimitPeriod::Daily);
    }

    // ── UsageTracker ────────────────────────────────────────────

    #[test]
    fn usage_tracker_records_tasks() {
        let mut tracker = UsageTracker::new();
        assert_eq!(tracker.daily_spend_cents("user_a"), 0);
        assert_eq!(tracker.daily_task_count("user_a"), 0);

        tracker.record_task("user_a", 10);
        tracker.record_task("user_a", 10);
        assert_eq!(tracker.daily_spend_cents("user_a"), 20);
        assert_eq!(tracker.daily_task_count("user_a"), 2);
        assert_eq!(tracker.daily_spend_cents("user_b"), 0);
    }

    #[test]
    fn usage_tracker_daily_limit_check() {
        let mut tracker = UsageTracker::new();
        let limit = SpendingLimit::daily("user_a", 500);

        tracker.record_task("user_a", 490);
        assert!(!tracker.would_exceed_daily_limit("user_a", 10, &limit));
        assert!(tracker.would_exceed_daily_limit("user_a", 11, &limit));
    }

    #[test]
    fn usage_tracker_reset_daily() {
        let mut tracker = UsageTracker::new();
        tracker.record_task("user_a", 100);
        assert_eq!(tracker.daily_spend_cents("user_a"), 100);

        tracker.reset_daily();
        assert_eq!(tracker.daily_spend_cents("user_a"), 0);
        assert_eq!(tracker.daily_task_count("user_a"), 0);
    }

    // ── CircuitBreaker ──────────────────────────────────────────

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

    // ── PrepaymentBalance ───────────────────────────────────────

    #[test]
    fn balance_starts_at_zero() {
        let balance = PrepaymentBalance::new("user_a");
        assert_eq!(balance.balance(), 0);
    }

    #[test]
    fn top_up_adds_funds() {
        let mut balance = PrepaymentBalance::new("user_a");
        balance.top_up(1000, 1000).unwrap();
        assert_eq!(balance.balance(), 1000);
    }

    #[test]
    fn top_up_rejects_below_minimum() {
        let mut balance = PrepaymentBalance::new("user_a");
        let result = balance.top_up(500, 1000);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Minimum top-up"));
        assert_eq!(balance.balance(), 0);
    }

    #[test]
    fn debit_task_deducts_cost() {
        let mut balance = PrepaymentBalance::with_balance("user_a", 1000);
        assert!(balance.debit_task(10));
        assert_eq!(balance.balance(), 990);
    }

    #[test]
    fn debit_task_allows_overdraft_within_limit() {
        let mut balance = PrepaymentBalance::with_balance("user_a", 5);
        // $0.05 balance, $0.10 task, $2.00 overdraft limit → should succeed
        assert!(balance.debit_task(10));
        assert_eq!(balance.balance(), -5);
    }

    #[test]
    fn debit_task_blocks_beyond_overdraft() {
        let mut balance = PrepaymentBalance::with_balance("user_a", -190);
        // -$1.90 balance, $0.10 task, $2.00 overdraft limit → $-2.00 exactly OK
        assert!(balance.debit_task(10));
        assert_eq!(balance.balance(), -200);

        // Now at -$2.00, another $0.10 would be -$2.10 > $2.00 overdraft
        assert!(!balance.debit_task(10));
        assert_eq!(balance.balance(), -200);
    }

    #[test]
    fn can_start_task_checks_overdraft_floor() {
        let balance = PrepaymentBalance::with_balance("user_a", 5);
        assert!(balance.can_start_task(10)); // -5 >= -200

        let balance = PrepaymentBalance::with_balance("user_a", -195);
        assert!(!balance.can_start_task(10)); // -205 < -200
    }

    #[test]
    fn balance_display_positive() {
        let balance = PrepaymentBalance::with_balance("user_a", 450);
        assert_eq!(balance.balance_display(), "$4.50");
    }

    #[test]
    fn balance_display_negative() {
        let balance = PrepaymentBalance::with_balance("user_a", -120);
        assert_eq!(balance.balance_display(), "-$1.20");
    }

    #[test]
    fn balance_display_zero() {
        let balance = PrepaymentBalance::new("user_a");
        assert_eq!(balance.balance_display(), "$0.00");
    }

    // ── Warning Levels ──────────────────────────────────────────

    #[test]
    fn warning_level_none_when_healthy() {
        let balance = PrepaymentBalance::with_balance("user_a", 500);
        assert_eq!(balance.warning_level(&[200, 100]), WarningLevel::None);
    }

    #[test]
    fn warning_level_low_at_threshold() {
        let balance = PrepaymentBalance::with_balance("user_a", 200);
        assert_eq!(balance.warning_level(&[200, 100]), WarningLevel::Low);
    }

    #[test]
    fn warning_level_critical_at_threshold() {
        let balance = PrepaymentBalance::with_balance("user_a", 100);
        assert_eq!(balance.warning_level(&[200, 100]), WarningLevel::Critical);
    }

    #[test]
    fn warning_level_critical_when_negative() {
        let balance = PrepaymentBalance::with_balance("user_a", -50);
        assert_eq!(balance.warning_level(&[200, 100]), WarningLevel::Critical);
    }

    // ── format_cents ────────────────────────────────────────────

    #[test]
    fn format_cents_positive() {
        assert_eq!(format_cents(1050), "$10.50");
        assert_eq!(format_cents(10), "$0.10");
        assert_eq!(format_cents(0), "$0.00");
    }

    #[test]
    fn format_cents_negative() {
        assert_eq!(format_cents(-120), "-$1.20");
        assert_eq!(format_cents(-5), "-$0.05");
    }

    // ── Serde ───────────────────────────────────────────────────

    #[test]
    fn prepayment_balance_serde_roundtrip() {
        let balance = PrepaymentBalance::with_balance("user_a", 750);
        let json = serde_json::to_string(&balance).unwrap();
        let parsed: PrepaymentBalance = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.owner_id, "user_a");
        assert_eq!(parsed.balance_cents, 750);
        assert_eq!(parsed.max_overdraft_cents, 200);
    }
}
