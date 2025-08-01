use bytes::Bytes;
use reqwest::Client;

#[derive(Clone)]
pub struct ProcessorClient {
    client: Client,
}

impl ProcessorClient {
    pub fn new(client: Client) -> ProcessorClient {
        return ProcessorClient { client: client };
    }

    pub async fn capture_default(&self, request: Bytes) -> bool {
        let url = "http://payment-processor-default:8080/payments";
        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .body(request)
            .send()
            .await;
        if let Ok(data) = response {
            return data.status().is_success();
        }
        return false;
    }
}
