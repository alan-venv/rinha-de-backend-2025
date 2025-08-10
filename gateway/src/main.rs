mod client;
mod controller;
mod entity;
mod repository;
mod service;
mod vars;

use std::{io::Result, os::unix::fs::PermissionsExt, path::Path};

use actix_web::{App, HttpServer, web::Data};
use mimalloc::MiMalloc;
use reqwest::Client;
use umbral_socket::stream::UmbralClient;

use crate::{
    client::ProcessorClient,
    controller::{payments, payments_summary, purge_payments},
    repository::Repository,
    service::Service,
};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[actix_web::main]
async fn main() -> Result<()> {
    let reqwest = Client::new();
    let umbral = UmbralClient::new("/sockets/database.sock", 16);
    let client = ProcessorClient::new(reqwest.clone());
    let repository = Repository::new(umbral.clone());
    let service = Service::new(client.clone(), repository.clone());

    service.initialize_master_worker();
    service.initialize_slave_workers();
    service.initialize_data_analyst();
    log_vars();

    let path = vars::socket();
    let socket = Path::new(&path);
    if socket.exists() {
        let _ = std::fs::remove_file(socket);
    }

    let server = HttpServer::new(move || {
        App::new()
            .service(payments)
            .service(purge_payments)
            .service(payments_summary)
            .app_data(Data::new(repository.clone()))
            .app_data(Data::new(service.clone()))
    })
    .workers(1)
    .bind_uds(socket)?;

    let permissions = std::fs::Permissions::from_mode(0o766);
    std::fs::set_permissions(socket, permissions)?;

    server.run().await
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
