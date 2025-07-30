mod client;
mod controller;
mod models;
mod repository;
mod service;
mod utils;

use std::{env, os::unix::fs::PermissionsExt, path::Path};

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, post,
    web::{Bytes, Data, Query},
};
use mimalloc::MiMalloc;
use reqwest::Client;
use umbral_socket::SocketClient;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use crate::{
    client::ProcessorClient, controller::Controller, models::SummaryQuery, repository::Repository,
    service::Service,
};

#[post("/payments")]
async fn payments(service: Data<Service>, request: Bytes) -> impl Responder {
    service.submit(Bytes::copy_from_slice(&request));
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
    let summary = controller.get_summary(info.from, info.to).await;
    return HttpResponse::Ok().json(summary);
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let reqwest = Client::new();
    let socket_client = SocketClient::new("/sockets/database.sock");
    let client = ProcessorClient::new(reqwest.clone());
    let repository = Repository::new(socket_client.clone());
    let controller = Controller::new(repository.clone());
    let service = Service::new(client.clone(), repository.clone());
    println!("VERSION: 6.4");

    service.initialize_worker();

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
            .app_data(Data::new(socket_client.clone()))
    })
    .workers(1)
    .bind_uds(socket)?;

    let permissions = std::fs::Permissions::from_mode(0o766);
    std::fs::set_permissions(path, permissions)?;

    server.run().await
}
