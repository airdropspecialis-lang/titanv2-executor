use crate::config::AppConfig;
use crate::grpc::filter::raydium_transaction_filters;
use crate::jito::bundle::JitoSender;
use crate::rpc::client::SolanaRpc;

use anyhow::{anyhow, Context, Result};
use futures::{SinkExt, StreamExt};
use http::Uri;
use log::{debug, error, info, warn};
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use tokio::time::{sleep, Duration};
use tonic::transport::ClientTlsConfig;
use tonic::Status;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::{SubscribeRequest, SubscribeRequestPing};

pub struct GeyserClient {
    cfg: Arc<AppConfig>,
    _rpc: Arc<SolanaRpc>,   // kept for future, not used here
    _jito: Arc<JitoSender>, // kept for future, not used here
    inflight: Arc<Semaphore>,
    output: Option<mpsc::Sender<crate::geyser::LiquiditySignal>>,
}

impl GeyserClient {
    pub fn new(cfg: Arc<AppConfig>, rpc: Arc<SolanaRpc>, jito: Arc<JitoSender>) -> Result<Self> {
        let max_inflight = cfg.grpc_max_inflight.max(1);
        Ok(Self {
            cfg,
            _rpc: rpc,
            _jito: jito,
            inflight: Arc::new(Semaphore::new(max_inflight)),
            output: None,
        })
    }

    pub fn set_output(&mut self, tx: mpsc::Sender<crate::geyser::LiquiditySignal>) {
        self.output = Some(tx);
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
                        return Ok(());
                    }
                    warn!("grpc stream ended; reconnecting");
                }
                Err(e) => {
                    if shutdown.is_cancelled() {
                        return Ok(());
                    }
                    error!("grpc error: {e:#}");
                }
            }

            let sleep_dur = Duration::from_millis(backoff_ms);
            debug!("grpc reconnect backoff_ms={}", backoff_ms);

            tokio::select! {
                _ = shutdown.cancelled() => return Ok(()),
                _ = sleep(sleep_dur) => {}
            }

            backoff_ms = (backoff_ms.saturating_mul(2)).min(max_backoff_ms);
        }
    }

    async fn connect_and_consume(&self) -> Result<()> {
        let shutdown = self.cfg.shutdown.token();

        let grpc_url = self.cfg.grpc_url.clone();
        let mut builder =
            GeyserGrpcClient::build_from_shared(grpc_url.clone()).context("invalid GRPC_URL")?;

        if grpc_url.starts_with("https://") {
            let uri: Uri = grpc_url.parse().context("invalid GRPC_URL")?;
            let host = uri.host().ok_or_else(|| anyhow!("GRPC_URL missing host"))?;

            let tls = ClientTlsConfig::new().with_native_roots().domain_name(host);

            builder = builder.tls_config(tls)?;
        }

        let mut client = builder
            .x_token(Some(self.cfg.grpc_token.clone()))?
            .connect()
            .await
            .context("failed to connect geyser grpc")?;

        let (mut subscribe_tx, mut stream) =
            client.subscribe().await.context("subscribe failed")?;

        let req = SubscribeRequest {
            transactions: raydium_transaction_filters(),
            ping: Some(SubscribeRequestPing { id: 1 }),
            ..Default::default()
        };

        subscribe_tx.send(req).await?;

        info!("grpc subscribed (observer mode)");

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => return Ok(()),
                next = stream.next() => {
                    match next {
                        None => return Ok(()),
                        Some(Err(status)) => {
                            return Err(status_to_anyhow(status));
                        }
                        Some(Ok(_update)) => {
                            let _permit = self.inflight.acquire().await?;

                            // OBSERVER PATTERN:
                            // gRPC only forwards updates, no processing here
                            if let Some(ref tx) = self.output {
                                let signal = crate::geyser::LiquiditySignal {
                                    account: "observer".to_string(),
                                    slot:0,
                                };
                                let _ = tx.send(signal).await;

                            }
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
