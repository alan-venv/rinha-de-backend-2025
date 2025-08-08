use std::sync::Arc;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use crossbeam_queue::SegQueue;

use crate::entity::{PaymentRequest, SummaryOrigin, SummaryResponse};

#[derive(Clone)]
pub struct Repository {
    default: Arc<SegQueue<PaymentRequest>>,
    fallback: Arc<SegQueue<PaymentRequest>>,
}

impl Repository {
    pub fn new() -> Repository {
        return Repository {
            default: Arc::new(SegQueue::new()),
            fallback: Arc::new(SegQueue::new()),
        };
    }

    pub async fn insert_default(&self, content: Bytes) {
        let request: PaymentRequest = serde_json::from_slice(&content).unwrap();
        self.default.push(request);
    }

    pub async fn purge_payments(&self) {
        while self.default.pop().is_some() {}
        while self.fallback.pop().is_some() {}
    }

    pub async fn get_summary(&self, content: Bytes) -> Bytes {
        let (from, to) = parse_params(&content);

        let mut default_items = Vec::new();
        while let Some(req) = self.default.pop() {
            default_items.push(req);
        }
        let mut fallback_items = Vec::new();
        while let Some(req) = self.fallback.pop() {
            fallback_items.push(req);
        }

        let default_summary: SummaryOrigin;
        let fallback_summary: SummaryOrigin;
        if let (Some(from), Some(to)) = (from, to) {
            let (dtr, dta) = default_items
                .iter()
                .filter(|x| x.requested_at >= from && x.requested_at <= to)
                .fold((0, 0.0), |(count, sum), r| (count + 1, sum + r.amount));

            let (ftr, fta) = fallback_items
                .iter()
                .filter(|x| x.requested_at >= from && x.requested_at <= to)
                .fold((0, 0.0), |(count, sum), r| (count + 1, sum + r.amount));

            default_summary = SummaryOrigin {
                total_requests: dtr,
                total_amount: (dta * 100.0).round() / 100.0,
            };
            fallback_summary = SummaryOrigin {
                total_requests: ftr,
                total_amount: (fta * 100.0).round() / 100.0,
            };
        } else {
            let dta: f64 = default_items.iter().map(|r| r.amount).sum();
            let fta: f64 = fallback_items.iter().map(|r| r.amount).sum();
            default_summary = SummaryOrigin {
                total_requests: default_items.len(),
                total_amount: (dta * 100.0).round() / 100.0,
            };
            fallback_summary = SummaryOrigin {
                total_requests: fallback_items.len(),
                total_amount: (fta * 100.0).round() / 100.0,
            };
        }

        for item in default_items {
            self.default.push(item);
        }
        for item in fallback_items {
            self.fallback.push(item);
        }

        let response = SummaryResponse {
            default: default_summary,
            fallback: fallback_summary,
        };
        Bytes::from(serde_json::to_vec(&response).unwrap())
    }
}

fn parse_params(input: &[u8]) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
    let s = match str::from_utf8(input) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };

    let mut from = None;
    let mut to = None;

    for kv in s.split('&') {
        if kv.len() < 4 {
            continue;
        }

        unsafe {
            if kv.get_unchecked(0..5) == "from=" {
                from = DateTime::parse_from_rfc3339(kv.get_unchecked(5..))
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc));
            } else if kv.get_unchecked(0..3) == "to=" {
                to = DateTime::parse_from_rfc3339(kv.get_unchecked(3..))
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc));
            }
        }
    }

    (from, to)
}
