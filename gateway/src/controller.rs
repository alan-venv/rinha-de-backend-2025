use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::Client;
use tokio::time::sleep;

use crate::models::SummaryResponse;

#[derive(Clone)]
pub struct Controller {
    client: Client,
}

impl Controller {
    pub fn new(client: Client) -> Controller {
        return Controller { client: client };
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

    pub async fn get_summary_from(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> SummaryResponse {
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
