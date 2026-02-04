use anyhow::Result;
use futures::StreamExt;
use log::{info, debug};
use std::sync::Arc;

use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient};
use yellowstone_grpc_proto::geyser::{
    SubscribeRequest,
    SubscribeRequestFilterSlots,
    SubscribeRequestFilterAccounts,
    subscribe_update::UpdateOneof,
};

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::hash::Hash;

use crate::state::blockhash::BlockhashCache;

/// Sinjal i pastër – nuk ka trade logic këtu
#[derive(Debug, Clone)]
pub struct LiquiditySignal {
    pub account: String,
    pub slot: u64,
}

pub async fn run_geyser_stream(
    grpc_url: String,
    grpc_token: Option<String>,
    rpc_url: String,
    blockhash_cache: BlockhashCache,
    liquidity_tx: tokio::sync::mpsc::Sender<LiquiditySignal>,
) -> Result<()> {
    let rpc = RpcClient::new(rpc_url);

    // Boot hash
    let h = rpc.get_latest_blockhash().await?;
    blockhash_cache.set(h);

    let mut builder = GeyserGrpcClient::build_from_shared(grpc_url)?;
    builder = builder
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .x_token(grpc_token)?;

    let mut client = builder.connect().await?;

    let mut req = SubscribeRequest::default();

    // 🔹 SLOT HEARTBEAT (blockhash sync)
    req.slots.insert(
        "slots".to_string(),
        SubscribeRequestFilterSlots::default(),
    );

    // 🔹 ACCOUNT UPDATES (Liquidity Sniper)
    let mut acc_filter = SubscribeRequestFilterAccounts::default();

    // 👉 Raydium AMM v4 authority / program
    acc_filter.account.push(
        "675kPX9MHTjS2zt1qfr1NYHuHdiXESLiG1e66f4Hmcfs".to_string()
    );

    // 👉 (opsionale) Pump.fun global / bonding
    // acc_filter.account.push("pumpfun_global_address".to_string());

    req.accounts.insert(
        "liquidity".to_string(),
        acc_filter,
    );

    let (_, mut stream) = client.subscribe_with_request(Some(req)).await?;
    info!("🚀 Geyser stream active (slots + liquidity)");

    while let Some(msg) = stream.next().await {
        let Ok(update) = msg else { continue };

        match update.update_oneof {
            // 🔥 SLOT → refresh hash instantly
            Some(UpdateOneof::Slot(_)) => {
                if let Ok(h) = rpc.get_latest_blockhash().await {
                    blockhash_cache.set(h);
                }
            }

            // 🔥 ACCOUNT UPDATE → Liquidity movement
            Some(UpdateOneof::Account(acc)) => {
                let signal = LiquiditySignal {
                    account: acc.pubkey.clone(),
                    slot: acc.slot,
                };

                // non-blocking, drop if congested (freshness > completeness)
                let _ = liquidity_tx.try_send(signal);
            }

            _ => {}
        }
    }

    Ok(())
}
