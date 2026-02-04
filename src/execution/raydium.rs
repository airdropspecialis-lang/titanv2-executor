use crate::{
    config::AppConfig,
    jito::bundle::{BundleIntent, JitoSender},
    rpc::client::SolanaRpc,
};

use anyhow::{Context, Result};
use log::{info, warn};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    hash::Hash,
    pubkey::Pubkey,
    signer::Signer,
    transaction::Transaction,
};

pub struct RaydiumExecutor;

impl RaydiumExecutor {
    pub async fn execute_new_pool(
        rpc: &SolanaRpc,
        jito: &JitoSender,
        cfg: &AppConfig,
        target_mint: Pubkey,
        recent_blockhash: Hash,
    ) -> Result<()> {
        info!("raydium execute_new_pool mint={}", target_mint);

        let tx = Self::build_swap_tx(cfg, target_mint, recent_blockhash)
            .context("build_swap_tx failed")?;

        if cfg.enable_simulation && !Self::simulate_ok(rpc, &tx).await {
            warn!("simulate rejected");
            return Ok(());
        }

        let intent = BundleIntent::RaydiumNewPool { tx };
        jito.submit_bundle(intent, rpc, cfg)
            .await
            .context("submit_bundle failed")?;

        info!("raydium bundle submitted");
        Ok(())
    }

    fn build_swap_tx(cfg: &AppConfig, target_mint: Pubkey, recent_blockhash: Hash) -> Result<Transaction> {
        let payer = cfg.signer.pubkey();

        let cu_limit = ComputeBudgetInstruction::set_compute_unit_limit(
            cfg.compute_unit_limit.unwrap_or(150_000),
        );
        let cu_price =
            ComputeBudgetInstruction::set_compute_unit_price(cfg.priority_fee_micro_lamports);

        let mut instructions = vec![cu_limit, cu_price];

        let swap_ixs = crate::raydium::instruction::build_swap_instructions(
            payer,
            target_mint,
            cfg.trade_amount_lamports,
            cfg.max_slippage_bps,
        )
        .context("build_swap_instructions failed")?;

        instructions.extend(swap_ixs);

        let mut tx = Transaction::new_with_payer(&instructions, Some(&payer));
        tx.try_sign(&[cfg.signer.as_ref()], recent_blockhash)
            .context("tx signing failed")?;

        Ok(tx)
    }

    async fn simulate_ok(rpc: &SolanaRpc, tx: &Transaction) -> bool {
        match rpc.simulate_transaction(tx).await {
            Ok(sim) => {
                if let Some(err) = sim.value.err {
                    warn!("simulate error: {:?}", err);
                    return false;
                }
                true
            }
            Err(e) => {
                warn!("simulate rpc error: {:?}", e);
                false
            }
        }
    }
}
