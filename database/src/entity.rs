use std::sync::Arc;

use chrono::{DateTime, Utc};
use crossbeam_queue::SegQueue;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default)]
pub struct State {
    pub default: Arc<SegQueue<PaymentRequest>>,
    pub fallback: Arc<SegQueue<PaymentRequest>>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequest {
    pub amount: f64,
    pub requested_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SummaryResponse {
    pub default: SummaryOrigin,
    pub fallback: SummaryOrigin,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SummaryOrigin {
    pub total_requests: usize,
    pub total_amount: f64,
}

#[derive(Deserialize)]
pub struct SummaryQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}
