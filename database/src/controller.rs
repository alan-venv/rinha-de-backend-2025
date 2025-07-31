use std::{io::Result, sync::Arc};

use bytes::Bytes;

use crate::entity::{PaymentRequest, State, SummaryOrigin, SummaryQuery, SummaryResponse};

pub async fn save(state: Arc<State>, content: Bytes) -> Result<Bytes> {
    let request: PaymentRequest = serde_json::from_slice(&content)?;
    state.default.push(request);
    Ok(Bytes::from_static(b"OK"))
}

pub async fn purge(state: Arc<State>, _: Bytes) -> Result<Bytes> {
    while state.default.pop().is_some() {}
    while state.fallback.pop().is_some() {}
    Ok(Bytes::from_static(b"OK"))
}

pub async fn summary(state: Arc<State>, content: Bytes) -> Result<Bytes> {
    let query: SummaryQuery = serde_json::from_slice(&content)?;

    let mut default_items = Vec::new();
    while let Some(req) = state.default.pop() {
        default_items.push(req);
    }
    let mut fallback_items = Vec::new();
    while let Some(req) = state.fallback.pop() {
        fallback_items.push(req);
    }

    let default_summary: SummaryOrigin;
    let fallback_summary: SummaryOrigin;
    if let (Some(from), Some(to)) = (&query.from, &query.to) {
        let (dtr, dta) = default_items
            .iter()
            .filter(|x| x.requested_at >= *from && x.requested_at <= *to)
            .fold((0, 0.0), |(count, sum), r| (count + 1, sum + r.amount));

        let (ftr, fta) = fallback_items
            .iter()
            .filter(|x| x.requested_at >= *from && x.requested_at <= *to)
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
        state.default.push(item);
    }
    for item in fallback_items {
        state.fallback.push(item);
    }

    let response = SummaryResponse {
        default: default_summary,
        fallback: fallback_summary,
    };
    Ok(Bytes::from(serde_json::to_vec(&response).unwrap()))
}
