use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub v: u16,
    pub id: String,
    pub ts_ms: u64,
    pub kind: Kind,
    pub payload: Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Opportunity,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    Opportunity(Opportunity),
    Ping(Ping),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ping {
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    pub strategy: Strategy,
    pub dex: Dex,
    pub mint: String,

    pub min_liquidity: f64,
    pub max_slippage_bps: u16,
    pub max_delay_ms: u32,

    #[serde(default)]
    pub source_sig: Option<String>,
    #[serde(default)]
    pub pool: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    RaydiumPoolSnipe,
    Backrun,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dex {
    Raydium,
    Orca,
    Meteora,
    Pumpfun,
    Other,
}
