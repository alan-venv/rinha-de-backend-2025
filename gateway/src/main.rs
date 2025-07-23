mod client;
mod controller;
mod models;
mod repository;
mod service;
mod utils;

use actix_web::{
    App, HttpResponse, HttpServer, Responder, get, post,
    web::{Data, Json, Query},
};
use reqwest::Client;

use crate::{
    controller::Controller,
    models::{PaymentRequest, SummaryQuery},
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
    let client = Client::new();
    let controller = Controller::new(client.clone());
    let service = Service::new();

    service.initialize_dispatcher();
    service.initialize_workers();

    HttpServer::new(move || {
        App::new()
            .service(payments)
            .service(purge_payments)
            .service(payments_summary)
            .app_data(Data::new(controller.clone()))
            .app_data(Data::new(service.clone()))
    })
    .bind(("0.0.0.0", 8080))?
    .workers(1)
    .run()
    .await
}
