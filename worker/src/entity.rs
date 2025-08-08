use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{repository::Repository, service::Service};

#[derive(Clone)]
pub struct State {
    pub repository: Repository,
    pub service: Service,
}

impl State {
    pub fn new(repository: Repository, service: Service) -> State {
        return State {
            repository,
            service,
        };
    }
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
