use solana_sdk::pubkey::Pubkey;
use tokio::sync::mpsc::Receiver;

use crate::finder::conflict_guard::ConflictGuard;
use crate::finder::stream_worker::LiquiditySignal;

pub async fn finder_loop(
    mut rx: Receiver<LiquiditySignal>,
) {
    let mut conflict_guard = ConflictGuard::new();

    while let Some(signal) = rx.recv().await {
        let account = signal.account;
        let slot = signal.slot;

        // 🛡️ Conflict Guard (FINAL LINE OF DEFENSE)
        if conflict_guard.should_skip(account, slot) {
            continue;
        }

        // 👉 KËTU VETËM NJË HERË PËR account + slot:
        // 1. Parse vault balances
        // 2. Build opportunity
        // 3. Delta simulation
        // 4. Slippage-aware net_profit_ok
        // 5. Send to JitoExecutor
    }
}
