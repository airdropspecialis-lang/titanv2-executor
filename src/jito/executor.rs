use anyhow::{Context, Result};
use log::{error, info};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::{mpsc, Mutex};

use solana_sdk::{
    hash::Hash, message::v0::Message, signature::Signer, system_instruction,
    transaction::VersionedTransaction,
};

use jito_protos::{bundle::Bundle, convert::proto_packet_from_versioned_tx};

use crate::{config::AppConfig as Config, ipc::types::Opportunity, jito::tip_oracle::TipOracle};

pub struct JitoExecutor {
    client: Arc<Mutex<jito_searcher_client::SearcherClient>>,
    config: Config,
    tip_accounts: Vec<solana_sdk::pubkey::Pubkey>,
    cursor: AtomicUsize,
    tip_oracle: &'static TipOracle,
    input_tx: mpsc::Sender<Opportunity>,
}

impl JitoExecutor {
    pub async fn new(config: Config, tip_oracle: &'static TipOracle) -> Result<Self> {
        let keypair = config.signer.clone();

        let client =
            jito_searcher_client::get_searcher_client(&config.jito_block_engine_url, &keypair)
                .await
                .map_err(|e| anyhow::anyhow!("jito client error: {}", e))?;

        let tip_accounts = vec![
            "96g9sAg9u3mBsJqcPRrGQv7Q4dp9wPMi65SfsBXYEJp5".parse()?,
            "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe".parse()?,
            "Cw8CFyM9FkoMi7K7Crf6HNWoAcFwasCsyvqzWkRGzZ7a".parse()?,
            "ADuBvALN67C9Cc4WansRfsEM9BpneysqiHshsZhg6S6S".parse()?,
            "DfXygSm4jCyNCmb3qSfeR38CZ97H8UUnY9U4968ivY4A".parse()?,
            "ADaUMid9yfUytqMBqkhicWLYGv51LAnp3id8qXvU9puz".parse()?,
            "DttWaMuVvTiduGkgbeGvE9No88onY64Xaxr8v8q7SQu9".parse()?,
            "3AVi9Tg9Uo68tJfuAWMwoU62Pjm3nGuUu98sh3Eqa73m".parse()?,
        ];

        let (input_tx, input_rx) = mpsc::channel(1024);

        let executor = Self {
            client: Arc::new(Mutex::new(client)),
            config,
            tip_accounts,
            cursor: AtomicUsize::new(0),
            tip_oracle,
            input_tx,
        };

        executor.spawn_worker(input_rx);

        Ok(executor)
    }

    pub async fn submit_opportunity(&self, o: Opportunity) -> Result<()> {
        self.input_tx.send(o).await?;
        Ok(())
    }

    fn spawn_worker(&self, mut rx: mpsc::Receiver<Opportunity>) {
        let client = Arc::clone(&self.client);

        tokio::spawn(async move {
            while let Some(o) = rx.recv().await {
                info!(
                    "executor received opportunity strategy={:?} dex={:?} mint={:?}",
                    o.strategy, o.dex, o.mint
                );

                // Execution logic will be added here
                // Build swap tx
                // Fetch blockhash
                // send_bundle(...)
            }

            drop(client);
        });
    }

    pub async fn send_bundle(&self, swap_tx: VersionedTransaction, blockhash: Hash) -> Result<()> {
        let idx = self.cursor.fetch_add(1, Ordering::Relaxed) % self.tip_accounts.len();
        let tip_account = self.tip_accounts[idx];
        let tip_lamports = self.tip_oracle.get();

        let tip_tx = self.build_tip_tx(tip_account, blockhash, tip_lamports)?;

        let bundle = Bundle {
            header: None,
            packets: vec![
                proto_packet_from_versioned_tx(&swap_tx),
                proto_packet_from_versioned_tx(&tip_tx),
            ],
        };

        let client = Arc::clone(&self.client);

        tokio::spawn(async move {
            let mut client = client.lock().await;
            match client.send_bundle(bundle).await {
                Ok(resp) => {
                    info!("jito bundle landed uuid={}", resp.uuid);
                }
                Err(e) => {
                    error!("jito bundle dropped {:?}", e);
                }
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
        let payer = self.config.signer.pubkey();

        let ix = system_instruction::transfer(&payer, &tip_account, lamports);

        let msg = Message::try_compile(&payer, &[ix], &[], blockhash)
            .context("tip compilation failed")?;

        let versioned_msg = solana_sdk::message::VersionedMessage::V0(msg);

        let tx = VersionedTransaction::try_new(versioned_msg, &[self.config.signer.as_ref()])
            .map_err(|e| anyhow::anyhow!("tip signing error: {}", e))?;

        Ok(tx)
    }
}
