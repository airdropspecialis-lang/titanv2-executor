use crate::ipc::types::Envelope;
use anyhow::{Context, Result};
use futures::StreamExt;
use log::{info, warn};
use rkyv::{Deserialize, Infallible};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, LengthDelimitedCodec};
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
                    let peer = peer.to_string();

                    tokio::spawn(async move {
                        if let Err(e) = handle_conn(stream, peer, txc, sd).await {
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

    // ✅ Length-prefixed framing → rkyv SAFE
    let mut framed = FramedRead::new(
        stream,
        LengthDelimitedCodec::builder()
            .max_frame_length(64 * 1024) // siguri, jo bottleneck
            .new_codec(),
    );

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => break,

            frame = framed.next() => {
                let Some(frame) = frame else { break };
                let bytes = frame.context("ipc frame error")?;

                // ✅ verifikim strukture (pa UB)
                let archived = rkyv::check_archived_root::<Envelope>(&bytes)
                    .map_err(|_| anyhow::anyhow!("corrupt IPC frame"))?;

                let env: Envelope =
                    archived.deserialize(&mut Infallible)?;

                if tx.try_send(env).is_err() {
                    warn!("ipc queue full; dropping message from {}", peer);
                }
            }
        }
    }

    info!("ipc disconnected peer={}", peer);
    Ok(())
}
