use anyhow::Result;
use log::{debug, error, warn};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, hash::Hash};
use std::sync::Arc;
use tokio::{
    sync::RwLock,
    time::{sleep, Duration},
};
use tokio_util::sync::CancellationToken;

pub struct SolanaRpc {
    client: RpcClient,
    latest_blockhash: Arc<RwLock<Hash>>,
}

impl SolanaRpc {
    pub fn new(rpc_url: &str) -> Result<Self> {
        let client =
            RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

        Ok(Self {
            client,
            latest_blockhash: Arc::new(RwLock::new(Hash::default())),
        })
    }

    #[allow(dead_code)]
    pub fn client(&self) -> &RpcClient {
        &self.client
    }

    #[allow(dead_code)]
    pub fn latest_blockhash(&self) -> Arc<RwLock<Hash>> {
        self.latest_blockhash.clone()
    }

    /// Background task:
    /// Continuously refreshes the recent blockhash for fast transaction signing.
    pub async fn run_blockhash_monitor(&self, shutdown: CancellationToken) -> Result<()> {
        let mut consecutive_errors = 0u32;

        loop {
            if shutdown.is_cancelled() {
                return Ok(());
            }

            match self.client.get_latest_blockhash().await {
                Ok(hash) => {
                    *self.latest_blockhash.write().await = hash;
                    consecutive_errors = 0;
                    debug!("blockhash updated {}", hash);
                }
                Err(err) => {
                    consecutive_errors = consecutive_errors.saturating_add(1);
                    warn!(
                        "failed to fetch blockhash (attempt {}): {}",
                        consecutive_errors, err
                    );

                    if consecutive_errors >= 10 {
                        error!("rpc blockhash monitor degraded");
                        consecutive_errors = 0;
                    }
                }
            }

            tokio::select! {
                _ = shutdown.cancelled() => {
                    return Ok(());
                }
                _ = sleep(Duration::from_millis(400)) => {}
            }
        }
    }
}
