use crate::metrics::prometheus::REGISTRY;
use prometheus::{Encoder, TextEncoder};
use warp::Filter;

pub async fn start_metrics() {
    let metrics_route = warp::path("metrics").and(warp::get()).map(|| {
        let encoder = TextEncoder::new();
        let metric_families = REGISTRY.gather();

        let mut buffer = Vec::new();
        if encoder.encode(&metric_families, &mut buffer).is_err() {
            return warp::reply::with_status(
                "failed to encode metrics".to_string(),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            );
        }

        let body = String::from_utf8(buffer).unwrap_or_default();
        warp::reply::with_status(body, warp::http::StatusCode::OK)
    });

    warp::serve(metrics_route).run(([0, 0, 0, 0], 9091)).await;
}
