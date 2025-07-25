use reqwest::Client;

use crate::models::PaymentProcessorRequest;

#[derive(Clone)]
pub struct ProcessorClient {
    client: Client,
}

impl ProcessorClient {
    pub fn new(client: Client) -> ProcessorClient {
        return ProcessorClient { client: client };
    }

    pub async fn capture_default(&self, request: &PaymentProcessorRequest) -> bool {
        let url = "http://payment-processor-default:8080/payments";
        let response = self.client.post(url).json(&request).send().await;
        if let Ok(data) = response {
            return data.status().is_success();
        }
        return false;
    }

    #[allow(dead_code)]
    pub async fn capture_fallback(&self, request: &PaymentProcessorRequest) -> bool {
        let url = "http://payment-processor-fallback:8080/payments";
        let response = self.client.post(url).json(&request).send().await;
        if let Ok(data) = response {
            return data.status().is_success();
        }
        return false;
    }
}
