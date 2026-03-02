//! Cash App payment provider placeholder.
//!
//! TODO: Implement Cash App Pay API integration (via Square). This is
//! scaffolding only — no real Cash App calls are made.

use super::traits::{
    Currency, PaymentIntent, PaymentIntentRequest, PaymentProvider, PaymentStatus, RefundRequest,
    RefundResult,
};
use async_trait::async_trait;

/// Cash App payment provider stub.
///
/// Cash App Pay integration routes through the Square API. This provider
/// will handle Cash App-specific checkout flows once billing strategy is
/// finalized.
pub struct CashAppProvider {
    // TODO: Add Square API credentials, Cash App Pay configuration, etc.
}

impl CashAppProvider {
    /// Create a new Cash App provider instance.
    ///
    /// TODO: Accept config struct with Square/Cash App credentials.
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PaymentProvider for CashAppProvider {
    fn name(&self) -> &str {
        "cashapp"
    }

    async fn initialize(&self) -> anyhow::Result<()> {
        // TODO: Initialize Square client for Cash App Pay
        anyhow::bail!("CashAppProvider is not yet implemented")
    }

    async fn create_payment_intent(
        &self,
        _request: &PaymentIntentRequest,
    ) -> anyhow::Result<PaymentIntent> {
        // TODO: Create Cash App Pay order via Square API
        anyhow::bail!("CashAppProvider::create_payment_intent is not yet implemented")
    }

    async fn verify_payment(&self, _payment_id: &str) -> anyhow::Result<PaymentStatus> {
        // TODO: Check Square payment status
        anyhow::bail!("CashAppProvider::verify_payment is not yet implemented")
    }

    async fn refund(&self, _request: &RefundRequest) -> anyhow::Result<RefundResult> {
        // TODO: Process refund via Square Refunds API
        anyhow::bail!("CashAppProvider::refund is not yet implemented")
    }

    fn supports_currency(&self, currency: &Currency) -> bool {
        // Cash App is US-only (USD) with limited GBP support in the UK.
        matches!(currency, Currency::Usd | Currency::Gbp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cashapp_provider_name() {
        let provider = CashAppProvider::new();
        assert_eq!(provider.name(), "cashapp");
    }

    #[test]
    fn cashapp_supports_usd_and_gbp() {
        let provider = CashAppProvider::new();
        assert!(provider.supports_currency(&Currency::Usd));
        assert!(provider.supports_currency(&Currency::Gbp));
        assert!(!provider.supports_currency(&Currency::Eur));
        assert!(!provider.supports_currency(&Currency::Btc));
    }

    #[tokio::test]
    async fn cashapp_initialize_returns_not_implemented() {
        let provider = CashAppProvider::new();
        let result = provider.initialize().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet implemented"));
    }
}
