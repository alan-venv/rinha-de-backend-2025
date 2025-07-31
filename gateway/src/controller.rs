use actix_web::{
    HttpResponse, Responder, get, post,
    web::{Bytes, Data, Query},
};

use crate::{entity::SummaryQuery, repository::Repository, service::Service};

#[post("/payments")]
pub async fn payments(service: Data<Service>, request: Bytes) -> impl Responder {
    service.submit(Bytes::copy_from_slice(&request));
    return HttpResponse::Accepted().finish();
}

#[post("/purge-payments")]
pub async fn purge_payments(repository: Data<Repository>) -> impl Responder {
    repository.purge_payments().await;
    return HttpResponse::Ok().finish();
}

#[get("/payments-summary")]
pub async fn payments_summary(
    repository: Data<Repository>,
    info: Query<SummaryQuery>,
) -> impl Responder {
    let summary = repository.get_summary(info.into_inner()).await;
    return HttpResponse::Ok().body(summary);
}
