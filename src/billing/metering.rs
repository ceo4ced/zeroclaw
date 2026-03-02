//! Task metering for flat-rate billing.
//!
//! Handles the core billing operation: charging a flat $0.10 per completed
//! task against a user's prepaid balance, enforcing daily limits (soft cap),
//! and emitting low-balance warnings.
//!
//! # Soft Cap Behavior
//!
//! The daily limit is a soft cap. When a task is submitted:
//! 1. Check if daily limit would be exceeded — if so, reject the *new* task.
//! 2. If a task is already in flight when the limit is hit, let it finish
//!    and charge normally (the overdraft on prepaid balance handles this).

use super::spending::{
    CircuitBreaker, PrepaymentBalance, SpendingLimit, UsageTracker, WarningLevel,
};

/// Result of attempting to charge for a task.
#[derive(Debug)]
pub struct ChargeResult {
    /// Whether the charge succeeded.
    pub charged: bool,
    /// Amount charged in cents (0 if not charged).
    pub amount_cents: u32,
    /// New balance after charge (or current balance if not charged).
    pub new_balance_cents: i64,
    /// Formatted balance string for display.
    pub balance_display: String,
    /// Warning level after the charge.
    pub warning: WarningLevel,
    /// If charge was rejected, the reason.
    pub rejection_reason: Option<String>,
}

/// Result of a pre-flight check before starting a task.
#[derive(Debug)]
pub struct PreflightResult {
    /// Whether the task is allowed to start.
    pub allowed: bool,
    /// If not allowed, the reason.
    pub reason: Option<String>,
    /// Current balance display.
    pub balance_display: String,
    /// Current warning level.
    pub warning: WarningLevel,
}

/// Core task meter — coordinates balance, usage tracking, and limits.
///
/// Holds references to the spending control components and applies
/// the flat-rate billing logic.
#[derive(Debug)]
pub struct TaskMeter {
    /// Cost per task in cents.
    task_cost_cents: u32,
    /// Warning thresholds in cents (e.g. [200, 100]).
    warning_thresholds: Vec<u32>,
}

impl TaskMeter {
    /// Create a new task meter with the given per-task cost.
    pub fn new(task_cost_cents: u32, warning_thresholds: Vec<u32>) -> Self {
        Self {
            task_cost_cents,
            warning_thresholds,
        }
    }

    /// Create with MVP defaults ($0.10/task, warnings at $2 and $1).
    pub fn default_mvp() -> Self {
        Self {
            task_cost_cents: 10,
            warning_thresholds: vec![200, 100],
        }
    }

    /// Cost per task in cents.
    pub fn task_cost_cents(&self) -> u32 {
        self.task_cost_cents
    }

    /// Pre-flight check: can a new task be started?
    ///
    /// Checks daily limit (soft cap) and balance (with overdraft).
    /// Does NOT deduct anything — call [`charge_task`] after task completes.
    pub fn preflight_check(
        &self,
        balance: &PrepaymentBalance,
        tracker: &UsageTracker,
        limit: &SpendingLimit,
        breaker: &CircuitBreaker,
    ) -> PreflightResult {
        // Circuit breaker check
        if !breaker.is_allowed() {
            return PreflightResult {
                allowed: false,
                reason: Some("Payment processing is temporarily halted due to repeated failures. Please try again later.".into()),
                balance_display: balance.balance_display(),
                warning: balance.warning_level(&self.warning_thresholds),
            };
        }

        // Daily limit soft cap check
        if tracker.would_exceed_daily_limit(&limit.scope_id, self.task_cost_cents, limit) {
            return PreflightResult {
                allowed: false,
                reason: Some(format!(
                    "Daily spending limit of {} reached ({} spent today). Limit resets at midnight UTC.",
                    super::spending::format_cents(i64::from(limit.max_amount_cents)),
                    super::spending::format_cents(i64::from(tracker.daily_spend_cents(&limit.scope_id))),
                )),
                balance_display: balance.balance_display(),
                warning: balance.warning_level(&self.warning_thresholds),
            };
        }

        // Balance + overdraft check
        if !balance.can_start_task(self.task_cost_cents) {
            return PreflightResult {
                allowed: false,
                reason: Some(format!(
                    "Insufficient balance ({}). Please top up to continue.",
                    balance.balance_display(),
                )),
                balance_display: balance.balance_display(),
                warning: balance.warning_level(&self.warning_thresholds),
            };
        }

        PreflightResult {
            allowed: true,
            reason: None,
            balance_display: balance.balance_display(),
            warning: balance.warning_level(&self.warning_thresholds),
        }
    }

