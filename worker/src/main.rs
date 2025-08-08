mod client;
mod controller;
mod entity;
mod repository;
mod service;
mod vars;

use std::io::Result;

use mimalloc::MiMalloc;
use reqwest::Client;
use umbral_socket::stream::UmbralServer;

use crate::{
    client::ProcessorClient,
    controller::{payments_summary, purge_payments},
    entity::State,
    repository::Repository,
    service::Service,
};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> Result<()> {
    let reqwest = Client::new();
    let client = ProcessorClient::new(reqwest.clone());
    let repository = Repository::new();
    let service = Service::new(client.clone(), repository.clone());

    service.initialize_master_worker();
    service.initialize_slave_workers();
    service.initialize_data_analyst();
    service.initialize_poller();
    log_vars();

    let state = State::new(repository, service);
    let socket = vars::socket();
    UmbralServer::new(state)
        .route("PURGE", purge_payments)
        .route("SUMMARY", payments_summary)
        .run(&socket)
        .await
}

fn log_vars() {
    let trigger = vars::trigger();
    let slaves = vars::slaves();
    let analyst = vars::analyst();
    println!("VERSION: 7.0 SKYLAKE");
    println!("TRIGGER: {}", trigger);
    println!("SLAVES: {}", slaves);
    println!("ANALYST: {}", analyst);
}
