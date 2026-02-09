use log::*;
use jito_protos::searcher::searcher_service_client::SearcherServiceClient;
use jito_protos::searcher::{
    GetNextScheduledLeaderRequest, GetNextScheduledLeaderResponse,
    SubscribeBundleResultsRequest, SendBundleRequest, SendBundleResponse,
};
use jito_protos::bundle::{Bundle, BundleResult, Accepted, SimulationFailure, InternalError};
use jito_protos::bundle::bundle_result::Result as BundleResultType;
use jito_protos::bundle::rejected::Reason;
use std::time::Duration;
use tokio::time::Instant;
use tonic::transport::Channel;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct SearcherClient {
    client: SearcherServiceClient<Channel>,
}

impl SearcherClient {
    pub fn new(client: SearcherServiceClient<Channel>) -> Self {
        Self { client }
    }

    pub async fn subscribe_bundle_results(&mut self) -> Result<tonic::Response<tonic::Streaming<BundleResult>>> {
        let request = SubscribeBundleResultsRequest {};
        self.client.subscribe_bundle_results(request).await.map_err(|e| e.into())
    }

    pub async fn get_next_scheduled_leader(&mut self) -> Result<GetNextScheduledLeaderResponse> {
        let request = GetNextScheduledLeaderRequest { regions: vec![] };
        self.client.get_next_scheduled_leader(request).await.map(|r| r.into_inner()).map_err(|e| e.into())
    }

    pub async fn send_bundle(&mut self, bundle: Bundle) -> Result<SendBundleResponse> {
        let request = SendBundleRequest { bundle: Some(bundle) };
        self.client.send_bundle(request).await.map(|r| r.into_inner()).map_err(|e| e.into())
    }
}

pub async fn wait_for_bundle_results(
    mut stream: tonic::Streaming<BundleResult>,
    _bundle_signatures: Vec<String>,
    timeout: Duration,
) -> Result<()> {
    let mut time_left = timeout.as_millis() as u64;
    while time_left > 0 {
        let instant = Instant::now();
        match tokio::time::timeout(Duration::from_millis(time_left), stream.message()).await {
            Ok(Ok(Some(bundle_result))) => {
                let bundle_id = bundle_result.bundle_id.clone();
                match bundle_result.result {
                    Some(BundleResultType::Accepted(Accepted { slot, validator_identity })) => {
                        info!("bundle {} accepted in slot {}, validator: {}", bundle_id, slot, validator_identity);
                        return Ok(());
                    }
                    Some(BundleResultType::Rejected(rejected)) => {
                        match rejected.reason {
                            Some(Reason::SimulationFailure(SimulationFailure { tx_signature, msg })) => {
                                info!("bundle {} simulation failure on tx {}, msg: {:?}", bundle_id, tx_signature, msg);
                            }
                            Some(Reason::InternalError(InternalError { msg })) => {
                                info!("bundle {} internal error, msg: {:?}", bundle_id, msg);
                            }
                            _ => {
                                info!("bundle {} rejected for other reasons", bundle_id);
                            }
                        }
                    }
                    None => {}
                }
            }
            _ => break,
        }
        time_left = time_left.saturating_sub(instant.elapsed().as_millis() as u64);
    }
    Err("Timeout waiting for bundle results".into())
}
