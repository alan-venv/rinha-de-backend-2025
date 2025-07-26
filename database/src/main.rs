use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use chrono::{DateTime, Utc};
use crossbeam_queue::SegQueue;
use mimalloc::MiMalloc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Request {
    amount: f64,
    requested_at: DateTime<Utc>,
}

async fn insert_default(storage: DefaultStorage, request: web::Json<Request>) -> impl Responder {
    storage.0.push(request.into_inner());
    HttpResponse::Ok()
}

async fn insert_fallback(storage: FallbackStorage, request: web::Json<Request>) -> impl Responder {
    storage.0.push(request.into_inner());
    HttpResponse::Ok()
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
    default: DefaultStorage,
    fallback: FallbackStorage,
    query: web::Query<SummaryQuery>,
) -> impl Responder {
    let mut default_items = Vec::new();
    while let Some(req) = default.0.pop() {
        default_items.push(req);
    }
    let mut fallback_items = Vec::new();
    while let Some(req) = fallback.0.pop() {
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
        default.0.push(item);
    }
    for item in fallback_items {
        fallback.0.push(item);
    }

    let response = Response {
        default: default_summary,
        fallback: fallback_summary,
    };
    return HttpResponse::Ok().json(response);
}

async fn purge_payments(fs: FallbackStorage, ds: DefaultStorage) -> impl Responder {
    while fs.0.pop().is_some() {}
    while ds.0.pop().is_some() {}
    HttpResponse::Ok()
}

#[derive(Clone)]
struct DefaultQueue(Arc<SegQueue<Request>>);

#[derive(Clone)]
struct FallbackQueue(Arc<SegQueue<Request>>);

type DefaultStorage = web::Data<DefaultQueue>;
type FallbackStorage = web::Data<FallbackQueue>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let default: DefaultStorage = web::Data::new(DefaultQueue(Arc::new(SegQueue::new())));
    let fallback: FallbackStorage = web::Data::new(FallbackQueue(Arc::new(SegQueue::new())));

    println!("VERSION: 5.1");
    HttpServer::new(move || {
        App::new()
            .app_data(default.clone())
            .app_data(fallback.clone())
            .route("/summary", web::get().to(summary))
            .route("/payments/default", web::post().to(insert_default))
            .route("/payments/fallback", web::post().to(insert_fallback))
            .route("/purge-payments", web::post().to(purge_payments))
    })
    .bind(("0.0.0.0", 8080))?
    .workers(1)
    .run()
    .await
}
