//! Payment provider trait definition.
//!
//! Defines the [`PaymentProvider`] trait that all payment integrations must
//! implement. Follows the same trait-driven architecture used by
//! [`Provider`](crate::providers::Provider), [`Channel`](crate::channels::Channel),
//! and [`Tool`](crate::tools::Tool).
//!
//! # Status
//!
//! This module is **placeholder scaffolding**. The billing strategy is still
//! being designed; no provider implements real payment processing yet.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── Domain types ─────────────────────────────────────────────────

/// Supported currency codes (ISO 4217 subset + crypto tickers).
///
/// TODO: Expand as providers are implemented. Keep the enum closed so
/// unsupported currencies fail fast at compile time.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Currency {
    Usd,
    Eur,
    Gbp,
    Jpy,
    /// Bitcoin
    Btc,
    /// Ethereum
    Eth,
    /// USDC stablecoin
    Usdc,
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usd => write!(f, "USD"),
            Self::Eur => write!(f, "EUR"),
            Self::Gbp => write!(f, "GBP"),
            Self::Jpy => write!(f, "JPY"),
            Self::Btc => write!(f, "BTC"),
            Self::Eth => write!(f, "ETH"),
            Self::Usdc => write!(f, "USDC"),
        }
    }
}

/// Amount with currency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Money {
    /// Amount in the smallest denomination (e.g. cents for USD, satoshi for BTC).
    pub amount_minor: u64,
    pub currency: Currency,
}

/// Request to create a payment intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIntentRequest {
    /// Amount to charge.
    pub amount: Money,
    /// Opaque tenant/user identifier for the payer.
    pub payer_id: String,
    /// Free-form description shown on the invoice/receipt.
    pub description: Option<String>,
    /// Idempotency key to prevent duplicate charges.
    pub idempotency_key: Option<String>,
}

/// Result of a successfully created payment intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIntent {
    /// Provider-assigned intent/transaction ID.
    pub id: String,
    /// Current status string (provider-specific, e.g. "requires_payment_method").
    pub status: String,
    /// Optional URL the payer should be redirected to for checkout.
    pub checkout_url: Option<String>,
}

/// Possible states returned by [`PaymentProvider::verify_payment`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    Pending,
    Succeeded,
    Failed,
    Refunded,
    /// Provider returned an unrecognized status string.
    Unknown(String),
}

/// Request to issue a refund.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundRequest {
    /// The original payment intent/transaction ID.
    pub payment_id: String,
    /// Amount to refund. If `None`, refund the full amount.
    pub amount: Option<Money>,
    /// Reason for the refund.
    pub reason: Option<String>,
}

/// Result of a successfully issued refund.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResult {
    /// Provider-assigned refund ID.
    pub id: String,
    /// Refund status.
    pub status: String,
}

// ── Trait ─────────────────────────────────────────────────────────

/// Core payment provider trait.
///
/// Implement this for each payment backend (Stripe, PayPal, crypto, etc.).
/// All methods are async and return `anyhow::Result` to keep error handling
/// consistent with the rest of the codebase.
///
/// # Extension
///
/// To add a new payment provider:
/// 1. Implement `PaymentProvider` in `src/billing/<provider>.rs`.
/// 2. Register the provider key in [`super::create_payment_provider`].
/// 3. Add focused tests for factory wiring and error paths.
#[async_trait]
pub trait PaymentProvider: Send + Sync {
    /// Canonical lowercase provider name used as factory key (e.g. `"stripe"`).
    fn name(&self) -> &str;

    /// One-time initialization: establish connections, validate credentials, etc.
    ///
    /// Called once during startup. Implementations should fail fast if
    /// configuration is missing or credentials are invalid.
    async fn initialize(&self) -> anyhow::Result<()>;

    /// Create a payment intent (authorize or begin a charge flow).
    async fn create_payment_intent(
        &self,
        request: &PaymentIntentRequest,
    ) -> anyhow::Result<PaymentIntent>;

    /// Query the current status of a previously created payment.
    async fn verify_payment(&self, payment_id: &str) -> anyhow::Result<PaymentStatus>;

    /// Issue a full or partial refund.
    async fn refund(&self, request: &RefundRequest) -> anyhow::Result<RefundResult>;

    /// Whether this provider supports the given currency.
    fn supports_currency(&self, currency: &Currency) -> bool;

    /// Optional health check (e.g. ping the provider API).
    /// Default returns `true` (healthy).
    async fn health_check(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn currency_display_outputs_iso_codes() {
        assert_eq!(Currency::Usd.to_string(), "USD");
        assert_eq!(Currency::Eur.to_string(), "EUR");
        assert_eq!(Currency::Btc.to_string(), "BTC");
        assert_eq!(Currency::Usdc.to_string(), "USDC");
    }

    #[test]
    fn currency_serde_roundtrip() {
        let usd = serde_json::to_string(&Currency::Usd).unwrap();
        assert_eq!(usd, "\"USD\"");
        let parsed: Currency = serde_json::from_str(&usd).unwrap();
        assert_eq!(parsed, Currency::Usd);
    }

    #[test]
    fn money_serde_roundtrip() {
        let money = Money {
            amount_minor: 1999,
            currency: Currency::Usd,
        };
        let json = serde_json::to_string(&money).unwrap();
        let parsed: Money = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.amount_minor, 1999);
        assert_eq!(parsed.currency, Currency::Usd);
    }

    #[test]
    fn payment_status_variants() {
        assert_eq!(PaymentStatus::Pending, PaymentStatus::Pending);
        assert_ne!(PaymentStatus::Succeeded, PaymentStatus::Failed);
        assert_eq!(
            PaymentStatus::Unknown("custom".into()),
            PaymentStatus::Unknown("custom".into())
        );
    }

    #[test]
    fn payment_intent_request_serde_roundtrip() {
        let req = PaymentIntentRequest {
            amount: Money {
                amount_minor: 5000,
                currency: Currency::Usd,
            },
            payer_id: "tenant_001".into(),
            description: Some("API usage".into()),
            idempotency_key: Some("idem-abc-123".into()),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: PaymentIntentRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.payer_id, "tenant_001");
        assert_eq!(parsed.amount.amount_minor, 5000);
    }

    #[test]
    fn refund_request_partial_amount() {
        let req = RefundRequest {
            payment_id: "pi_test_123".into(),
            amount: Some(Money {
                amount_minor: 500,
                currency: Currency::Usd,
            }),
            reason: Some("Customer request".into()),
        };
        assert!(req.amount.is_some());
    }

    #[test]
    fn refund_request_full_amount() {
        let req = RefundRequest {
            payment_id: "pi_test_456".into(),
            amount: None,
            reason: None,
        };
        assert!(req.amount.is_none());
    }
}
