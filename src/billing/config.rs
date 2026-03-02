//! Billing configuration schema placeholder.
//!
//! Defines the [`BillingConfig`] struct that will eventually be added to the
//! main [`Config`](crate::config::Config) under a `[billing]` section.
//!
//! # Status
//!
//! This is **placeholder scaffolding**. The struct is not yet wired into the
//! main config schema. When the billing strategy is finalized:
//! 1. Add `pub billing: BillingConfig` to `Config` in `src/config/schema.rs`.
//! 2. Ensure serde defaults work for backward compatibility.
//! 3. Update `docs/config-reference.md` with the new keys.

use serde::{Deserialize, Serialize};

/// Top-level billing configuration.
///
/// TODO: Wire into `Config` in `src/config/schema.rs` once billing
/// strategy is finalized. All fields have safe defaults so existing
/// configs without a `[billing]` section will keep working.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingConfig {
    /// Whether billing is enabled. Default: `false`.
    #[serde(default)]
    pub enabled: bool,

    /// Default payment provider key (e.g. `"stripe"`, `"paypal"`).
    /// Must match a key registered in the payment provider factory.
    #[serde(default)]
    pub default_provider: Option<String>,

    /// Default currency for charges. Default: `"USD"`.
    #[serde(default = "default_currency")]
    pub default_currency: String,

    /// Per-provider configuration sections.
    #[serde(default)]
    pub providers: BillingProvidersConfig,

    /// Spending limits and controls.
    #[serde(default)]
    pub spending: SpendingConfig,
}

impl Default for BillingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_provider: None,
            default_currency: default_currency(),
            providers: BillingProvidersConfig::default(),
            spending: SpendingConfig::default(),
        }
    }
}

fn default_currency() -> String {
    "USD".to_string()
}

/// Per-provider credential and endpoint configuration.
///
/// Each field is optional; only configured providers will be available
/// at runtime.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BillingProvidersConfig {
    /// Stripe-specific configuration.
    #[serde(default)]
    pub stripe: Option<StripeConfig>,

    /// PayPal-specific configuration.
    #[serde(default)]
    pub paypal: Option<PaypalConfig>,

    /// Crypto payment configuration.
    #[serde(default)]
    pub crypto: Option<CryptoConfig>,

    /// Amex-specific configuration.
    #[serde(default)]
    pub amex: Option<AmexConfig>,

    /// Venmo-specific configuration (via Braintree).
    #[serde(default)]
    pub venmo: Option<VenmoConfig>,

    /// Cash App-specific configuration (via Square).
    #[serde(default)]
    pub cashapp: Option<CashAppConfig>,
}

/// Stripe provider configuration.
///
/// TODO: Finalize required fields once Stripe integration is designed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeConfig {
    /// Stripe secret key. Should be injected via env var or secrets store.
    /// TODO: Use the ZeroClaw secrets infrastructure instead of raw strings.
    pub api_key: Option<String>,
    /// Webhook signing secret for verifying Stripe events.
    pub webhook_secret: Option<String>,
}

/// PayPal provider configuration.
///
/// TODO: Finalize required fields once PayPal integration is designed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaypalConfig {
    /// PayPal OAuth client ID.
    pub client_id: Option<String>,
    /// PayPal OAuth client secret.
    pub client_secret: Option<String>,
    /// Use sandbox environment. Default: `true`.
    #[serde(default = "default_true")]
    pub sandbox: bool,
}

/// Crypto payment configuration.
///
/// TODO: Finalize required fields once crypto integration is designed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoConfig {
    /// JSON-RPC endpoint for the target chain.
    pub rpc_url: Option<String>,
    /// Number of confirmations required before marking payment as succeeded.
    #[serde(default = "default_confirmations")]
    pub confirmations_required: u32,
}

/// Amex provider configuration.
///
/// TODO: Finalize required fields once Amex integration is designed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmexConfig {
    /// Amex merchant identifier.
    pub merchant_id: Option<String>,
}

/// Venmo provider configuration (via Braintree).
///
/// TODO: Finalize required fields once Venmo integration is designed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenmoConfig {
    /// Braintree merchant ID.
    pub merchant_id: Option<String>,
    /// Braintree public key.
    pub public_key: Option<String>,
    /// Braintree private key.
    pub private_key: Option<String>,
}

/// Cash App provider configuration (via Square).
///
/// TODO: Finalize required fields once Cash App integration is designed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashAppConfig {
    /// Square access token.
    pub access_token: Option<String>,
    /// Square location ID.
    pub location_id: Option<String>,
}

/// Spending limit and budget enforcement configuration.
///
/// TODO: Wire into spending controls module once strategy is finalized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingConfig {
    /// Per-transaction maximum in minor units of default currency.
    #[serde(default)]
    pub max_transaction_amount_minor: Option<u64>,

    /// Daily spending limit in minor units of default currency.
    #[serde(default)]
    pub daily_limit_minor: Option<u64>,

    /// Monthly spending limit in minor units of default currency.
    #[serde(default)]
    pub monthly_limit_minor: Option<u64>,

    /// Whether the circuit breaker auto-halts on repeated failures.
    #[serde(default = "default_true")]
    pub circuit_breaker_enabled: bool,

    /// Number of consecutive failures before circuit breaker trips.
    #[serde(default = "default_circuit_breaker_threshold")]
    pub circuit_breaker_threshold: u32,
}

impl Default for SpendingConfig {
    fn default() -> Self {
        Self {
            max_transaction_amount_minor: None,
            daily_limit_minor: None,
            monthly_limit_minor: None,
            circuit_breaker_enabled: true,
            circuit_breaker_threshold: default_circuit_breaker_threshold(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_confirmations() -> u32 {
    6
}

fn default_circuit_breaker_threshold() -> u32 {
    5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn billing_config_default_is_disabled() {
        let config = BillingConfig::default();
        assert!(!config.enabled);
        assert!(config.default_provider.is_none());
        assert_eq!(config.default_currency, "USD");
    }

    #[test]
    fn billing_config_serde_roundtrip() {
        let config = BillingConfig {
            enabled: true,
            default_provider: Some("stripe".into()),
            default_currency: "EUR".into(),
            providers: BillingProvidersConfig::default(),
            spending: SpendingConfig::default(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: BillingConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.default_provider.as_deref(), Some("stripe"));
        assert_eq!(parsed.default_currency, "EUR");
    }

    #[test]
    fn spending_config_defaults_are_safe() {
        let config = SpendingConfig::default();
        assert!(config.max_transaction_amount_minor.is_none());
        assert!(config.daily_limit_minor.is_none());
        assert!(config.monthly_limit_minor.is_none());
        assert!(config.circuit_breaker_enabled);
        assert_eq!(config.circuit_breaker_threshold, 5);
    }

    #[test]
    fn billing_config_deserializes_from_empty_object() {
        let config: BillingConfig = serde_json::from_str("{}").unwrap();
        assert!(!config.enabled);
        assert_eq!(config.default_currency, "USD");
    }
}
