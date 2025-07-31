use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Clone)]
pub struct SummaryQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}
