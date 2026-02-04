mod config;
mod grpc;
mod ipc;
mod jito;
mod logger;
mod rpc;
mod strategy;
mod utils;
mod finder; // 👈 TITAN ADDITION

use crate::config::AppConfig;
use crate::grpc::client::GeyserClient;
use crate::ipc::types::{Envelope, Kind, Payload};
use crate::jito::bundle::JitoSender;
use crate::rpc::client::SolanaRpc;
use crate::utils::keypair::load_keypair;

use crate::finder::stream_worker::stream_worker;
use crate::finder::logic::finder_loop;

use anyhow::{Context, Result};
use log::{error, info, warn};
use solana_sdk::signature::Signer;
use std::sync::Arc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    logger::init();

    let cfg = Arc::new(AppConfig::load().context("failed to load configuration")?);

    info!("🚀 titan_bot starting");
    info!("grpc_url={}", cfg.grpc_url);
    info!("rpc_url={}", cfg.rpc_url);
    info!("jito_url={}", cfg.jito_block_engine_url);

    if let Some(wss) = cfg.wss_url.as_deref() {
        info!("wss_url={}", wss);
    }

    info!("log_level={}", cfg.log_level);

    let kp = load_keypair(&cfg.keypair_path)?;
    info!("signer_pubkey={}", kp.pubkey());

    let rpc = Arc::new(
        SolanaRpc::new(&cfg.rpc_url).context("failed to init rpc client")?
    );

    let jito = Arc::new(
        JitoSender::new(
            &cfg.jito_block_engine_url,
            &cfg.jito_tip_account,
            cfg.jito_tip_lamports,
        )
        .context("failed to init jito sender")?,
    );

    let geyser = GeyserClient::new(
        cfg.clone(),
        rpc.clone(),
        jito.clone(),
    )
    .context("failed to init geyser client")?;

    // ─────────────────────────────────────────────
    // CHANNELS (Titan Data Plane)
    // ─────────────────────────────────────────────

    let (geyser_tx, geyser_rx) = tokio::sync::mpsc::channel(2048);
    let (finder_tx, finder_rx) = tokio::sync::mpsc::channel(2048);

    // Geyser pushes LiquiditySignal into geyser_tx
    geyser.set_output(geyser_tx);

    // ─────────────────────────────────────────────
    // IPC SERVER (optional / backward compatibility)
    // ─────────────────────────────────────────────

    let ipc = ipc::server::start(
        &cfg.ipc_listen_host,
        cfg.ipc_listen_port,
        cfg.ipc_max_queue,
        cfg.shutdown.token(),
    )
    .await
    .context("failed to start ipc server")?;

    let mut tasks: JoinSet<Result<()>> = JoinSet::new();

    // IPC task
    {
        let shutdown = cfg.shutdown.token();
        let rx = ipc.receiver();
        tasks.spawn(async move { run_ipc(rx, shutdown).await });
    }

    // Blockhash monitor (RPC fallback / metrics / safety)
    {
        let rpc_bg = rpc.clone();
        let shutdown = cfg.shutdown.token();
        tasks.spawn(async move {
            rpc_bg.run_blockhash_monitor(shutdown).await
        });
    }

    // Geyser gRPC main loop
    tasks.spawn(async move {
        geyser.run().await
    });

    // 🔥 STREAM WORKER (Geyser → Finder)
    tasks.spawn(async move {
        stream_worker(geyser_rx, finder_tx).await;
        Ok(())
    });

    // 🧠 FINDER CORE LOOP (ConflictGuard + Risk + Execution)
    tasks.spawn(async move {
        finder_loop(finder_rx).await;
        Ok(())
    });

    // ─────────────────────────────────────────────
    // SHUTDOWN HANDLING
    // ─────────────────────────────────────────────

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            warn!("shutdown requested (ctrl_c)");
        }
        res = wait_any_task(&mut tasks) => {
            match res {
                Ok(()) => warn!("a task exited early; shutting down"),
                Err(e) => error!("a task failed; shutting down: {e:#}"),
            }
        }
    }

    cfg.shutdown.request();

    while let Some(joined) = tasks.join_next().await {
        match joined {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                error!("task error: {e:#}");
            }
            Err(e) => {
                error!("task join error: {e}");
            }
        }
    }

    info!("🛑 titan_bot stopped");
    Ok(())
}

async fn wait_any_task(tasks: &mut JoinSet<Result<()>>) -> Result<()> {
    match tasks.join_next().await {
        Some(Ok(r)) => r,
        Some(Err(e)) => Err(anyhow::anyhow!(e)),
        None => Ok(()),
    }
}

async fn run_ipc(
    mut rx: tokio::sync::mpsc::Receiver<Envelope>,
    shutdown: CancellationToken,
) -> Result<()> {
    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            msg = rx.recv() => {
                let Some(env) = msg else { break };

                match (env.kind, env.payload) {
                    (Kind::Ping, Payload::Ping(p)) => {
                        info!("ipc ping id={} nonce={}", env.id, p.nonce);
                    }
                    (Kind::Opportunity, Payload::Opportunity(o)) => {
                        info!(
                            "ipc opportunity id={} strategy={:?} dex={:?} mint={} min_liq={} max_slip_bps={} max_delay_ms={}",
                            env.id,
                            o.strategy,
                            o.dex,
                            o.mint,
                            o.min_liquidity,
                            o.max_slippage_bps,
                            o.max_delay_ms
                        );
                    }
                    _ => {
                        warn!("ipc invalid envelope id={}", env.id);
                    }
                }
            }
        }
    }

    Ok(())
}
