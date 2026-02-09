use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSimulateTransactionConfig;
use solana_sdk::transaction::VersionedTransaction;

pub async fn net_profit_ok(rpc: &RpcClient, tx: &VersionedTransaction, _min_out: u64) -> bool {
    let config = RpcSimulateTransactionConfig {
        sig_verify: false,
        replace_recent_blockhash: true,
        commitment: None,
        encoding: None,
        accounts: None,
        min_context_slot: None,
        inner_instructions: false,
    };

    let sim_res = rpc.simulate_transaction_with_config(tx, config).await.ok();
    let Some(sim) = sim_res else {
        return false;
    };

    if sim.value.err.is_some() {
        return false;
    }

    if let Some(logs) = sim.value.logs {
        let blacklist = ["honeypot", "frozen", "blacklist", "error", "insufficient"];
        for l in logs {
            let lower_log = l.to_lowercase();
            if blacklist.iter().any(|x| lower_log.contains(x)) {
                return false;
            }
        }
    }

    true
}
