use crate::{
    config::AppConfig,
    jito::bundle::{BundleIntent, JitoSender},
    rpc::client::SolanaRpc,
};

use anyhow::{Context, Result};
use log::info;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, hash::Hash, instruction::Instruction, pubkey::Pubkey,
    signer::Signer, transaction::Transaction,
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

        let intent = BundleIntent::RaydiumNewPool { tx };
        jito.submit_bundle(intent, rpc, cfg)
            .await
            .context("submit_bundle failed")?;

        info!("raydium bundle submitted");
        Ok(())
    }

    fn build_swap_tx(
        cfg: &AppConfig,
        target_mint: Pubkey,
        recent_blockhash: Hash,
    ) -> Result<Transaction> {
        let payer = cfg.signer.pubkey();

        let cu_limit = ComputeBudgetInstruction::set_compute_unit_limit(150_000);
        let cu_price = ComputeBudgetInstruction::set_compute_unit_price(1000);

        let mut instructions = vec![cu_limit, cu_price];

        let swap_ixs = Self::build_swap_instructions(payer, target_mint, 100_000)?;

        instructions.extend(swap_ixs);

        let mut tx = Transaction::new_with_payer(&instructions, Some(&payer));

        tx.try_sign(&[cfg.signer.as_ref()], recent_blockhash)
            .map_err(|e| anyhow::anyhow!("Signing error: {}", e))?;

        Ok(tx)
    }

    fn build_swap_instructions(
        _payer: Pubkey,
        _target_mint: Pubkey,
        _amount: u64,
    ) -> Result<Vec<Instruction>> {
        Ok(vec![])
    }
}
