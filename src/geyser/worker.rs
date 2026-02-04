use anyhow::Result;
use futures::StreamExt;
use std::sync::Arc;

use solana_client::nonblocking::rpc_client::RpcClient;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::{
    SubscribeRequest, SubscribeRequestFilterSlots, SubscribeRequestFilterAccounts,
};

use crate::blockhash::cache::BlockhashCache;

pub async fn geyser_worker(
    endpoint: String,
    token: Option<String>,
    cache: Arc<BlockhashCache>,
    rpc: RpcClient,
) -> Result<()> {
    let mut client = GeyserGrpcClient::connect(endpoint, token, None).await?;

    let mut req = SubscribeRequest::default();

    req.slots.insert(
        "slots".into(),
        SubscribeRequestFilterSlots::default(),
    );

    let mut accounts = SubscribeRequestFilterAccounts::default();
    accounts.account.push(
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".into(), // Raydium AMM
    );
    req.accounts.insert("liquidity".into(), accounts);

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
