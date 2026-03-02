//! Venmo payment provider placeholder.
//!
//! TODO: Implement Venmo Pay API integration (via PayPal/Braintree).
//! This is scaffolding only — no real Venmo calls are made.

use super::traits::{
    Currency, PaymentIntent, PaymentIntentRequest, PaymentProvider, PaymentStatus, RefundRequest,
    RefundResult,
};
use async_trait::async_trait;

/// Venmo payment provider stub.
///
/// Venmo integration typically routes through PayPal/Braintree APIs.
/// This provider will handle Venmo-specific checkout flows once billing
/// strategy is finalized.
pub struct VenmoProvider {
    // TODO: Add Braintree credentials, Venmo merchant profile, etc.
}

impl VenmoProvider {
    /// Create a new Venmo provider instance.
    ///
    /// TODO: Accept config struct with Braintree/Venmo credentials.
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PaymentProvider for VenmoProvider {
    fn name(&self) -> &str {
        "venmo"
    }

    async fn initialize(&self) -> anyhow::Result<()> {
        // TODO: Initialize Braintree client for Venmo payments
        anyhow::bail!("VenmoProvider is not yet implemented")
    }

    async fn create_payment_intent(
        &self,
        _request: &PaymentIntentRequest,
    ) -> anyhow::Result<PaymentIntent> {
        // TODO: Create Venmo payment via Braintree, return deeplink
        anyhow::bail!("VenmoProvider::create_payment_intent is not yet implemented")
    }

    async fn verify_payment(&self, _payment_id: &str) -> anyhow::Result<PaymentStatus> {
        // TODO: Check Braintree transaction status for Venmo payment
        anyhow::bail!("VenmoProvider::verify_payment is not yet implemented")
    }

    async fn refund(&self, _request: &RefundRequest) -> anyhow::Result<RefundResult> {
        // TODO: Process Venmo refund via Braintree
        anyhow::bail!("VenmoProvider::refund is not yet implemented")
    }

    fn supports_currency(&self, currency: &Currency) -> bool {
        // Venmo is US-only.
        matches!(currency, Currency::Usd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn venmo_provider_name() {
        let provider = VenmoProvider::new();
        assert_eq!(provider.name(), "venmo");
    }

    #[test]
    fn venmo_supports_usd_only() {
        let provider = VenmoProvider::new();
        assert!(provider.supports_currency(&Currency::Usd));
        assert!(!provider.supports_currency(&Currency::Eur));
        assert!(!provider.supports_currency(&Currency::Btc));
    }

    #[tokio::test]
    async fn venmo_initialize_returns_not_implemented() {
        let provider = VenmoProvider::new();
        let result = provider.initialize().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet implemented"));
    }
}
