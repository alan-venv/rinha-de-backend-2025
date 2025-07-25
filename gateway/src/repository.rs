use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use tokio::time::sleep;
use umbral_socket::UmbralSocket;

use crate::models::{PaymentProcessorRequest, SummaryResponse};

#[derive(Clone)]
pub struct Repository {
    socket: UmbralSocket,
}

type Data = Option<DateTime<Utc>>;

impl Repository {
    pub fn new(socket: UmbralSocket) -> Repository {
        return Repository { socket };
    }

    pub async fn insert_default(&self, request: &PaymentProcessorRequest) {
        let endpoint = "/payments/default";
        let response = self.socket.post_raw(endpoint, request).await;
        if let Err(_) = response {
            println!("Failed to insert payment");
        }
    }

    #[allow(dead_code)]
    pub async fn insert_fallback(&self, request: &PaymentProcessorRequest) {
        let endpoint = "/payments/fallback";
        let response = self.socket.post_raw(endpoint, request).await;
        if let Err(_) = response {
            println!("Failed to insert payment");
        }
    }

    pub async fn purge_payments(&self) {
        let endpoint = "/purge-payments";
        let response = self.socket.post_trigger(endpoint).await;
        if let Err(_) = response {
            println!("Failed to purge payments");
        }
    }

    pub async fn get_summary(&self, from: Data, to: Data) -> SummaryResponse {
        let mut endpoint = String::from("/summary");
        if let (Some(from), Some(to)) = (from, to) {
            endpoint = format!(
                "/summary?from={}&to={}",
                from.to_rfc3339_opts(SecondsFormat::Millis, true),
                to.to_rfc3339_opts(SecondsFormat::Millis, true)
            );
        }
        for _ in 0..3 {
            let response = self.socket.get(&endpoint).await;
            if let Ok(data) = response {
                return data.response.unwrap();
            }
            sleep(Duration::from_millis(100)).await;
        }
        panic!("Deu ruim");
    }
}
