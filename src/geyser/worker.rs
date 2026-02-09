use anyhow::Result;
use futures::StreamExt;
use std::sync::Arc;

use solana_client::nonblocking::rpc_client::RpcClient;
use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient};
use yellowstone_grpc_proto::geyser::{
    SubscribeRequest, SubscribeRequestFilterAccounts, SubscribeRequestFilterSlots,
};

use crate::blockhash::cache::BlockhashCache;

pub async fn geyser_worker(
    endpoint: String,
    token: Option<String>,
    cache: Arc<BlockhashCache>,
    rpc: RpcClient,
) -> Result<()> {
    // RREGULLIMI: Përdorimi i Builder në vend të .connect() direkt
    let mut builder = GeyserGrpcClient::build_from_shared(endpoint)?;

    builder = builder
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .x_token(token)?;

    let mut client = builder.connect().await?;

    let mut req = SubscribeRequest::default();

    req.slots
        .insert("slots".into(), SubscribeRequestFilterSlots::default());

    let mut accounts = SubscribeRequestFilterAccounts::default();
    accounts.account.push(
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".into(), // Raydium AMM
    );
    req.accounts.insert("liquidity".into(), accounts);

    // Tani që klienti është krijuar saktë, Rust e identifikon tipin e stream-it automatikisht
    let (_, mut stream) = client.subscribe_with_request(Some(req)).await?;

    while let Some(msg) = stream.next().await {
        if msg.is_ok() {
            if let Ok(h) = rpc.get_latest_blockhash().await {
                cache.set(h);
            }
        }
    }

    Ok(())
}
