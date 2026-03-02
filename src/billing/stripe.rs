//! Stripe payment provider placeholder.
//!
//! TODO: Implement Stripe API integration for credit card and international
//! payment processing. This is scaffolding only — no real Stripe calls are made.

use super::traits::{
    Currency, PaymentIntent, PaymentIntentRequest, PaymentProvider, PaymentStatus, RefundRequest,
    RefundResult,
};
use async_trait::async_trait;

/// Stripe payment provider stub.
///
/// Will support credit cards, bank transfers, and international payments
/// via the Stripe API once billing strategy is finalized.
pub struct StripeProvider {
    // TODO: Add Stripe API client, secret key handle, webhook config, etc.
}

impl StripeProvider {
    /// Create a new Stripe provider instance.
    ///
    /// TODO: Accept config struct with API keys, webhook secret, etc.
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PaymentProvider for StripeProvider {
    fn name(&self) -> &str {
        "stripe"
    }

    async fn initialize(&self) -> anyhow::Result<()> {
        // TODO: Validate Stripe API key, set up webhook listener
        anyhow::bail!("StripeProvider is not yet implemented")
    }

    async fn create_payment_intent(
        &self,
        _request: &PaymentIntentRequest,
    ) -> anyhow::Result<PaymentIntent> {
        // TODO: Call Stripe PaymentIntents API
        anyhow::bail!("StripeProvider::create_payment_intent is not yet implemented")
    }

    async fn verify_payment(&self, _payment_id: &str) -> anyhow::Result<PaymentStatus> {
        // TODO: Retrieve PaymentIntent from Stripe and map status
        anyhow::bail!("StripeProvider::verify_payment is not yet implemented")
    }

    async fn refund(&self, _request: &RefundRequest) -> anyhow::Result<RefundResult> {
        // TODO: Call Stripe Refunds API
        anyhow::bail!("StripeProvider::refund is not yet implemented")
    }

    fn supports_currency(&self, currency: &Currency) -> bool {
        // Stripe supports most fiat currencies.
        // TODO: Query Stripe for the full supported list based on account country.
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
    fn stripe_provider_name() {
        let provider = StripeProvider::new();
        assert_eq!(provider.name(), "stripe");
    }

    #[test]
    fn stripe_supports_fiat_currencies() {
        let provider = StripeProvider::new();
        assert!(provider.supports_currency(&Currency::Usd));
        assert!(provider.supports_currency(&Currency::Eur));
        assert!(!provider.supports_currency(&Currency::Btc));
    }

    #[tokio::test]
    async fn stripe_initialize_returns_not_implemented() {
        let provider = StripeProvider::new();
        let result = provider.initialize().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet implemented"));
    }
}
