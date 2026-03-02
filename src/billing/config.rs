//! Billing configuration schema for MVP.
//!
//! Defines the [`BillingConfig`] struct and related types for the flat-rate
//! task billing model. All amounts are stored in cents (USD minor units).
//!
//! # MVP Billing Model
//!
//! - Flat $0.10 per task for all users on the default tier.
//! - Self-hosted Gemma as the default LLM (no per-token API cost to user).
//! - Paid tier unlocks BYOK (bring your own API key) — no LLM markup.
//! - Prepaid balance with $10 minimum top-up, no refunds.
//! - $5/day default spending limit (soft cap).
//! - Low balance warnings at $2 and $1.
//! - Up to $2 negative overdraft allowed (temporary policy).
//! - Real-time cost display with batched updates.

use serde::{Deserialize, Serialize};

/// Top-level billing configuration.
///
/// TODO: Wire into `Config` in `src/config/schema.rs` once billing
/// integration is ready. All fields have safe defaults so existing
/// configs without a `[billing]` section will keep working.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingConfig {
    /// Whether billing is enabled. Default: `false`.
    #[serde(default)]
    pub enabled: bool,

    /// Default payment provider key for top-ups (e.g. `"stripe"`, `"paypal"`).
    #[serde(default)]
    pub default_provider: Option<String>,

    /// Default currency for charges. Default: `"USD"`.
    #[serde(default = "default_currency")]
    pub default_currency: String,

    /// Per-provider configuration sections (for top-up payment processing).
    #[serde(default)]
    pub providers: BillingProvidersConfig,

    /// Task pricing configuration.
    #[serde(default)]
    pub pricing: PricingConfig,

    /// Spending limits and controls.
    #[serde(default)]
    pub spending: SpendingConfig,

    /// Balance and top-up configuration.
    #[serde(default)]
    pub balance: BalanceConfig,
}

impl Default for BillingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_provider: None,
            default_currency: default_currency(),
            providers: BillingProvidersConfig::default(),
            pricing: PricingConfig::default(),
            spending: SpendingConfig::default(),
            balance: BalanceConfig::default(),
        }
    }
}

fn default_currency() -> String {
    "USD".to_string()
}

// ── User tier ───────────────────────────────────────────────────

/// User billing tier.
///
/// Determines LLM access model and pricing behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserTier {
    /// Default tier: uses platform-hosted Gemma, flat $0.10/task rate.
    Default,
    /// Paid tier: can bring own API key (BYOK), skips LLM markup,
    /// still pays platform fees per task.
    Paid,
}

impl std::fmt::Display for UserTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Default => write!(f, "default"),
            Self::Paid => write!(f, "paid"),
        }
    }
}

// ── Pricing ─────────────────────────────────────────────────────

/// Task pricing configuration.
///
/// MVP uses a flat per-task rate regardless of complexity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingConfig {
    /// Cost per task in cents. Default: 10 ($0.10).
    #[serde(default = "default_task_cost_cents")]
    pub task_cost_cents: u32,
}

impl Default for PricingConfig {
    fn default() -> Self {
        Self {
            task_cost_cents: default_task_cost_cents(),
        }
    }
}

fn default_task_cost_cents() -> u32 {
    10
}

// ── Spending limits ─────────────────────────────────────────────

/// Spending limit and budget enforcement configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingConfig {
    /// Daily spending limit in cents. Default: 500 ($5.00).
    #[serde(default = "default_daily_limit_cents")]
    pub daily_limit_cents: u32,

    /// Whether the circuit breaker auto-halts on repeated payment failures.
    #[serde(default = "default_true")]
    pub circuit_breaker_enabled: bool,

    /// Number of consecutive failures before circuit breaker trips.
    #[serde(default = "default_circuit_breaker_threshold")]
    pub circuit_breaker_threshold: u32,
}

impl Default for SpendingConfig {
    fn default() -> Self {
        Self {
            daily_limit_cents: default_daily_limit_cents(),
            circuit_breaker_enabled: true,
            circuit_breaker_threshold: default_circuit_breaker_threshold(),
        }
    }
}

fn default_daily_limit_cents() -> u32 {
    500
}

fn default_circuit_breaker_threshold() -> u32 {
    5
}

fn default_true() -> bool {
    true
}

// ── Balance ─────────────────────────────────────────────────────

/// Balance and top-up configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceConfig {
    /// Minimum top-up amount in cents. Default: 1000 ($10.00).
    #[serde(default = "default_min_topup_cents")]
    pub min_topup_cents: u32,

    /// Maximum allowed overdraft in cents. Default: 200 ($2.00).
    /// When balance goes this far negative, new tasks are blocked.
    #[serde(default = "default_max_overdraft_cents")]
    pub max_overdraft_cents: u32,

    /// Balance thresholds (in cents) at which low-balance warnings fire.
    /// Default: [200, 100] ($2.00 and $1.00).
    /// Evaluated in order — first match triggers the corresponding alert level.
    #[serde(default = "default_warning_thresholds_cents")]
    pub warning_thresholds_cents: Vec<u32>,

    /// Whether refunds are allowed. Default: `false` (no refunds in MVP).
    #[serde(default)]
    pub refunds_enabled: bool,
}

