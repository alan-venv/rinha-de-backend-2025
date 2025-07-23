use reqwest::blocking::Client;

use crate::models::PaymentProcessorRequest;

#[derive(Clone)]
pub struct ProcessorClient {
    client: Client,
}

impl ProcessorClient {
    pub fn new(client: Client) -> ProcessorClient {
        return ProcessorClient { client: client };
    }

    pub fn capture_default_sync(&self, request: &PaymentProcessorRequest) -> bool {
        let url = "http://payment-processor-default:8080/payments";
        let response = self.client.post(url).json(&request).send();
        match response {
            Ok(data) => {
                return data.status().is_success();
            }
            Err(_) => return false,
        }
    }

    #[allow(dead_code)]
    pub fn capture_fallback_sync(&self, request: &PaymentProcessorRequest) -> bool {
        let url = "http://payment-processor-fallback:8080/payments";
        let response = self.client.post(url).json(&request).send();
        match response {
            Ok(data) => {
                return data.status().is_success();
            }
            Err(_) => return false,
        }
    }
}
