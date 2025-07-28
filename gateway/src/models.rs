use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequest {
    pub correlation_id: Uuid,
    pub amount: f64,
    pub requested_at: Option<DateTime<Utc>>,
}

impl PaymentRequest {
    pub fn insert_date(&mut self) {
        self.requested_at = Some(Utc::now())
    }

    pub fn remove_date(&mut self) {
        self.requested_at = None
    }
}

#[derive(Serialize, Deserialize)]
pub struct SummaryResponse {
    pub default: SummaryOrigin,
    pub fallback: SummaryOrigin,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryOrigin {
    pub total_requests: usize,
    pub total_amount: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceHealthResponse {
    pub failing: bool,
    pub min_response_time: usize,
}

#[derive(Deserialize)]
pub struct SummaryQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}