    /// Charge for a completed task.
    ///
    /// Deducts the flat rate from balance and records the charge in the
    /// usage tracker. Should be called after a task finishes (soft cap:
    /// always let the current task complete).
    pub fn charge_task(
        &self,
        balance: &mut PrepaymentBalance,
        tracker: &mut UsageTracker,
    ) -> ChargeResult {
        let scope_id = balance.owner_id.clone();

        if balance.debit_task(self.task_cost_cents) {
            tracker.record_task(&scope_id, self.task_cost_cents);
            let warning = balance.warning_level(&self.warning_thresholds);
            ChargeResult {
                charged: true,
                amount_cents: self.task_cost_cents,
                new_balance_cents: balance.balance(),
                balance_display: balance.balance_display(),
                warning,
                rejection_reason: None,
            }
        } else {
            let warning = balance.warning_level(&self.warning_thresholds);
            ChargeResult {
                charged: false,
                amount_cents: 0,
                new_balance_cents: balance.balance(),
                balance_display: balance.balance_display(),
                warning,
                rejection_reason: Some(format!(
                    "Overdraft limit reached ({}). Please top up to continue.",
                    balance.balance_display(),
                )),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (
        TaskMeter,
        PrepaymentBalance,
        UsageTracker,
        SpendingLimit,
        CircuitBreaker,
    ) {
        let meter = TaskMeter::default_mvp();
        let balance = PrepaymentBalance::with_balance("user_a", 1000); // $10.00
        let tracker = UsageTracker::new();
        let limit = SpendingLimit::daily("user_a", 500); // $5.00/day
        let breaker = CircuitBreaker::new(5);
        (meter, balance, tracker, limit, breaker)
    }

    #[test]
    fn preflight_allows_when_healthy() {
        let (meter, balance, tracker, limit, breaker) = setup();
        let result = meter.preflight_check(&balance, &tracker, &limit, &breaker);
        assert!(result.allowed);
        assert!(result.reason.is_none());
        assert_eq!(result.warning, WarningLevel::None);
    }

    #[test]
    fn preflight_blocks_at_daily_limit() {
        let (meter, balance, mut tracker, limit, breaker) = setup();
        // Spend $5.00 (50 tasks at $0.10)
        for _ in 0..50 {
            tracker.record_task("user_a", 10);
        }
        let result = meter.preflight_check(&balance, &tracker, &limit, &breaker);
        assert!(!result.allowed);
        assert!(result.reason.unwrap().contains("Daily spending limit"));
    }

    #[test]
    fn preflight_blocks_when_balance_exhausted() {
        let (meter, _balance, tracker, limit, breaker) = setup();
        // Balance of -$1.95, task cost $0.10 would go to -$2.05 > $2.00 overdraft
        let low_balance = PrepaymentBalance::with_balance("user_a", -195);
        let result = meter.preflight_check(&low_balance, &tracker, &limit, &breaker);
        assert!(!result.allowed);
        assert!(result.reason.unwrap().contains("Insufficient balance"));
    }

    #[test]
    fn preflight_blocks_when_circuit_breaker_open() {
        let (meter, balance, tracker, limit, mut breaker) = setup();
        for _ in 0..5 {
            breaker.record_failure();
        }
        let result = meter.preflight_check(&balance, &tracker, &limit, &breaker);
        assert!(!result.allowed);
        assert!(result.reason.unwrap().contains("temporarily halted"));
    }

    #[test]
    fn charge_task_deducts_and_records() {
        let (meter, mut balance, mut tracker, _limit, _breaker) = setup();
        let result = meter.charge_task(&mut balance, &mut tracker);
        assert!(result.charged);
        assert_eq!(result.amount_cents, 10);
        assert_eq!(result.new_balance_cents, 990);
        assert_eq!(tracker.daily_task_count("user_a"), 1);
        assert_eq!(tracker.daily_spend_cents("user_a"), 10);
    }

    #[test]
    fn charge_task_triggers_low_warning() {
        let meter = TaskMeter::default_mvp();
        // Balance at $2.00, after charge will be $1.90 — low warning
        let mut balance = PrepaymentBalance::with_balance("user_a", 200);
        let mut tracker = UsageTracker::new();
        let result = meter.charge_task(&mut balance, &mut tracker);
        assert!(result.charged);
        assert_eq!(result.warning, WarningLevel::Low);
    }

    #[test]
    fn charge_task_triggers_critical_warning() {
        let meter = TaskMeter::default_mvp();
        // Balance at $1.00, after charge will be $0.90 — critical warning
        let mut balance = PrepaymentBalance::with_balance("user_a", 100);
        let mut tracker = UsageTracker::new();
        let result = meter.charge_task(&mut balance, &mut tracker);
        assert!(result.charged);
        assert_eq!(result.warning, WarningLevel::Critical);
    }

    #[test]
    fn charge_task_allows_overdraft() {
        let meter = TaskMeter::default_mvp();
        let mut balance = PrepaymentBalance::with_balance("user_a", 5); // $0.05
        let mut tracker = UsageTracker::new();
        let result = meter.charge_task(&mut balance, &mut tracker);
        assert!(result.charged);
        assert_eq!(result.new_balance_cents, -5); // -$0.05
    }

    #[test]
    fn charge_task_rejects_at_overdraft_limit() {
        let meter = TaskMeter::default_mvp();
        let mut balance = PrepaymentBalance::with_balance("user_a", -195); // -$1.95
        let mut tracker = UsageTracker::new();
        let result = meter.charge_task(&mut balance, &mut tracker);
        assert!(!result.charged);
        assert_eq!(result.amount_cents, 0);
        assert!(result.rejection_reason.unwrap().contains("Overdraft limit"));
    }

    #[test]
    fn fifty_tasks_costs_five_dollars() {
        let meter = TaskMeter::default_mvp();
        let mut balance = PrepaymentBalance::with_balance("user_a", 1000); // $10.00
        let mut tracker = UsageTracker::new();

        for _ in 0..50 {
            let result = meter.charge_task(&mut balance, &mut tracker);
            assert!(result.charged);
        }

        assert_eq!(balance.balance(), 500); // $5.00 remaining
        assert_eq!(tracker.daily_spend_cents("user_a"), 500); // $5.00 spent
        assert_eq!(tracker.daily_task_count("user_a"), 50);
    }

    #[test]
    fn default_mvp_values() {
        let meter = TaskMeter::default_mvp();
        assert_eq!(meter.task_cost_cents(), 10);
    }
}
