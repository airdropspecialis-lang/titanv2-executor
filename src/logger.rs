use env_logger::{Builder, Env};
use log::LevelFilter;
use std::io::Write;

pub fn init() {
    // Respect RUST_LOG if set, otherwise fall back to LOG_LEVEL or "info"
    let env = Env::default()
        .filter_or("RUST_LOG", std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()));

    let mut builder = Builder::from_env(env);

    builder
        .format(|buf, record| {
            let ts = buf.timestamp_millis();
            writeln!(
                buf,
                "{} {:<5} [{}] {}",
                ts,
                record.level(),
                record.target(),
                record.args()
            )
        })
        .filter_level(LevelFilter::Info)
        .try_init()
        .ok(); // prevent panic on double init
}
