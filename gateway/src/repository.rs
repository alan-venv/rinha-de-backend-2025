use bytes::Bytes;
use umbral_socket::stream::UmbralAsyncClient;

use crate::entity::SummaryQuery;

#[derive(Clone)]
pub struct Repository {
    socket: UmbralAsyncClient,
}

impl Repository {
    pub fn new(socket: UmbralAsyncClient) -> Repository {
        return Repository { socket };
    }

    pub async fn insert_default(&self, request: Bytes) {
        let response = self.socket.send("SAVE", request).await;
        if let Err(_) = response {
            println!("Failed to insert payment");
        }
    }

    pub async fn purge_payments(&self) {
        let response = self.socket.send("PURGE", Bytes::new()).await;
        if let Err(_) = response {
            println!("Failed to purge payments");
        }
    }

    pub async fn get_summary(&self, query: SummaryQuery) -> Bytes {
        let conent = Bytes::from(serde_json::to_vec(&query).unwrap());
        return self
            .socket
            .send("SUMMARY", conent)
            .await
            .unwrap_or(Bytes::new());
    }
}
