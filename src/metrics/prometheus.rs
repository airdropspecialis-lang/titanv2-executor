use lazy_static::{initialize, lazy_static};
use prometheus::{
    opts, register_counter_vec_with_registry, register_counter_with_registry,
    register_histogram_with_registry, Counter, CounterVec, Histogram, Registry,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    pub static ref OPPORTUNITIES_TOTAL: Counter = register_counter_with_registry!(
        opts!(
            "mev_opportunities_total",
            "Total MEV opportunities detected"
        ),
        &REGISTRY
    )
    .unwrap();
    pub static ref LANDED_TOTAL: Counter = register_counter_with_registry!(
        opts!("mev_landed_total", "Bundles successfully sent"),
        &REGISTRY
    )
    .unwrap();
    pub static ref SIM_FAILURES: CounterVec = register_counter_vec_with_registry!(
        "mev_sim_failures",
        "Simulation failures by reason",
        &["reason"],
        &REGISTRY
    )
    .unwrap();
    pub static ref NET_PROFIT_LAMPORTS: Counter = register_counter_with_registry!(
        opts!("mev_net_profit_lamports", "Total net profit in lamports"),
        &REGISTRY
    )
    .unwrap();
    pub static ref EXECUTION_LATENCY_MS: Histogram = register_histogram_with_registry!(
        "mev_execution_latency_ms",
        "Time from detection to bundle send",
        &REGISTRY
    )
    .unwrap();
}

pub fn init_metrics() {
    initialize(&REGISTRY);
    initialize(&OPPORTUNITIES_TOTAL);
    initialize(&LANDED_TOTAL);
    initialize(&SIM_FAILURES);
    initialize(&NET_PROFIT_LAMPORTS);
    initialize(&EXECUTION_LATENCY_MS);
}
