use std::sync::Arc;

use chrono::{DateTime, Utc};
use crossbeam_queue::SegQueue;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default)]
pub struct State {
    pub default: Arc<SegQueue<Request>>,
    pub fallback: Arc<SegQueue<Request>>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub amount: f64,
    pub requested_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct Response {
    pub default: Origin,
    pub fallback: Origin,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Origin {
    pub total_requests: usize,
    pub total_amount: f64,
}

impl Origin {
    pub fn new(requests: usize, amount: f64) -> Origin {
        return Origin {
            total_requests: requests,
            total_amount: amount,
        };
    }
}

#[derive(Deserialize)]
pub struct SummaryQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}
