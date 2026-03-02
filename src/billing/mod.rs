//! Billing subsystem for task metering, prepaid balance, and payment processing.
//!
//! # MVP Billing Model
//!
//! - Flat $0.10 per task, charged after task completion (soft cap).
//! - Self-hosted Gemma as the default LLM; paid tier unlocks BYOK.
//! - Prepaid balance: $10 minimum top-up, no refunds.
//! - $5/day default spending limit (soft cap — finishes current task).
//! - Low-balance warnings at $2 and $1.
//! - Up to $2 negative overdraft allowed.
//! - Real-time cost display with batched updates.
//! - Dollar amounts shown everywhere — no credits abstraction.
//!
//! # Architecture
//!
//! - [`account::UserAccount`] — primary entry point for billing operations.
//! - [`metering::TaskMeter`] — flat-rate charging logic.
//! - [`spending`] — balance, limits, usage tracking, circuit breaker.
//! - [`display::CostDisplay`] — batched real-time cost UI updates.
//! - [`config`] — billing configuration schema.
//! - [`traits::PaymentProvider`] — trait for top-up payment processors.
//! - Provider stubs (stripe, paypal, crypto, amex, venmo, cashapp) —
//!   top-up payment processing (not yet implemented).
//!
//! # Usage
//!
//! ```rust,no_run
//! use zeroclaw::billing::{UserAccount, BillingConfig, CostDisplay};
//!
//! let config = BillingConfig::default();
//! let mut account = UserAccount::new("user_a", &config);
//! let mut display = CostDisplay::new();
//!
//! // Top up $10.00
//! account.top_up(1000).unwrap();
//!
//! // Before each task
//! let preflight = account.start_task();
//! if preflight.allowed {
//!     // ... run the task ...
//!
//!     // After task completes
//!     let charge = account.complete_task();
//!     if charge.charged {
//!         display.record(charge.amount_cents);
//!     }
//! }
//!
//! // Periodically flush display updates
//! if let Some(update) = display.flush(account.balance_cents(), /* warning */ zeroclaw::billing::WarningLevel::None) {
//!     println!("Balance: {} | Session: {} tasks ({})", update.balance, update.session_tasks, update.session_cost);
//! }
//! ```
//!
//! # Extension
//!
//! To add a new payment provider (for top-ups):
//! 1. Implement [`PaymentProvider`] in `src/billing/<provider>.rs`.
//! 2. Add the submodule and re-export below.
//! 3. Register the provider key in [`create_payment_provider`].
//! 4. Add provider-specific config to [`config::BillingProvidersConfig`].

pub mod account;
pub mod amex;
pub mod cashapp;
pub mod config;
pub mod crypto;
pub mod display;
pub mod metering;
pub mod paypal;
pub mod spending;
pub mod stripe;
pub mod traits;
pub mod venmo;

pub use account::{AccountSummary, UserAccount};
pub use amex::AmexProvider;
pub use cashapp::CashAppProvider;
pub use config::{BillingConfig, UserTier};
pub use crypto::CryptoProvider;
pub use display::{CostDisplay, DisplayUpdate};
pub use metering::{ChargeResult, PreflightResult, TaskMeter};
pub use paypal::PaypalProvider;
pub use spending::{
    format_cents, CircuitBreaker, CircuitState, PrepaymentBalance, SpendingLimit, UsageTracker,
    WarningLevel,
};
pub use stripe::StripeProvider;
pub use traits::PaymentProvider;
pub use venmo::VenmoProvider;

/// Create a payment provider by its canonical string key.
///
/// These are top-up payment processors — used when a user adds funds
/// to their prepaid balance.
///
/// Supported keys: `"stripe"`, `"crypto"`, `"amex"`, `"paypal"`,
/// `"venmo"`, `"cashapp"`.
pub fn create_payment_provider(key: &str) -> anyhow::Result<Box<dyn PaymentProvider>> {
    match key {
        "stripe" => Ok(Box::new(StripeProvider::new())),
        "crypto" => Ok(Box::new(CryptoProvider::new())),
        "amex" => Ok(Box::new(AmexProvider::new())),
        "paypal" => Ok(Box::new(PaypalProvider::new())),
        "venmo" => Ok(Box::new(VenmoProvider::new())),
        "cashapp" => Ok(Box::new(CashAppProvider::new())),
        other => anyhow::bail!(
            "Unknown payment provider: \"{other}\". Supported: stripe, crypto, amex, paypal, venmo, cashapp"
        ),
    }
}

/// List all registered payment provider keys.
pub fn registered_provider_keys() -> &'static [&'static str] {
    &["stripe", "crypto", "amex", "paypal", "venmo", "cashapp"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_creates_all_registered_providers() {
        for key in registered_provider_keys() {
            let provider = create_payment_provider(key)
                .unwrap_or_else(|e| panic!("Failed to create provider \"{key}\": {e}"));
            assert_eq!(provider.name(), *key);
        }
    }

    #[test]
    fn factory_rejects_unknown_key() {
        let result = create_payment_provider("unknown_provider");
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("Unknown payment provider"));
        assert!(err.contains("unknown_provider"));
    }

    #[test]
    fn factory_error_lists_supported_keys() {
        let err = create_payment_provider("nope").err().unwrap().to_string();
        for key in registered_provider_keys() {
            assert!(
                err.contains(key),
                "Error should mention supported key \"{key}\""
            );
        }
    }

    #[test]
    fn registered_keys_are_non_empty() {
        assert!(!registered_provider_keys().is_empty());
    }

    #[test]
    fn registered_keys_are_lowercase() {
        for key in registered_provider_keys() {
            assert_eq!(
                *key,
                key.to_lowercase(),
                "Factory key \"{key}\" must be lowercase"
            );
        }
    }

    // ── Integration test: full billing flow ─────────────────────

    #[test]
    fn full_billing_flow_mvp() {
        let config = BillingConfig::default();
        let mut account = UserAccount::new("user_a", &config);
        let mut display = CostDisplay::new();

        // New account starts at $0 (but $2 overdraft allows some tasks)
        assert_eq!(account.balance_cents(), 0);

        // Top up $10.00
        account.top_up(1000).unwrap();
        assert_eq!(account.balance_cents(), 1000);

        // Run 10 tasks
        for i in 0..10 {
            let preflight = account.start_task();
            assert!(preflight.allowed, "Task {i} should be allowed");

            let charge = account.complete_task();
            assert!(charge.charged, "Task {i} should charge successfully");
            display.record(charge.amount_cents);
        }

        // $10.00 - 10 * $0.10 = $9.00
        assert_eq!(account.balance_cents(), 900);
        assert_eq!(account.tasks_today(), 10);
        assert_eq!(account.spent_today_cents(), 100);

        // Flush display
        let update = display.flush(account.balance_cents(), WarningLevel::None);
        assert!(update.is_some());
        let update = update.unwrap();
        assert_eq!(update.session_tasks, 10);
        assert_eq!(update.session_cost_cents, 100);
        assert_eq!(update.balance, "$9.00");
    }
}
