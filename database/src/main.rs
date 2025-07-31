use bytes::Bytes;
use chrono::{DateTime, Utc};
use crossbeam_queue::SegQueue;
use mimalloc::MiMalloc;
use serde::{Deserialize, Serialize};
use std::{io::Result, sync::Arc};
use umbral_socket::stream::UmbralServer;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Clone, Default)]
struct State {
    default: Arc<SegQueue<Request>>,
    fallback: Arc<SegQueue<Request>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let state = State::default();

    UmbralServer::new(state)
        .route("SAVE", save)
        .route("PURGE", purge)
        .route("SUMMARY", summary)
        .run("/sockets/database.sock")
        .await
}

async fn save(state: Arc<State>, content: Bytes) -> Result<Bytes> {
    let request: Request = serde_json::from_slice(&content)?;
    state.default.push(request);
    Ok(Bytes::from_static(b"OK"))
}

async fn purge(state: Arc<State>, _: Bytes) -> Result<Bytes> {
    while state.default.pop().is_some() {}
    while state.fallback.pop().is_some() {}
    Ok(Bytes::from_static(b"OK"))
}

async fn summary(state: Arc<State>, content: Bytes) -> Result<Bytes> {
    let query: SummaryQuery = serde_json::from_slice(&content)?;

    let mut default_items = Vec::new();
    while let Some(req) = state.default.pop() {
        default_items.push(req);
    }
    let mut fallback_items = Vec::new();
    while let Some(req) = state.fallback.pop() {
        fallback_items.push(req);
    }

    let default_summary: Origin;
    let fallback_summary: Origin;
    if let (Some(from), Some(to)) = (&query.from, &query.to) {
        let (dtr, dta) = default_items
            .iter()
            .filter(|x| x.requested_at >= *from && x.requested_at <= *to)
            .fold((0, 0.0), |(count, sum), r| (count + 1, sum + r.amount));

        let (ftr, fta) = fallback_items
            .iter()
            .filter(|x| x.requested_at >= *from && x.requested_at <= *to)
            .fold((0, 0.0), |(count, sum), r| (count + 1, sum + r.amount));

        default_summary = Origin::new(dtr, (dta * 100.0).round() / 100.0);
        fallback_summary = Origin::new(ftr, (fta * 100.0).round() / 100.0);
    } else {
        let dta: f64 = default_items.iter().map(|r| r.amount).sum();
        let fta: f64 = fallback_items.iter().map(|r| r.amount).sum();
        default_summary = Origin::new(default_items.len(), (dta * 100.0).round() / 100.0);
        fallback_summary = Origin::new(fallback_items.len(), (fta * 100.0).round() / 100.0);
    }

    for item in default_items {
        state.default.push(item);
    }
    for item in fallback_items {
        state.fallback.push(item);
    }

    let response = Response {
        default: default_summary,
        fallback: fallback_summary,
    };
    Ok(Bytes::from(serde_json::to_vec(&response).unwrap()))
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Request {
    amount: f64,
    requested_at: DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Origin {
    total_requests: usize,
    total_amount: f64,
}

impl Origin {
    fn new(requests: usize, amount: f64) -> Origin {
        return Origin {
            total_requests: requests,
            total_amount: amount,
        };
    }
}

#[derive(Serialize)]
struct Response {
    default: Origin,
    fallback: Origin,
}

#[derive(Deserialize)]
struct SummaryQuery {
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
}
