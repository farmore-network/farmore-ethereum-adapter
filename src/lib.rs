//! # farmore-adapter-evm
//!
//! The EVM implementation of the Farmore [`ChainAdapter`] neutrality boundary, plus the
//! home-chain contract bindings used by the node and resolver. Built on `alloy`.
//!
//! `EvmAdapter` is the *destination-chain* settlement/verification adapter: it fronts a
//! recipient from the operator's own account and verifies fills to finality. The same
//! provider/bindings power the node's *home-chain* contract calls (bond, assert,
//! finalize). The chain-neutral core (`farmore-core`) depends on none of this.

#![forbid(unsafe_code)]

pub mod bindings;

use std::time::Duration;

use alloy::consensus::Transaction as _;
use alloy::network::EthereumWallet;
use alloy::primitives::{Address, TxHash, U256};
use alloy::providers::{DynProvider, Provider, ProviderBuilder};
use alloy::rpc::types::TransactionRequest;
use alloy::signers::local::PrivateKeySigner;
use anyhow::Context;
use async_trait::async_trait;
use tracing::warn;

use farmore_core::{
    Account, AdapterError, Capabilities, Capable, FillQuery, FillReceipt, FillStatus,
    OperatorSigner, Settler, TransferRequest, Verifier,
};

pub use bindings::{IBondVault, IERC20Faucet, INamespace, ISettlement};

/// The EVM operator signer: the account that fronts recipients. The private key lives in
/// the provider's wallet filler; this type carries only the chain-neutral identity.
#[derive(Debug, Clone, Copy)]
pub struct EvmSigner {
    pub address: Address,
}

impl OperatorSigner for EvmSigner {
    fn account(&self) -> Account {
        self.address.into_word()
    }
}

/// Builds a signing HTTP provider (with recommended nonce/gas/chain-id fillers) plus the
/// operator signer. Used for any account that sends transactions (node operator).
pub fn build_provider(
    rpc_url: &str,
    private_key: &str,
) -> anyhow::Result<(DynProvider, EvmSigner)> {
    let signer: PrivateKeySigner = private_key.parse().context("invalid private key")?;
    let address = signer.address();
    let wallet = EthereumWallet::from(signer);
    let url = rpc_url.parse().context("invalid rpc url")?;
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect_http(url)
        .erased();
    Ok((provider, EvmSigner { address }))
}

/// Builds a read-only HTTP provider (no signer), for the resolver/indexer.
pub fn build_readonly_provider(rpc_url: &str) -> anyhow::Result<DynProvider> {
    let url = rpc_url.parse().context("invalid rpc url")?;
    Ok(ProviderBuilder::new().connect_http(url).erased())
}

/// Retries an async RPC operation with exponential backoff. Production daemons must
/// survive transient RPC hiccups without crashing.
pub async fn with_retry<T, F, Fut>(
    label: &str,
    max_attempts: u32,
    mut op: F,
) -> Result<T, AdapterError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, AdapterError>>,
{
    let mut delay = Duration::from_millis(250);
    let mut last: Option<AdapterError> = None;
    for attempt in 1..=max_attempts {
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                warn!(target: "farmore::evm", label, attempt, error = %e, "rpc attempt failed");
                last = Some(e);
                if attempt < max_attempts {
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(8));
                }
            }
        }
    }
    Err(last.unwrap_or_else(|| AdapterError::Other(format!("{label}: retry exhausted"))))
}

fn rpc_err<E: std::fmt::Display>(e: E) -> AdapterError {
    AdapterError::Rpc(e.to_string())
}

/// The EVM destination-chain adapter.
#[derive(Clone)]
pub struct EvmAdapter {
    provider: DynProvider,
    signer: EvmSigner,
    caps: Capabilities,
}

impl EvmAdapter {
    pub fn new(provider: DynProvider, signer: EvmSigner, caps: Capabilities) -> Self {
        Self {
            provider,
            signer,
            caps,
        }
    }

    pub fn provider(&self) -> &DynProvider {
        &self.provider
    }
}

#[async_trait]
impl Settler for EvmAdapter {
    type Signer = EvmSigner;

    fn signer(&self) -> &Self::Signer {
        &self.signer
    }

    async fn transfer(&self, req: &TransferRequest) -> Result<FillReceipt, AdapterError> {
        let info = self
            .caps
            .assets
            .resolve(&req.asset)
            .ok_or(AdapterError::UnsupportedAsset(req.asset))?
            .clone();
        let to = Address::from_word(req.to);

        let receipt = if info.native {
            let tx = TransactionRequest::default().to(to).value(req.amount);
            let pending = self.provider.send_transaction(tx).await.map_err(rpc_err)?;
            pending.get_receipt().await.map_err(rpc_err)?
        } else {
            let token = IERC20Faucet::new(Address::from_word(info.token), self.provider.clone());
            let pending = token
                .transfer(to, req.amount)
                .send()
                .await
                .map_err(|e| AdapterError::TransferReverted(e.to_string()))?;
            pending.get_receipt().await.map_err(rpc_err)?
        };

        if !receipt.status() {
            return Err(AdapterError::TransferReverted(format!(
                "tx {} reverted",
                receipt.transaction_hash
            )));
        }
        Ok(FillReceipt {
            tx: receipt.transaction_hash,
            block: receipt.block_number.unwrap_or_default(),
        })
    }
}

#[async_trait]
impl Verifier for EvmAdapter {
    async fn verify_fill(&self, q: &FillQuery) -> Result<FillStatus, AdapterError> {
        let info = self
            .caps
            .assets
            .resolve(&q.asset)
            .ok_or(AdapterError::UnsupportedAsset(q.asset))?
            .clone();
        let tx: TxHash = q.tx;
        let receipt = self
            .provider
            .get_transaction_receipt(tx)
            .await
            .map_err(rpc_err)?;
        let Some(receipt) = receipt else {
            return Ok(FillStatus::Unknown);
        };
        if !receipt.status() {
            return Ok(FillStatus::Mismatch);
        }

        let to = Address::from_word(q.to);
        let mut matched = false;

        if info.native {
            // Verify the native value transfer directly from the transaction.
            if let Some(t) = self
                .provider
                .get_transaction_by_hash(tx)
                .await
                .map_err(rpc_err)?
            {
                matched = t.to() == Some(to) && t.value() == q.amount;
            }
        } else {
            let token = Address::from_word(info.token);
            for log in receipt.inner.logs() {
                if log.address() != token {
                    continue;
                }
                if let Ok(decoded) = log.log_decode::<IERC20Faucet::Transfer>() {
                    let d = decoded.inner.data;
                    if d.to == to && d.value == q.amount {
                        matched = true;
                        break;
                    }
                }
            }
        }

        if !matched {
            return Ok(FillStatus::Mismatch);
        }

        let block = receipt.block_number.unwrap_or_default();
        let head = self.provider.get_block_number().await.map_err(rpc_err)?;
        let confirmations = head.saturating_sub(block) + 1;
        Ok(if self.caps.finality.is_final(confirmations) {
            FillStatus::Final
        } else {
            FillStatus::Seen { confirmations }
        })
    }

    async fn finalized_head(&self) -> Result<u64, AdapterError> {
        self.provider.get_block_number().await.map_err(rpc_err)
    }
}

impl Capable for EvmAdapter {
    fn capabilities(&self) -> &Capabilities {
        &self.caps
    }
}

/// Convenience: U256 with the given whole-token amount scaled by `decimals`.
pub fn scale(whole: u64, decimals: u8) -> U256 {
    U256::from(whole) * U256::from(10u64).pow(U256::from(decimals))
}
