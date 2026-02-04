use crate::config::AppConfig;
use crate::grpc::filter::raydium_transaction_filters;
use crate::jito::bundle::JitoSender;
use crate::rpc::client::SolanaRpc;
use crate::strategy::raydium::RaydiumStrategy;

use anyhow::{anyhow, Context, Result};
use futures::{SinkExt, StreamExt};
use http::Uri;
use log::{debug, error, info, warn};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};
use tonic::Status;
use tonic::transport::ClientTlsConfig;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::{SubscribeRequest, SubscribeRequestPing};

pub struct GeyserClient {
    cfg: Arc<AppConfig>,
    rpc: Arc<SolanaRpc>,
    jito: Arc<JitoSender>,
    inflight: Arc<Semaphore>,
}

impl GeyserClient {
    pub fn new(cfg: Arc<AppConfig>, rpc: Arc<SolanaRpc>, jito: Arc<JitoSender>) -> Result<Self> {
        let max_inflight = cfg.grpc_max_inflight.max(1);
        Ok(Self {
            cfg,
            rpc,
            jito,
            inflight: Arc::new(Semaphore::new(max_inflight)),
        })
    }

    pub async fn run(self) -> Result<()> {
        let shutdown = self.cfg.shutdown.token();

        let mut backoff_ms = self.cfg.grpc_reconnect_min_ms.max(1);
        let max_backoff_ms = self.cfg.grpc_reconnect_max_ms.max(backoff_ms);

        loop {
            if shutdown.is_cancelled() {
                info!("grpc loop stopped (shutdown)");
                return Ok(());
            }

            info!("grpc connect attempt url={}", self.cfg.grpc_url);

            match self.connect_and_consume().await {
                Ok(()) => {
                    if shutdown.is_cancelled() {
                        info!("grpc stream ended (shutdown)");
                        return Ok(());
                    }
                    warn!("grpc stream ended; reconnecting");
                }
                Err(e) => {
                    if shutdown.is_cancelled() {
                        info!("grpc error during shutdown: {e:#}");
                        return Ok(());
                    }
                    error!("grpc error: {e:#}");
                }
            }

            let sleep_dur = Duration::from_millis(backoff_ms);
            debug!("grpc reconnect backoff_ms={}", backoff_ms);

            tokio::select! {
                _ = shutdown.cancelled() => {
                    info!("grpc loop stopped (shutdown)");
                    return Ok(());
                }
                _ = sleep(sleep_dur) => {}
            }

            backoff_ms = (backoff_ms.saturating_mul(2)).min(max_backoff_ms);
        }
    }

    async fn connect_and_consume(&self) -> Result<()> {
        let shutdown = self.cfg.shutdown.token();

        let grpc_url = self.cfg.grpc_url.clone();
        let mut builder = GeyserGrpcClient::build_from_shared(grpc_url.clone())
            .context("invalid GRPC_URL")?;

        if grpc_url.starts_with("https://") {
            let uri: Uri = grpc_url.parse().context("invalid GRPC_URL (uri parse)")?;
            let host = uri.host().ok_or_else(|| anyhow!("invalid GRPC_URL: missing host"))?;

            let tls = ClientTlsConfig::new()
                .with_native_roots()
                .domain_name(host);

            builder = builder
                .tls_config(tls)
                .context("failed to set grpc tls config")?;
        }

        let mut client = builder
            .x_token(Some(self.cfg.grpc_token.clone()))
            .context("failed to set GRPC token")?
            .connect()
            .await
            .context("failed to connect to geyser grpc")?;

        let (mut subscribe_tx, mut stream) = client.subscribe().await.context("subscribe failed")?;

        let req = SubscribeRequest {
            transactions: raydium_transaction_filters(),
            ping: Some(SubscribeRequestPing { id: 1 }),
            ..Default::default()
        };

        subscribe_tx
            .send(req)
            .await
            .context("failed to send subscribe request")?;

        info!("grpc subscribed: raydium_amm_v4");

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    info!("grpc consume stopped (shutdown)");
                    return Ok(());
                }
                next = stream.next() => {
                    match next {
                        None => {
                            warn!("grpc stream closed by server");
                            return Ok(());
                        }
                        Some(Err(status)) => {
                            return Err(status_to_anyhow(status).context("grpc stream error"));
                        }
                        Some(Ok(update)) => {
                            let permit = match self.inflight.clone().acquire_owned().await {
                                Ok(p) => p,
                                Err(_) => return Ok(()),
                            };

                            let rpc = self.rpc.clone();
                            let jito = self.jito.clone();
                            let cfg = self.cfg.clone();

                            tokio::spawn(async move {
                                let _permit = permit;
                                if let Err(e) = RaydiumStrategy::process_update(update, &rpc, &jito, &cfg).await {
                                    error!("strategy error: {e:#}");
                                }
                            });
                        }
                    }
                }
            }
        }
    }
}

fn status_to_anyhow(status: Status) -> anyhow::Error {
    anyhow::anyhow!(
        "grpc status code={:?} message={}",
        status.code(),
        status.message()
    )
}
