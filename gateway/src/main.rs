mod client;
mod controller;
mod models;
mod repository;
mod service;
mod utils;

use std::{env, os::unix::fs::PermissionsExt, path::Path};

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, post,
    web::{Data, Json, Query},
};
use mimalloc::MiMalloc;
use reqwest::Client;
use umbral_socket::UmbralClient;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use crate::{
    client::ProcessorClient,
    controller::Controller,
    models::{PaymentRequest, SummaryQuery},
    repository::Repository,
    service::Service,
};

#[post("/payments")]
async fn payments(service: Data<Service>, request: Json<PaymentRequest>) -> impl Responder {
    service.submit(request.0);
    return HttpResponse::Accepted().finish();
}

#[post("/purge-payments")]
async fn purge_payments(controller: Data<Controller>) -> impl Responder {
    controller.purge_payments().await;
    return HttpResponse::Ok().finish();
}

#[get("/payments-summary")]
async fn payments_summary(
    controller: Data<Controller>,
    info: Query<SummaryQuery>,
) -> impl Responder {
    if let (Some(from), Some(to)) = (info.from, info.to) {
        let summary = controller.get_summary_from(from, to).await;
        return HttpResponse::Ok().json(summary);
    }
    let summary = controller.get_summary().await;
    return HttpResponse::Ok().json(summary);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let reqwest = Client::new();
    let umbral_client = UmbralClient::new("/sockets/database.sock");
    let client = ProcessorClient::new(reqwest.clone());
    let repository = Repository::new(reqwest.clone(), umbral_client.clone());
    let controller = Controller::new(repository.clone());
    let service = Service::new(client.clone(), repository.clone());
    println!("VERSION: 5");

    service.initialize_dispatcher();
    service.initialize_workers();

    let path = env::var("SOCKET_PATH").unwrap();
    let socket = Path::new(&path);
    if socket.exists() {
        let _ = std::fs::remove_file(socket);
    }

    let server = HttpServer::new(move || {
        App::new()
            .service(payments)
            .service(purge_payments)
            .service(payments_summary)
            .app_data(Data::new(controller.clone()))
            .app_data(Data::new(service.clone()))
            .app_data(Data::new(umbral_client.clone()))
    })
    .workers(1)
    .bind_uds(socket)?;

    let permissions = std::fs::Permissions::from_mode(0o766);
    std::fs::set_permissions(path, permissions)?;

    server.run().await
}
