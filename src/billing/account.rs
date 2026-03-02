//! User billing account.
//!
//! Ties together the user's tier, prepaid balance, daily usage tracking,
//! spending limits, and circuit breaker into one cohesive account object.
//!
//! This is the primary entry point for billing operations from the agent
//! runtime — call [`UserAccount::start_task`] before a task and
//! [`UserAccount::complete_task`] after.

use super::config::{BalanceConfig, BillingConfig, UserTier};
use super::metering::{ChargeResult, PreflightResult, TaskMeter};
use super::spending::{CircuitBreaker, PrepaymentBalance, SpendingLimit, UsageTracker};

/// A user's billing account.
///
/// Encapsulates all billing state for a single user/tenant.
/// In-memory for MVP — TODO: persist to storage.
#[derive(Debug)]
pub struct UserAccount {
    /// User/tenant identifier.
    pub user_id: String,
    /// User's billing tier.
    pub tier: UserTier,
    /// Whether the user has provided their own API key (BYOK).
    pub has_own_api_key: bool,
    /// Prepaid balance.
    balance: PrepaymentBalance,
    /// Daily usage tracker.
    tracker: UsageTracker,
    /// Daily spending limit.
    limit: SpendingLimit,
    /// Circuit breaker for payment failures.
    breaker: CircuitBreaker,
    /// Task meter for charging.
    meter: TaskMeter,
    /// Balance config (for top-up minimum, etc.).
    balance_config: BalanceConfig,
}

impl UserAccount {
    /// Create a new account from config defaults.
    pub fn new(user_id: impl Into<String>, config: &BillingConfig) -> Self {
        let user_id = user_id.into();
        Self {
            balance: PrepaymentBalance::new(&user_id),
            tracker: UsageTracker::new(),
            limit: SpendingLimit::daily(&user_id, config.spending.daily_limit_cents),
            breaker: CircuitBreaker::new(config.spending.circuit_breaker_threshold),
            meter: TaskMeter::new(
                config.pricing.task_cost_cents,
                config.balance.warning_thresholds_cents.clone(),
            ),
            balance_config: config.balance.clone(),
            user_id,
            tier: UserTier::Default,
            has_own_api_key: false,
        }
    }

    /// Create a paid-tier account with BYOK.
    pub fn new_paid(user_id: impl Into<String>, config: &BillingConfig) -> Self {
        let mut account = Self::new(user_id, config);
        account.tier = UserTier::Paid;
        account.has_own_api_key = true;
        account
    }

    /// Top up the account balance. Enforces minimum top-up amount.
    pub fn top_up(&mut self, amount_cents: u32) -> anyhow::Result<TopUpResult> {
        self.balance
            .top_up(amount_cents, self.balance_config.min_topup_cents)?;
        Ok(TopUpResult {
            new_balance_cents: self.balance.balance(),
            balance_display: self.balance.balance_display(),
        })
    }

    /// Pre-flight check before starting a task.
    ///
    /// Returns whether the task is allowed to start. Does not charge.
    pub fn start_task(&self) -> PreflightResult {
        self.meter
            .preflight_check(&self.balance, &self.tracker, &self.limit, &self.breaker)
    }

    /// Charge for a completed task.
    ///
    /// Should be called after the task finishes (soft cap behavior).
    pub fn complete_task(&mut self) -> ChargeResult {
        self.meter.charge_task(&mut self.balance, &mut self.tracker)
    }

    /// Restore balance from persisted state (e.g. on startup).
    pub fn restore_balance(&mut self, balance_cents: i64) {
        self.balance.balance_cents = balance_cents;
    }

    /// Current balance in cents.
    pub fn balance_cents(&self) -> i64 {
        self.balance.balance()
    }

    /// Formatted balance string.
    pub fn balance_display(&self) -> String {
        self.balance.balance_display()
    }

    /// Number of tasks completed today.
    pub fn tasks_today(&self) -> u32 {
        self.tracker.daily_task_count(&self.user_id)
    }

    /// Amount spent today in cents.
    pub fn spent_today_cents(&self) -> u32 {
        self.tracker.daily_spend_cents(&self.user_id)
    }

    /// Daily spending limit in cents.
    pub fn daily_limit_cents(&self) -> u32 {
        self.limit.max_amount_cents
    }

    /// Update the daily spending limit (user can raise/lower).
    pub fn set_daily_limit(&mut self, limit_cents: u32) {
        self.limit = SpendingLimit::daily(&self.user_id, limit_cents);
    }

    /// Reset daily counters (call at day boundary).
    pub fn reset_daily(&mut self) {
        self.tracker.reset_daily();
    }

    /// Record a payment processing failure (feeds circuit breaker).
    pub fn record_payment_failure(&mut self) {
        self.breaker.record_failure();
    }

    /// Record a payment processing success (resets circuit breaker).
    pub fn record_payment_success(&mut self) {
        self.breaker.record_success();
    }

    /// Whether the user should use platform-hosted Gemma (default tier)
    /// or their own API key (paid tier).
    pub fn uses_platform_llm(&self) -> bool {
        self.tier == UserTier::Default || !self.has_own_api_key
    }

