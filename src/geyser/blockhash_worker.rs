use anyhow::Result;
use futures::StreamExt;
use log::info;

use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient};
use yellowstone_grpc_proto::geyser::{SubscribeRequest, SubscribeRequestFilterSlots};

use crate::state::blockhash::BlockhashCache;
use solana_client::nonblocking::rpc_client::RpcClient;

pub async fn run_geyser_blockhash_worker(
    grpc_url: String,
    grpc_token: Option<String>,
    rpc_url: String,
    cache: BlockhashCache,
) -> Result<()> {
    let rpc = RpcClient::new(rpc_url);

    let initial = rpc.get_latest_blockhash().await?;
    cache.set(initial);

    let mut builder = GeyserGrpcClient::build_from_shared(grpc_url)?;

    builder = builder
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .x_token(grpc_token)?;

    let mut client = builder.connect().await?;

    let mut req = SubscribeRequest::default();
    req.slots
        .insert("slots".to_string(), SubscribeRequestFilterSlots::default());

    let (_, mut stream) = client.subscribe_with_request(Some(req)).await?;

    info!("🧠 Geyser SLOT stream active (Blockhash sync)");

    while let Some(msg) = stream.next().await {
        let Ok(update) = msg else { continue };

        if update.update_oneof.is_some() {
            if let Ok(h) = rpc.get_latest_blockhash().await {
                cache.set(h);
            }
        }
    }

    Ok(())
}
