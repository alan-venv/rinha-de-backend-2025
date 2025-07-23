use reqwest::blocking::Client;

use crate::models::PaymentProcessorRequest;

#[derive(Clone)]
pub struct Repository {
    client: Client,
}

impl Repository {
    pub fn new(sync: reqwest::blocking::Client) -> Repository {
        return Repository { client: sync };
    }

    pub fn insert_default_sync(&self, request: &PaymentProcessorRequest) {
        let url = "http://rinha-db:8080/payments/default";
        let response = self.client.post(url).json(request).send();
        if let Err(_) = response {
            println!("FAILED TO INSERT PAYMENT");
        }
    }

    #[allow(dead_code)]
    pub async fn insert_fallback_sync(&self, request: &PaymentProcessorRequest) {
        let url = "http://rinha-db:8080/payments/fallback";
        let response = self.client.post(url).json(request).send();
        if let Err(_) = response {
            println!("FAILED TO INSERT PAYMENT");
        }
    }
}
