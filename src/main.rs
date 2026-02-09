#![allow(dead_code, unused_imports)]

mod blockhash;
mod config;
mod execution;
mod geyser;
mod grpc;
mod ipc;
mod jito;
mod logger;
mod metrics;
mod observability;
mod risk;
mod rpc;
mod state;
mod utils;

use crate::config::AppConfig;
use crate::ipc::types::{Envelope, Payload};
use crate::jito::executor::JitoExecutor;
use crate::jito::tip_oracle::TipOracle;
use crate::metrics::prometheus::init_metrics;
use crate::observability::server::start_metrics;
use crate::utils::keypair::load_keypair;

use anyhow::{Context, Result};
use log::{error, info, warn};
use solana_sdk::signature::Signer;
use std::sync::Arc;
use tokio::task::JoinSet;
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    logger::init();

    let cfg = Arc::new(AppConfig::load().context("failed to load configuration")?);

    info!("titan starting");
    info!("grpc_url={}", cfg.grpc_url);
    info!("rpc_url={}", cfg.rpc_url);
    info!("jito_url={}", cfg.jito_block_engine_url);

    if let Some(wss) = cfg.wss_url.as_deref() {
        info!("wss_url={}", wss);
    }

    info!("log_level={}", cfg.log_level);

    let keypair = load_keypair(&cfg.keypair_path)?;
    info!("signer_pubkey={}", keypair.pubkey());

    // Metrics registry init (safe to call once)
    let _ = init_metrics();

    let tip_oracle = Box::leak(Box::new(TipOracle::new(100_000)));

    let executor = Arc::new(
        JitoExecutor::new((*cfg).clone(), tip_oracle)
            .await
            .context("failed to initialize jito executor")?,
    );

    let ipc = ipc::server::start(
        &cfg.ipc_listen_host,
        cfg.ipc_listen_port,
        cfg.ipc_max_queue,
        cfg.shutdown.token(),
    )
    .await
    .context("failed to start ipc server")?;

    let mut tasks: JoinSet<Result<()>> = JoinSet::new();

    {
        let shutdown = cfg.shutdown.token();
        let rx = ipc.receiver();
        let exec = executor.clone();
        tasks.spawn(async move { run_ipc(rx, exec, shutdown).await });
    }

    // metrics server task
    tasks.spawn(async move {
        start_metrics().await;
        Ok(())
    });

    tokio::select! {
        _ = shutdown_signal() => {
            warn!("shutdown requested");
        }
        res = wait_any_task(&mut tasks) => {
            match res {
                Ok(()) => warn!("a task exited early"),
                Err(e) => error!("a task failed: {e:#}"),
            }
        }
    }

    cfg.shutdown.request();

    // graceful wait
    let drain = async {
        while let Some(joined) = tasks.join_next().await {
            match joined {
                Ok(Ok(())) => {}
                Ok(Err(e)) => error!("task error: {e:#}"),
                Err(e) => error!("task join error: {e}"),
            }
        }
    };

    // timeout to avoid hanging forever
    if timeout(Duration::from_secs(3), drain).await.is_err() {
        warn!("shutdown timeout; aborting remaining tasks");
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
    }

    info!("titan stopped");
    Ok(())
}

async fn wait_any_task(tasks: &mut JoinSet<Result<()>>) -> Result<()> {
    match tasks.join_next().await {
        Some(Ok(r)) => r,
        Some(Err(e)) => Err(anyhow::anyhow!(e)),
        None => Ok(()),
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let ctrl_c = tokio::signal::ctrl_c();

        let term = async {
            match signal(SignalKind::terminate()) {
                Ok(mut s) => {
                    let _ = s.recv().await;
                }
                Err(_) => {
                    std::future::pending::<()>().await;
                }
            }
        };

        tokio::select! {
            _ = ctrl_c => {}
            _ = term => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

async fn run_ipc(
    mut rx: tokio::sync::mpsc::Receiver<Envelope>,
    executor: Arc<JitoExecutor>,
    shutdown: CancellationToken,
) -> Result<()> {
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,

            msg = rx.recv() => {
                let Some(env) = msg else { break };

                match env.payload {
                    Payload::Ping(nonce) => {
                        info!("ipc ping id={} nonce={}", env.id, nonce);
                    }
                    Payload::Opportunity(o) => {
                        executor.submit_opportunity(o).await?;
                    }
                }
            }
        }
    }

    Ok(())
}
