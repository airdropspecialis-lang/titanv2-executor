use anyhow::Result;
use futures::StreamExt;
use log::info;

use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient};
use yellowstone_grpc_proto::geyser::{
    subscribe_update::UpdateOneof, SubscribeRequest, SubscribeRequestFilterAccounts,
    SubscribeRequestFilterSlots,
};

use crate::state::blockhash::BlockhashCache;
use solana_client::nonblocking::rpc_client::RpcClient;

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

    let h = rpc.get_latest_blockhash().await?;
    blockhash_cache.set(h);

    let mut builder = GeyserGrpcClient::build_from_shared(grpc_url)?;
    builder = builder
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .x_token(grpc_token)?;

    let mut client = builder.connect().await?;
    let mut req = SubscribeRequest::default();

    req.slots
        .insert("slots".to_string(), SubscribeRequestFilterSlots::default());

    let mut acc_filter = SubscribeRequestFilterAccounts::default();
    acc_filter
        .account
        .push("675kPX9MHTjS2zt1qfr1NYHuHdiXESLiG1e66f4Hmcfs".to_string());

    req.accounts.insert("liquidity".to_string(), acc_filter);

    let (_, mut stream) = client.subscribe_with_request(Some(req)).await?;
    info!("🚀 Geyser stream active (slots + liquidity)");

    while let Some(msg) = stream.next().await {
        let Ok(update) = msg else { continue };

        match update.update_oneof {
            Some(UpdateOneof::Slot(_)) => {
                if let Ok(h) = rpc.get_latest_blockhash().await {
                    blockhash_cache.set(h);
                }
            }

            Some(UpdateOneof::Account(acc)) => {
                if let Some(actual_account) = acc.account {
                    let pubkey_str = bs58::encode(actual_account.pubkey).into_string();

                    let signal = LiquiditySignal {
                        account: pubkey_str,
                        slot: acc.slot,
                    };

                    let _ = liquidity_tx.try_send(signal);
                }
            }

            _ => {}
        }
    }

    Ok(())
}