impl Default for BalanceConfig {
    fn default() -> Self {
        Self {
            min_topup_cents: default_min_topup_cents(),
            max_overdraft_cents: default_max_overdraft_cents(),
            warning_thresholds_cents: default_warning_thresholds_cents(),
            refunds_enabled: false,
        }
    }
}

fn default_min_topup_cents() -> u32 {
    1000
}

fn default_max_overdraft_cents() -> u32 {
    200
}

fn default_warning_thresholds_cents() -> Vec<u32> {
    vec![200, 100]
}

// ── Payment provider configs ────────────────────────────────────

/// Per-provider credential and endpoint configuration.
///
/// Each field is optional; only configured providers will be available
/// at runtime. These are used for top-up payment processing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BillingProvidersConfig {
    #[serde(default)]
    pub stripe: Option<StripeConfig>,
    #[serde(default)]
    pub paypal: Option<PaypalConfig>,
    #[serde(default)]
    pub crypto: Option<CryptoConfig>,
    #[serde(default)]
    pub amex: Option<AmexConfig>,
    #[serde(default)]
    pub venmo: Option<VenmoConfig>,
    #[serde(default)]
    pub cashapp: Option<CashAppConfig>,
}

/// Stripe provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeConfig {
    pub api_key: Option<String>,
    pub webhook_secret: Option<String>,
}

/// PayPal provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaypalConfig {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    #[serde(default = "default_true")]
    pub sandbox: bool,
}

/// Crypto payment configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoConfig {
    pub rpc_url: Option<String>,
    #[serde(default = "default_confirmations")]
    pub confirmations_required: u32,
}

fn default_confirmations() -> u32 {
    6
}

/// Amex provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmexConfig {
    pub merchant_id: Option<String>,
}

/// Venmo provider configuration (via Braintree).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenmoConfig {
    pub merchant_id: Option<String>,
    pub public_key: Option<String>,
    pub private_key: Option<String>,
}

/// Cash App provider configuration (via Square).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashAppConfig {
    pub access_token: Option<String>,
    pub location_id: Option<String>,
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
    fn pricing_defaults_to_ten_cents() {
        let config = PricingConfig::default();
        assert_eq!(config.task_cost_cents, 10);
    }

    #[test]
    fn spending_defaults_to_five_dollars_daily() {
        let config = SpendingConfig::default();
        assert_eq!(config.daily_limit_cents, 500);
    }

    #[test]
    fn balance_defaults_match_mvp_spec() {
        let config = BalanceConfig::default();
        assert_eq!(config.min_topup_cents, 1000);
        assert_eq!(config.max_overdraft_cents, 200);
        assert_eq!(config.warning_thresholds_cents, vec![200, 100]);
        assert!(!config.refunds_enabled);
    }

    #[test]
    fn user_tier_serde_roundtrip() {
        let tier = UserTier::Default;
        let json = serde_json::to_string(&tier).unwrap();
        assert_eq!(json, "\"default\"");
        let parsed: UserTier = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, UserTier::Default);

        let paid = UserTier::Paid;
        let json = serde_json::to_string(&paid).unwrap();
        assert_eq!(json, "\"paid\"");
        let parsed: UserTier = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, UserTier::Paid);
    }

    #[test]
    fn user_tier_display() {
        assert_eq!(UserTier::Default.to_string(), "default");
        assert_eq!(UserTier::Paid.to_string(), "paid");
    }

    #[test]
    fn billing_config_serde_roundtrip() {
        let config = BillingConfig {
            enabled: true,
            default_provider: Some("stripe".into()),
            default_currency: "EUR".into(),
            providers: BillingProvidersConfig::default(),
            pricing: PricingConfig {
                task_cost_cents: 15,
            },
            spending: SpendingConfig::default(),
            balance: BalanceConfig::default(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: BillingConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.default_provider.as_deref(), Some("stripe"));
        assert_eq!(parsed.pricing.task_cost_cents, 15);
    }

    #[test]
    fn billing_config_deserializes_from_empty_object() {
        let config: BillingConfig = serde_json::from_str("{}").unwrap();
        assert!(!config.enabled);
        assert_eq!(config.default_currency, "USD");
        assert_eq!(config.pricing.task_cost_cents, 10);
        assert_eq!(config.spending.daily_limit_cents, 500);
        assert_eq!(config.balance.min_topup_cents, 1000);
    }
}
