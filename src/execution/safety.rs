use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSimulateTransactionConfig;
use solana_sdk::transaction::VersionedTransaction;
use std::sync::Arc;

pub struct SafetyChecker {
    rpc: Arc<RpcClient>,
}

impl SafetyChecker {
    pub fn new(url: String) -> Self {
        Self {
            rpc: Arc::new(RpcClient::new(url)),
        }
    }

    pub async fn simulate(&self, tx: &VersionedTransaction) -> Option<(u64, u64, Vec<String>)> {
        let cfg = RpcSimulateTransactionConfig {
            sig_verify: false,
            replace_recent_blockhash: true,
            ..Default::default()
        };

        let res = self
            .rpc
            .simulate_transaction_with_config(tx, cfg)
            .await
            .ok()?;
        let logs = res.value.logs.unwrap_or_default();

        // këtu normalisht lexon balances nga accounts
        Some((0, 1, logs)) // skeleton i saktë për pipeline
    }
}
