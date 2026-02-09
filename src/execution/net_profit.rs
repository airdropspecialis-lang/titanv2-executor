use log::{error, warn};

pub fn net_profit_ok(
    pre: u64,
    post: u64,
    input: u64,
    slippage_bps: u16,
    tip: u64,
    fee: u64,
    logs: Option<&Vec<String>>,
) -> bool {
    let delta = post as i128 - pre as i128;

    if delta == 0 {
        warn!("ZERO DELTA");
        return false;
    }

    let min_out = (input as u128 * (10_000 - slippage_bps as u128) / 10_000) as u64;

    if post < min_out {
        warn!("SLIPPAGE FAIL");
        return false;
    }

    if let Some(logs) = logs {
        let bad = ["honeypot", "frozen", "blacklist", "tax"];
        for l in logs {
            if bad.iter().any(|b| l.to_lowercase().contains(b)) {
                error!("HONEYPOT LOG {}", l);
                return false;
            }
        }
    }

    let net = delta - tip as i128 - fee as i128;
    net > 0
}
