use anyhow::{Context, Result};
use solana_sdk::transaction::{Transaction, VersionedTransaction};
use std::sync::Arc;

use crate::config::AppConfig;
use crate::jito::executor::JitoExecutor;
use crate::rpc::client::SolanaRpc;

pub enum BundleIntent {
    RaydiumNewPool { tx: Transaction },
}

pub struct JitoSender {
    executor: Arc<JitoExecutor>,
}

impl JitoSender {
    pub fn new(executor: Arc<JitoExecutor>) -> Self {
        Self { executor }
    }

    pub async fn submit_bundle(
        &self,
        intent: BundleIntent,
        rpc: &SolanaRpc,
        _cfg: &AppConfig,
    ) -> Result<()> {
        match intent {
            BundleIntent::RaydiumNewPool { tx } => {
                let blockhash = rpc
                    .get_latest_blockhash()
                    .await
                    .context("Failed to get blockhash for bundle")?;

                // Convert standard Transaction to VersionedTransaction for Jito
                let versioned_tx = VersionedTransaction::from(tx);

                self.executor
                    .send_bundle(versioned_tx, blockhash)
                    .await
                    .context("Jito executor failed to send bundle")?;
            }
        }

        Ok(())
    }
}
