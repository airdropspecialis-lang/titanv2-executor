use jito_protos::searcher::searcher_service_client::SearcherServiceClient;
use jito_protos::searcher::{SendBundleRequest, SendBundleResponse};
use jito_protos::bundle::Bundle;
use tonic::transport::Channel;
use solana_sdk::signature::Keypair;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct SearcherClient {
    pub client: SearcherServiceClient<Channel>,
}

pub async fn get_searcher_client(url: &str, _keypair: &Keypair) -> Result<SearcherClient> {
    let endpoint = tonic::transport::Endpoint::from_shared(url.to_string())?;
    let channel = endpoint.connect().await?;
    let client = SearcherServiceClient::new(channel);
    Ok(SearcherClient { client })
}

impl SearcherClient {
    pub async fn send_bundle(&mut self, bundle: Bundle) -> Result<SendBundleResponse> {
        let request = SendBundleRequest { bundle: Some(bundle) };
        let response = self.client.send_bundle(request).await?;
        Ok(response.into_inner())
    }
}