    /// Get a summary of the account state for display.
    pub fn summary(&self) -> AccountSummary {
        let spent = self.tracker.daily_spend_cents(&self.user_id);
        let limit = self.limit.max_amount_cents;
        AccountSummary {
            user_id: self.user_id.clone(),
            tier: self.tier,
            balance_display: self.balance.balance_display(),
            balance_cents: self.balance.balance(),
            tasks_today: self.tracker.daily_task_count(&self.user_id),
            spent_today_cents: spent,
            spent_today_display: super::spending::format_cents(i64::from(spent)),
            daily_limit_cents: limit,
            daily_limit_display: super::spending::format_cents(i64::from(limit)),
            uses_platform_llm: self.uses_platform_llm(),
        }
    }
}

/// Result of a successful top-up.
#[derive(Debug)]
pub struct TopUpResult {
    pub new_balance_cents: i64,
    pub balance_display: String,
}

/// Account summary for display.
#[derive(Debug, Clone)]
pub struct AccountSummary {
    pub user_id: String,
    pub tier: UserTier,
    pub balance_display: String,
    pub balance_cents: i64,
    pub tasks_today: u32,
    pub spent_today_cents: u32,
    pub spent_today_display: String,
    pub daily_limit_cents: u32,
    pub daily_limit_display: String,
    pub uses_platform_llm: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> BillingConfig {
        BillingConfig::default()
    }

    #[test]
    fn new_account_starts_with_zero_balance() {
        let config = test_config();
        let account = UserAccount::new("user_a", &config);
        assert_eq!(account.balance_cents(), 0);
        assert_eq!(account.tier, UserTier::Default);
        assert!(!account.has_own_api_key);
        assert!(account.uses_platform_llm());
    }

    #[test]
    fn paid_account_uses_own_key() {
        let config = test_config();
        let account = UserAccount::new_paid("user_a", &config);
        assert_eq!(account.tier, UserTier::Paid);
        assert!(account.has_own_api_key);
        assert!(!account.uses_platform_llm());
    }

    #[test]
    fn top_up_and_run_tasks() {
        let config = test_config();
        let mut account = UserAccount::new("user_a", &config);

        // Top up $10.00
        let result = account.top_up(1000).unwrap();
        assert_eq!(result.new_balance_cents, 1000);

        // Run 5 tasks ($0.50 total)
        for _ in 0..5 {
            let preflight = account.start_task();
            assert!(preflight.allowed);
            let charge = account.complete_task();
            assert!(charge.charged);
        }

        assert_eq!(account.balance_cents(), 950);
        assert_eq!(account.tasks_today(), 5);
        assert_eq!(account.spent_today_cents(), 50);
    }

    #[test]
    fn top_up_rejects_below_minimum() {
        let config = test_config();
        let mut account = UserAccount::new("user_a", &config);
        let result = account.top_up(500); // $5 < $10 minimum
        assert!(result.is_err());
    }

    #[test]
    fn daily_limit_blocks_new_tasks() {
        let config = test_config();
        let mut account = UserAccount::new("user_a", &config);
        account.top_up(5000).unwrap(); // $50.00

        // Run 50 tasks to hit $5.00 daily limit
        for _ in 0..50 {
            account.complete_task();
        }

        let preflight = account.start_task();
        assert!(!preflight.allowed);
        assert!(preflight.reason.unwrap().contains("Daily spending limit"));
    }

    #[test]
    fn daily_reset_allows_new_tasks() {
        let config = test_config();
        let mut account = UserAccount::new("user_a", &config);
        account.top_up(5000).unwrap();

        // Hit daily limit
        for _ in 0..50 {
            account.complete_task();
        }
        assert!(!account.start_task().allowed);

        // Reset daily counters
        account.reset_daily();
        assert!(account.start_task().allowed);
    }

    #[test]
    fn set_daily_limit() {
        let config = test_config();
        let mut account = UserAccount::new("user_a", &config);
        assert_eq!(account.daily_limit_cents(), 500);

        account.set_daily_limit(2000); // $20/day
        assert_eq!(account.daily_limit_cents(), 2000);
    }

    #[test]
    fn summary_reflects_state() {
        let config = test_config();
        let mut account = UserAccount::new("user_a", &config);
        account.top_up(1000).unwrap();
        account.complete_task();

        let summary = account.summary();
        assert_eq!(summary.user_id, "user_a");
        assert_eq!(summary.tier, UserTier::Default);
        assert_eq!(summary.balance_cents, 990);
        assert_eq!(summary.tasks_today, 1);
        assert!(summary.uses_platform_llm);
    }

    #[test]
    fn circuit_breaker_blocks_tasks() {
        let config = test_config();
        let mut account = UserAccount::new("user_a", &config);
        account.top_up(1000).unwrap();

        // Trip circuit breaker (5 failures)
        for _ in 0..5 {
            account.record_payment_failure();
        }

        let preflight = account.start_task();
        assert!(!preflight.allowed);
        assert!(preflight.reason.unwrap().contains("temporarily halted"));

        // Recovery
        account.record_payment_success();
        assert!(account.start_task().allowed);
    }
}
