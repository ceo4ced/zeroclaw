//! Billing subsystem for payment processing and spending controls.
//!
//! This module implements the factory pattern for payment providers. Each
//! provider implements the [`PaymentProvider`] trait defined in [`traits`],
//! and is registered in the factory function [`create_payment_provider`] by
//! its canonical string key (e.g., `"stripe"`, `"paypal"`, `"crypto"`).
//!
//! The subsystem also provides spending controls ([`spending`]) for
//! per-user/per-tenant budget enforcement, circuit breaking, and
//! prepayment balance tracking.
//!
//! # Status
//!
//! This entire module is **placeholder scaffolding**. The billing strategy
//! is still being designed. No provider performs real payment processing.
//! All provider methods return errors indicating they are not yet implemented.
//!
//! # Extension
//!
//! To add a new payment provider:
//! 1. Implement [`PaymentProvider`] in `src/billing/<provider>.rs`.
//! 2. Add the submodule and re-export below.
//! 3. Register the provider key in [`create_payment_provider`].
//! 4. Add provider-specific config to [`config::BillingProvidersConfig`].
//! 5. Add focused tests for factory wiring and error paths.

pub mod amex;
pub mod cashapp;
pub mod config;
pub mod crypto;
pub mod paypal;
pub mod spending;
pub mod stripe;
pub mod traits;
pub mod venmo;

pub use amex::AmexProvider;
pub use cashapp::CashAppProvider;
pub use config::BillingConfig;
pub use crypto::CryptoProvider;
pub use paypal::PaypalProvider;
pub use spending::{CircuitBreaker, PrepaymentBalance, SpendingLimit, UsageTracker};
pub use stripe::StripeProvider;
pub use traits::PaymentProvider;
pub use venmo::VenmoProvider;

/// Create a payment provider by its canonical string key.
///
/// Supported keys:
/// - `"stripe"` — credit cards, international payments (via Stripe)
/// - `"crypto"` — cryptocurrency payments (BTC, ETH, USDC)
/// - `"amex"` — American Express direct processing
/// - `"paypal"` — PayPal checkout
/// - `"venmo"` — Venmo (via Braintree)
/// - `"cashapp"` — Cash App Pay (via Square)
///
/// Returns an error if the key is unrecognized.
///
/// # Example
///
/// ```rust,no_run
/// # use zeroclaw::billing::create_payment_provider;
/// let provider = create_payment_provider("stripe").unwrap();
/// assert_eq!(provider.name(), "stripe");
/// ```
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
}
