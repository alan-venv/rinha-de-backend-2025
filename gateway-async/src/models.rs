use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequest {
    pub correlation_id: Uuid,
    pub amount: f64,
}

impl PaymentRequest {
    pub fn to_processor(&self) -> PaymentProcessorRequest {
        PaymentProcessorRequest {
            correlation_id: self.correlation_id.clone(),
            amount: self.amount,
            requested_at: Utc::now(),
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PaymentProcessorRequest {
    pub correlation_id: Uuid,
    pub amount: f64,
    pub requested_at: DateTime<Utc>,
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
