use anyhow::{Context, Result};
use std::env;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct AppConfig {
    pub rpc_url: String,
    pub wss_url: Option<String>,

    pub grpc_url: String,
    pub grpc_token: String,

    pub jito_block_engine_url: String,
    pub jito_tip_account: String,
    pub jito_tip_lamports: u64,

    pub keypair_path: String,

    pub min_liquidity: f64,

    pub grpc_max_inflight: usize,
    pub grpc_reconnect_min_ms: u64,
    pub grpc_reconnect_max_ms: u64,

    // IPC (MEV Finder -> titan executor)
    pub ipc_listen_host: String,
    pub ipc_listen_port: u16,
    pub ipc_max_queue: usize,

    pub log_level: String,

    pub shutdown: Shutdown,
}

#[derive(Clone)]
pub struct Shutdown {
    token: CancellationToken,
}

impl Shutdown {
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    pub fn request(&self) {
        self.token.cancel();
    }

    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        dotenvy::dotenv().ok();

        let rpc_url = required("RPC_URL")?;
        let grpc_url = required("GRPC_URL")?;
        let grpc_token = required("GRPC_TOKEN")?;
        let jito_block_engine_url = required("JITO_BLOCK_ENGINE_URL")?;
        let jito_tip_account = required("JITO_TIP_ACCOUNT")?;
        let keypair_path = required("KEYPAIR_PATH")?;

        let jito_tip_lamports = env_u64("JITO_TIP_LAMPORTS", 100_000)?;
        let min_liquidity = env_f64("MIN_LIQUIDITY", 50.0)?;

        let grpc_max_inflight = env_usize("GRPC_MAX_INFLIGHT", 256)?;
        let grpc_reconnect_min_ms = env_u64("GRPC_RECONNECT_MIN_MS", 500)?;
        let grpc_reconnect_max_ms = env_u64("GRPC_RECONNECT_MAX_MS", 15_000)?;

        let ipc_listen_host =
            env::var("IPC_LISTEN_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let ipc_listen_port = env_u16("IPC_LISTEN_PORT", 9000)?;
        let ipc_max_queue = env_usize("IPC_MAX_QUEUE", 1024)?;

        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        let wss_url = env::var("WSS_URL").ok();

        Ok(Self {
            rpc_url,
            wss_url,
            grpc_url,
            grpc_token,
            jito_block_engine_url,
            jito_tip_account,
            jito_tip_lamports,
            keypair_path,
            min_liquidity,
            grpc_max_inflight,
            grpc_reconnect_min_ms,
            grpc_reconnect_max_ms,
            ipc_listen_host,
            ipc_listen_port,
            ipc_max_queue,
            log_level,
            shutdown: Shutdown::new(),
        })
    }
}

/* ------------------------------
   Helpers
-------------------------------- */

fn required(key: &str) -> Result<String> {
    env::var(key).with_context(|| format!("{key} missing"))
}

fn env_u64(key: &str, default: u64) -> Result<u64> {
    match env::var(key) {
        Ok(v) => v.parse::<u64>().with_context(|| format!("Invalid {key}")),
        Err(_) => Ok(default),
    }
}

fn env_u16(key: &str, default: u16) -> Result<u16> {
    match env::var(key) {
        Ok(v) => v.parse::<u16>().with_context(|| format!("Invalid {key}")),
        Err(_) => Ok(default),
    }
}

fn env_usize(key: &str, default: usize) -> Result<usize> {
    match env::var(key) {
        Ok(v) => v.parse::<usize>().with_context(|| format!("Invalid {key}")),
        Err(_) => Ok(default),
    }
}

fn env_f64(key: &str, default: f64) -> Result<f64> {
    match env::var(key) {
        Ok(v) => v.parse::<f64>().with_context(|| format!("Invalid {key}")),
        Err(_) => Ok(default),
    }
}
