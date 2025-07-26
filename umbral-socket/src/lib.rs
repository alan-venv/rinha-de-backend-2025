use http_body_util::BodyExt;
use http_body_util::Empty;
use http_body_util::Full;
use http_body_util::combinators::BoxBody;
use hyper::Method;
use hyper::Request;
use hyper::StatusCode;
use hyper::body::{Buf, Bytes};
use hyper::header;
use hyper_util::client::legacy::Client;
use hyper_util::rt::tokio::TokioExecutor;
use hyperlocal::UnixConnector;
use hyperlocal::Uri;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::convert::Infallible;
use std::env;

// pub async fn post_body<T: Serialize, U: DeserializeOwned>(
//     socket: &str,
//     request: &T,
// ) -> Result<U, ()> {
//     let connector = UnixConnector;
//     let client: Client<UnixConnector, Empty<Bytes>> =
//         Client::builder(TokioExecutor::new()).build(connector);

//     let socket_path = env::var("SOCKET_PATH").unwrap();
//     let endpoint_path = "/dados";

//     let uri = hyperlocal::Uri::new(&socket_path, endpoint_path).into();

//     let res = client.get(uri).await;

//     if let Ok(data) = res {
//         let body_bytes = data.into_body().collect().await;
//         if let Ok(body) = body_bytes {
//             let data: Result<U, serde_json::Error> =
//                 serde_json::from_reader(body.to_bytes().reader());
//             if let Ok(response) = data {
//                 return Ok(response);
//             }
//         }
//     }
//     return Err(());
// }

pub async fn post_body<T: Serialize>(
    socket: &str,
    endpoint: &str,
    request: &T,
) -> Result<StatusCode, ()> {
    let connector = UnixConnector;

    let body_bytes = serde_json::to_vec(&*request).unwrap();
    let request_body = Full::new(Bytes::from(body_bytes));

    let client: Client<UnixConnector, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build(connector);

    let request = Request::builder()
        .method(Method::POST)
        .uri(Uri::new(socket, endpoint))
        .header(header::CONTENT_TYPE, "application/json")
        .body(request_body)
        .unwrap();

    let response = client.request(request).await.unwrap();
    return Ok(response.status());
}

pub async fn post_body_res<T: Serialize, U: DeserializeOwned>(
    socket: &str,
    endpoint: &str,
    request: &T,
) -> Result<U, ()> {
    let connector = UnixConnector;

    let body_bytes = serde_json::to_vec(&*request).unwrap();
    let request_body = Full::new(Bytes::from(body_bytes));

    let client: Client<UnixConnector, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build(connector);

    let request = Request::builder()
        .method(Method::POST)
        .uri(Uri::new(socket, endpoint))
        .header(header::CONTENT_TYPE, "application/json")
        .body(request_body)
        .unwrap();

    let response = client.request(request).await.unwrap();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let data: U = serde_json::from_reader(body_bytes.reader()).unwrap();
    return Ok(data);
}

// #
//

pub struct UmbralResponse<T: DeserializeOwned> {
    pub status: u16,
    pub response: Option<T>,
}

impl<T: DeserializeOwned> UmbralResponse<T> {
    fn new(status: u16, response: Option<T>) -> Self {
        return UmbralResponse {
            status: status,
            response: response,
        };
    }
}

use std::error::Error;

pub async fn post<T: Serialize, U: DeserializeOwned>(
    socket: &str,
    endpoint: &str,
    request: &T,
) -> Result<UmbralResponse<U>, Box<dyn Error>> {
    let connector = UnixConnector;

    let body_bytes = serde_json::to_vec(&*request).unwrap();
    let request_body = Full::new(Bytes::from(body_bytes));

    let client: Client<UnixConnector, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build(connector);

    let request = Request::builder()
        .method(Method::POST)
        .uri(Uri::new(socket, endpoint))
        .header(header::CONTENT_TYPE, "application/json")
        .body(request_body)
        .unwrap();

    let response = client.request(request).await?;
    let status = response.status().as_u16();
    let body_bytes = response.into_body().collect().await?.to_bytes();
    let data: U = serde_json::from_reader(body_bytes.reader())?;
    return Ok(UmbralResponse::new(status, Some(data)));
}

type BoxedBody = BoxBody<Bytes, Infallible>;
type HyperClient = Client<UnixConnector, BoxedBody>;
#[derive(Clone)]
pub struct UmbralClient {
    socket: String,
    client: HyperClient,
}

impl UmbralClient {
    pub fn new(socket: &str) -> UmbralClient {
        let connector = UnixConnector;
        let client: HyperClient = Client::builder(TokioExecutor::new()).build(connector);
        return UmbralClient {
            socket: String::from(socket),
            client: client,
        };
    }

    pub async fn post_raw<T: Serialize>(
        &self,
        endpoint: &str,
        request: &T,
    ) -> Result<UmbralResponse<()>, Box<dyn Error>> {
        let body_bytes = serde_json::to_vec(&*request)?;
        let request_body = Full::new(Bytes::from(body_bytes))
            .map_err(|e| match e {})
            .boxed();

        let request = Request::builder()
            .method(Method::POST)
            .uri(Uri::new(&self.socket, endpoint))
            .header(header::CONTENT_TYPE, "application/json")
            .body(request_body)
            .unwrap();

        let response = self.client.request(request).await?;
        let status = response.status().as_u16();
        return Ok(UmbralResponse::new(status, None));
    }

    pub async fn post_trigger(&self, endpoint: &str) -> Result<UmbralResponse<()>, Box<dyn Error>> {
        let request_body = Empty::new().map_err(|e| match e {}).boxed();
        let request = Request::builder()
            .method(Method::POST)
            .uri(Uri::new(&self.socket, endpoint))
            .header(header::CONTENT_TYPE, "application/json")
            .body(request_body)
            .unwrap();

        let response = self.client.request(request).await?;
        let status = response.status().as_u16();
        return Ok(UmbralResponse::new(status, None));
    }

    pub async fn get<T: Serialize>(
        &self,
        endpoint: &str,
    ) -> Result<UmbralResponse<()>, Box<dyn Error>> {
        let uri = hyperlocal::Uri::new(&self.socket, endpoint).into();

        let response = self.client.get(uri).await?;
        let status = response.status().as_u16();
        return Ok(UmbralResponse::new(status, None));
    }
}
