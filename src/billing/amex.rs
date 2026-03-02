//! American Express payment provider placeholder.
//!
//! TODO: Implement Amex-specific processing for merchants that require
//! direct Amex integration (separate from Stripe/generic card processing).
//! This is scaffolding only — no real Amex API calls are made.

use super::traits::{
    Currency, PaymentIntent, PaymentIntentRequest, PaymentProvider, PaymentStatus, RefundRequest,
    RefundResult,
};
use async_trait::async_trait;

/// American Express payment provider stub.
///
/// Some merchant configurations require direct Amex integration rather
/// than processing through Stripe. This provider will handle those cases
/// once billing strategy is finalized.
pub struct AmexProvider {
    // TODO: Add Amex API credentials, merchant ID, etc.
}

impl AmexProvider {
    /// Create a new Amex provider instance.
    ///
    /// TODO: Accept config struct with merchant credentials.
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PaymentProvider for AmexProvider {
    fn name(&self) -> &str {
        "amex"
    }

    async fn initialize(&self) -> anyhow::Result<()> {
        // TODO: Validate Amex merchant credentials
        anyhow::bail!("AmexProvider is not yet implemented")
    }

    async fn create_payment_intent(
        &self,
        _request: &PaymentIntentRequest,
    ) -> anyhow::Result<PaymentIntent> {
        // TODO: Create Amex payment authorization
        anyhow::bail!("AmexProvider::create_payment_intent is not yet implemented")
    }

    async fn verify_payment(&self, _payment_id: &str) -> anyhow::Result<PaymentStatus> {
        // TODO: Query Amex transaction status
        anyhow::bail!("AmexProvider::verify_payment is not yet implemented")
    }

    async fn refund(&self, _request: &RefundRequest) -> anyhow::Result<RefundResult> {
        // TODO: Process Amex refund
        anyhow::bail!("AmexProvider::refund is not yet implemented")
    }

    fn supports_currency(&self, currency: &Currency) -> bool {
        // Amex primarily supports major fiat currencies.
        matches!(
            currency,
            Currency::Usd | Currency::Eur | Currency::Gbp | Currency::Jpy
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amex_provider_name() {
        let provider = AmexProvider::new();
        assert_eq!(provider.name(), "amex");
    }

    #[test]
    fn amex_supports_fiat_currencies() {
        let provider = AmexProvider::new();
        assert!(provider.supports_currency(&Currency::Usd));
        assert!(provider.supports_currency(&Currency::Gbp));
        assert!(!provider.supports_currency(&Currency::Btc));
    }

    #[tokio::test]
    async fn amex_initialize_returns_not_implemented() {
        let provider = AmexProvider::new();
        let result = provider.initialize().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet implemented"));
    }
}
