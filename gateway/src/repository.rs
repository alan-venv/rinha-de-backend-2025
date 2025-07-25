use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::Client;
use tokio::time::sleep;

use crate::models::{PaymentProcessorRequest, SummaryResponse};

#[derive(Clone)]
pub struct Repository {
    client: Client,
}
type Data = DateTime<Utc>;

impl Repository {
    pub fn new(client: Client) -> Repository {
        return Repository { client };
    }

    pub async fn insert_default(&self, request: &PaymentProcessorRequest) {
        let url = "http://rinha-db:8080/payments/default";
        let response = self.client.post(url).json(request).send().await;
        if let Err(_) = response {
            println!("FAILED TO INSERT PAYMENT");
        }
    }

    #[allow(dead_code)]
    pub async fn insert_fallback(&self, request: &PaymentProcessorRequest) {
        let url = "http://rinha-db:8080/payments/fallback";
        let response = self.client.post(url).json(request).send().await;
        if let Err(_) = response {
            println!("FAILED TO INSERT PAYMENT");
        }
    }

    pub async fn purge_payments(&self) {
        let url = "http://rinha-db:8080/purge-payments";
        let response = self.client.post(url).send().await;
        if let Err(_) = response {
            println!("FAILED TO PURGE PAYMENTS");
        }
    }

    pub async fn get_summary(&self) -> SummaryResponse {
        let url = "http://rinha-db:8080/summary";
        for _ in 0..3 {
            let response = self.client.get(url).send().await;
            if let Ok(data) = response {
                if data.status().is_success() {
                    if let Ok(summary) = data.json::<SummaryResponse>().await {
                        return summary;
                    }
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
        panic!("Deu ruim");
    }

    pub async fn get_summary_from(&self, from: Data, to: Data) -> SummaryResponse {
        let url = "http://rinha-db:8080/summary";
        for _ in 0..3 {
            let query = &[("from", from), ("to", to)];
            let response = self.client.get(url).query(query).send().await;
            if let Ok(data) = response {
                if data.status().is_success() {
                    if let Ok(summary) = data.json::<SummaryResponse>().await {
                        return summary;
                    }
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
        panic!("Deu ruim");
    }
}
