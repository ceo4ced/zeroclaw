//! PayPal payment provider placeholder.
//!
//! TODO: Implement PayPal Orders/Checkout API integration. This is
//! scaffolding only — no real PayPal calls are made.

use super::traits::{
    Currency, PaymentIntent, PaymentIntentRequest, PaymentProvider, PaymentStatus, RefundRequest,
    RefundResult,
};
use async_trait::async_trait;

/// PayPal payment provider stub.
///
/// Will support PayPal checkout flows, buyer/seller protection, and
/// subscription billing once billing strategy is finalized.
pub struct PaypalProvider {
    // TODO: Add PayPal client ID, secret, sandbox/live mode flag, etc.
}

impl PaypalProvider {
    /// Create a new PayPal provider instance.
    ///
    /// TODO: Accept config struct with OAuth credentials and environment.
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PaymentProvider for PaypalProvider {
    fn name(&self) -> &str {
        "paypal"
    }

    async fn initialize(&self) -> anyhow::Result<()> {
        // TODO: Obtain PayPal OAuth access token, validate credentials
        anyhow::bail!("PaypalProvider is not yet implemented")
    }

    async fn create_payment_intent(
        &self,
        _request: &PaymentIntentRequest,
    ) -> anyhow::Result<PaymentIntent> {
        // TODO: Create PayPal Order, return approval URL as checkout_url
        anyhow::bail!("PaypalProvider::create_payment_intent is not yet implemented")
    }

    async fn verify_payment(&self, _payment_id: &str) -> anyhow::Result<PaymentStatus> {
        // TODO: Capture and verify PayPal Order status
        anyhow::bail!("PaypalProvider::verify_payment is not yet implemented")
    }

    async fn refund(&self, _request: &RefundRequest) -> anyhow::Result<RefundResult> {
        // TODO: Issue PayPal refund via Captures API
        anyhow::bail!("PaypalProvider::refund is not yet implemented")
    }

    fn supports_currency(&self, currency: &Currency) -> bool {
        // PayPal supports most fiat currencies.
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
    fn paypal_provider_name() {
        let provider = PaypalProvider::new();
        assert_eq!(provider.name(), "paypal");
    }

    #[test]
    fn paypal_supports_fiat_currencies() {
        let provider = PaypalProvider::new();
        assert!(provider.supports_currency(&Currency::Usd));
        assert!(provider.supports_currency(&Currency::Eur));
        assert!(!provider.supports_currency(&Currency::Btc));
    }

    #[tokio::test]
    async fn paypal_initialize_returns_not_implemented() {
        let provider = PaypalProvider::new();
        let result = provider.initialize().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet implemented"));
    }
}
