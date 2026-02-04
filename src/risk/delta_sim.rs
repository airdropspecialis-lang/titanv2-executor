use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::transaction::VersionedTransaction;

pub async fn net_profit_ok(
    rpc: &RpcClient,
    tx: &VersionedTransaction,
    min_out: u64,
) -> bool {
    let sim = rpc.simulate_transaction(tx).await.ok();
    let Some(sim) = sim else { return false };

    let post = sim.value.post_balances.get(0).copied().unwrap_or(0);

    if post == 0 || post < min_out {
        return false;
    }

    if let Some(logs) = sim.value.logs {
        let blacklist = ["honeypot", "frozen", "blacklist"];
        for l in logs {
            if blacklist.iter().any(|x| l.to_lowercase().contains(x)) {
                return false;
            }
        }
    }

    true
}
