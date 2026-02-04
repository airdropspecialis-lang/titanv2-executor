use anyhow::{Context, Result};
use log::{info, error};
use std::sync::{Arc};
use std::sync::atomic::{AtomicUsize, Ordering};

use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::VersionedTransaction,
    system_instruction,
    hash::Hash,
    message::v0::Message,
};

use jito_searcher_client::searcher_service::SendBundleRequest;
use crate::{config::Config, jito::tip_oracle::TipOracle};

pub struct JitoExecutor {
    client: Arc<jito_searcher_client::SearcherClient>,
    keypair: Arc<Keypair>,
    config: Config,
    tip_accounts: Vec<solana_sdk::pubkey::Pubkey>,
    cursor: AtomicUsize,
    tip_oracle: &'static TipOracle,
}

impl JitoExecutor {
    pub async fn new(config: Config, tip_oracle: &'static TipOracle) -> Result<Self> {
        let keypair = Arc::new(
            solana_sdk::signature::read_keypair_file(&config.keypair_path)?
        );

        let client = jito_searcher_client::get_searcher_client(
            &config.jito_block_engine_url,
            &keypair,
        ).await?;

        let tip_accounts = vec![
            "96g9sAg9u3PBsJpbNcUeghSugTCCBDVbgUXsKUHSHv7Z".parse()?,
            "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe".parse()?,
            "Cw8CFyMvRWqyUvvoQQnQLh6qqw8bsM4fM1SxBcyRLSGS".parse()?,
            "ADaUMid9yfUytqMBBmgrR2iXnd95S2nryhNreNf9Skkd".parse()?,
        ];

        Ok(Self {
            client: Arc::new(client),
            keypair,
            config,
            tip_accounts,
            cursor: AtomicUsize::new(0),
            tip_oracle,
        })
    }

    pub async fn send_bundle(
        &self,
        swap_tx: VersionedTransaction,
        blockhash: Hash,
    ) -> Result<()> {
        let idx = self.cursor.fetch_add(1, Ordering::Relaxed) % self.tip_accounts.len();
        let tip_account = self.tip_accounts[idx];
        let tip_lamports = self.tip_oracle.get();

        let tip_tx = self.build_tip_tx(tip_account, blockhash, tip_lamports)?;

        let req = SendBundleRequest {
            bundle: vec![swap_tx, tip_tx],
            max_leader_slots: Some(self.config.jito_max_leader_slots),
            ..Default::default()
        };

        let client = Arc::clone(&self.client);
        tokio::spawn(async move {
            match client.send_bundle(req).await {
                Ok(resp) => info!("🔥 JITO LANDED {}", resp.into_inner().uuid),
                Err(e) => error!("❌ JITO DROP {:?}", e),
            }
        });

        Ok(())
    }

    fn build_tip_tx(
        &self,
        tip_account: solana_sdk::pubkey::Pubkey,
        blockhash: Hash,
        lamports: u64,
    ) -> Result<VersionedTransaction> {
        let ix = system_instruction::transfer(
            &self.keypair.pubkey(),
            &tip_account,
            lamports,
        );

        let msg = Message::try_compile(
            &self.keypair.pubkey(),
            &[ix],
            &[],
            blockhash,
        ).context("tip compile failed")?;

        Ok(VersionedTransaction::try_new(
            solana_sdk::message::VersionedMessage::V0(msg),
            &[self.keypair.as_ref()],
        )?)
    }
}
