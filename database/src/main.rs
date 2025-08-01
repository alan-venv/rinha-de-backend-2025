mod controller;
mod entity;

use std::io::Result;

use mimalloc::MiMalloc;
use umbral_socket::stream::UmbralServer;

use crate::controller::{purge, save, summary};
use crate::entity::State;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> Result<()> {
    let state = State::default();

    println!("VERSION: 6.5");

    UmbralServer::new(state)
        .route("SAVE", save)
        .route("PURGE", purge)
        .route("SUMMARY", summary)
        .run("/sockets/database.sock")
        .await
}
