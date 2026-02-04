use warp::Filter;
use prometheus::Encoder;
use crate::metrics::REGISTRY;

pub async fn start_metrics() {
    let route = warp::path("metrics").map(|| {
        let mut buf = Vec::new();
        let enc = prometheus::TextEncoder::new();
        enc.encode(&REGISTRY.gather(), &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    });

    warp::serve(route).run(([0,0,0,0], 9091)).await;
}
