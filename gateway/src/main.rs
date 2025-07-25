mod client;
mod controller;
mod models;
mod repository;
mod service;
mod utils;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
};
use mimalloc::MiMalloc;
use reqwest::Client;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use crate::{
    client::ProcessorClient,
    controller::Controller,
    models::{PaymentRequest, SummaryQuery, SummaryResponse},
    repository::Repository,
    service::Service,
};

async fn payments(
    State(state): State<AxumState>,
    Json(request): Json<PaymentRequest>,
) -> StatusCode {
    state.service.submit(request);
    return StatusCode::ACCEPTED;
}

async fn purge_payments(State(state): State<AxumState>) -> StatusCode {
    state.controller.purge_payments().await;
    return StatusCode::OK;
}

async fn payments_summary(
    State(state): State<AxumState>,
    Query(info): Query<SummaryQuery>,
) -> (StatusCode, Json<SummaryResponse>) {
    if let (Some(from), Some(to)) = (info.from, info.to) {
        let summary = state.controller.get_summary_from(from, to).await;
        return (StatusCode::OK, Json(summary));
    }
    let summary = state.controller.get_summary().await;
    return (StatusCode::OK, Json(summary));
}

#[derive(Clone)]
struct AxumState {
    controller: Controller,
    service: Service,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let reqwest = Client::new();
    let client = ProcessorClient::new(reqwest.clone());
    let repository = Repository::new(reqwest.clone());
    let controller = Controller::new(repository.clone());
    let service = Service::new(client.clone(), repository.clone());
    println!("VERSION: 4.0");

    service.initialize_dispatcher();
    service.initialize_workers();

    let state = AxumState {
        controller: controller,
        service: service,
    };

    let app = Router::new()
        .route("/payments", post(payments))
        .route("/purge-payments", post(purge_payments))
        .route("/payments-summary", get(payments_summary))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    // Obs: Actix Web SEMPRE inicializa no mínimo 3 threads de SO causando muito overhead de troca de contexto já que a aplicação não tem 3 núcleos de CPU.
    // Por isso que, para este desafio especifico, trocar para Axum e deixar o Tokio fazer sua mágica dentro de uma única thread de SO é extremamente necessário.
}
