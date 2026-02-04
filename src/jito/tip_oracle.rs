use std::sync::atomic::{AtomicU64, Ordering};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

pub struct TipOracle {
    tip_lamports: AtomicU64,
}

impl TipOracle {
    pub fn new(default: u64) -> Self {
        Self {
            tip_lamports: AtomicU64::new(default),
        }
    }

    #[inline(always)]
    pub fn get(&self) -> u64 {
        self.tip_lamports.load(Ordering::Relaxed)
    }

    pub async fn run(self: &'static Self, client: Client, base_url: String) {
        loop {
            if let Ok(resp) = client
                .get(format!("{}/api/v1/bundles/tip_floor", base_url))
                .send()
                .await
            {
                if let Ok(json) = resp.json::<Vec<Value>>().await {
                    if let Some(v) = json.first() {
                        if let Some(p50) = v["ema_landed_50th_percentile"].as_u64() {
                            let aggressive = (p50 as f64 * 1.05) as u64;
                            self.tip_lamports.store(aggressive, Ordering::Relaxed);
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(1500)).await;
        }
    }
}
