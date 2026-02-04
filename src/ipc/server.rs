use crate::ipc::types::Envelope;
use anyhow::{Context, Result};
use log::{info, warn};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;

pub struct IpcServer {
    rx: mpsc::Receiver<Envelope>,
}

impl IpcServer {
    pub fn receiver(self) -> mpsc::Receiver<Envelope> {
        self.rx
    }
}

pub async fn start(
    host: &str,
    port: u16,
    max_queue: usize,
    shutdown: CancellationToken,
) -> Result<IpcServer> {
    let bind_addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind IPC listener on {bind_addr}"))?;

    let (tx, rx) = mpsc::channel::<Envelope>(max_queue);

    tokio::spawn(async move {
        info!("ipc listening on {}", bind_addr);

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    info!("ipc shutting down");
                    break;
                }
                accepted = listener.accept() => {
                    let (stream, peer) = match accepted {
                        Ok(v) => v,
                        Err(e) => {
                            warn!("ipc accept error: {}", e);
                            continue;
                        }
                    };

                    let txc = tx.clone();
                    let sd = shutdown.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_conn(stream, peer.to_string(), txc, sd).await {
                            warn!("ipc connection ended: {}", e);
                        }
                    });
                }
            }
        }
    });

    Ok(IpcServer { rx })
}

async fn handle_conn(
    stream: TcpStream,
    peer: String,
    tx: mpsc::Sender<Envelope>,
    shutdown: CancellationToken,
) -> Result<()> {
    info!("ipc connected peer={}", peer);

    let mut lines = BufReader::new(stream).lines();

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,
            line = lines.next_line() => {
                let Some(line) = line.context("ipc read error")? else { break };

                let env: Envelope = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("ipc invalid json from {}: {}", peer, e);
                        continue;
                    }
                };

                if tx.try_send(env).is_err() {
                    warn!("ipc queue full; dropping message from {}", peer);
                }
            }
        }
    }

    info!("ipc disconnected peer={}", peer);
    Ok(())
}
