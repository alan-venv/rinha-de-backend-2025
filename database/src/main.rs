use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
};
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use chrono::{DateTime, Utc};
use crossbeam_queue::SegQueue;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Request {
    amount: f64,
    requested_at: DateTime<Utc>,
}

async fn insert_default(
    State(state): State<AxumState>,
    Json(request): Json<Request>,
) -> StatusCode {
    state.default_storage.push(request);
    return StatusCode::OK;
}

async fn insert_fallback(
    State(state): State<AxumState>,
    Json(request): Json<Request>,
) -> StatusCode {
    state.fallback_sorage.push(request);
    return StatusCode::OK;
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

async fn summary(
    State(state): State<AxumState>,
    Query(query): Query<SummaryQuery>,
) -> (StatusCode, Json<Response>) {
    let mut default_items = Vec::new();
    while let Some(req) = state.default_storage.pop() {
        default_items.push(req);
    }
    let mut fallback_items = Vec::new();
    while let Some(req) = state.fallback_sorage.pop() {
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
        state.default_storage.push(item);
    }
    for item in fallback_items {
        state.fallback_sorage.push(item);
    }

    let response = Response {
        default: default_summary,
        fallback: fallback_summary,
    };
    return (StatusCode::OK, Json(response));
}

async fn purge_payments(State(state): State<AxumState>) -> StatusCode {
    while state.default_storage.pop().is_some() {}
    while state.fallback_sorage.pop().is_some() {}
    return StatusCode::OK;
}

#[derive(Clone)]
struct AxumState {
    default_storage: Arc<SegQueue<Request>>,
    fallback_sorage: Arc<SegQueue<Request>>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let state = AxumState {
        default_storage: Arc::new(SegQueue::new()),
        fallback_sorage: Arc::new(SegQueue::new()),
    };
    println!("VERSION: 2.0");

    let app = Router::new()
        .route("/summary", get(summary))
        .route("/payments/default", post(insert_default))
        .route("/payments/fallback", post(insert_fallback))
        .route("/purge-payments", post(purge_payments))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
