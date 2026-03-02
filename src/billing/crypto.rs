//! Cryptocurrency payment provider placeholder.
//!
//! TODO: Implement on-chain payment verification for BTC, ETH, USDC, and
//! other supported tokens. This is scaffolding only — no real chain calls
//! are made.

use super::traits::{
    Currency, PaymentIntent, PaymentIntentRequest, PaymentProvider, PaymentStatus, RefundRequest,
    RefundResult,
};
use async_trait::async_trait;

/// Cryptocurrency payment provider stub.
///
/// Will support on-chain payments (BTC, ETH, USDC) via wallet address
/// generation and transaction verification once billing strategy is finalized.
pub struct CryptoProvider {
    // TODO: Add chain RPC endpoints, wallet derivation config, confirmation
    // thresholds, etc.
}

impl CryptoProvider {
    /// Create a new crypto provider instance.
    ///
    /// TODO: Accept config struct with RPC URLs, HD wallet seed, network, etc.
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl PaymentProvider for CryptoProvider {
    fn name(&self) -> &str {
        "crypto"
    }

    async fn initialize(&self) -> anyhow::Result<()> {
        // TODO: Connect to chain RPC, derive payment addresses
        anyhow::bail!("CryptoProvider is not yet implemented")
    }

    async fn create_payment_intent(
        &self,
        _request: &PaymentIntentRequest,
    ) -> anyhow::Result<PaymentIntent> {
        // TODO: Generate a unique deposit address and return it as checkout_url
        anyhow::bail!("CryptoProvider::create_payment_intent is not yet implemented")
    }

    async fn verify_payment(&self, _payment_id: &str) -> anyhow::Result<PaymentStatus> {
        // TODO: Check on-chain confirmations for the deposit address
        anyhow::bail!("CryptoProvider::verify_payment is not yet implemented")
    }

    async fn refund(&self, _request: &RefundRequest) -> anyhow::Result<RefundResult> {
        // TODO: Initiate on-chain refund transaction
        anyhow::bail!("CryptoProvider::refund is not yet implemented")
    }

    fn supports_currency(&self, currency: &Currency) -> bool {
        matches!(currency, Currency::Btc | Currency::Eth | Currency::Usdc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crypto_provider_name() {
        let provider = CryptoProvider::new();
        assert_eq!(provider.name(), "crypto");
    }

    #[test]
    fn crypto_supports_only_crypto_currencies() {
        let provider = CryptoProvider::new();
        assert!(provider.supports_currency(&Currency::Btc));
        assert!(provider.supports_currency(&Currency::Eth));
        assert!(provider.supports_currency(&Currency::Usdc));
        assert!(!provider.supports_currency(&Currency::Usd));
        assert!(!provider.supports_currency(&Currency::Eur));
    }

    #[tokio::test]
    async fn crypto_initialize_returns_not_implemented() {
        let provider = CryptoProvider::new();
        let result = provider.initialize().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet implemented"));
    }
}
