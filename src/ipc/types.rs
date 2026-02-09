use rkyv::{Archive, Deserialize, Serialize};

pub const IPC_SCHEMA_VERSION: u32 = 1;

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct Envelope {
    pub v: u32,
    pub id: u64,
    pub ttl_ms: u32,
    pub sent_at_ms: u32,
    pub payload: Payload,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
#[archive(check_bytes)]
pub enum Payload {
    Ping(u64),
    Opportunity(Opportunity),
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy)]
#[archive(check_bytes)]
pub enum Strategy {
    RaydiumPoolSnipe,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone, Copy)]
#[archive(check_bytes)]
pub enum Dex {
    Raydium,
    Pumpfun,
    Orca,
    Meteora,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
#[archive(check_bytes)]
pub struct Opportunity {
    pub strategy: Strategy,
    pub dex: Dex,
    pub mint: [u8; 32],
    pub min_liquidity_lamports: u64,
    pub max_slippage_bps: u32,
    pub max_delay_ms: u32,
    pub suggested_size_lamports: Option<u64>,
}
