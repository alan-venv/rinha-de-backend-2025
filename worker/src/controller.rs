use std::io::Result;
use std::sync::Arc;

use bytes::Bytes;

use crate::entity::State;

pub async fn payments(state: Arc<State>, content: Bytes) -> Result<Bytes> {
    state.service.submit(content);
    return Ok(Bytes::new());
}

pub async fn purge_payments(state: Arc<State>, _: Bytes) -> Result<Bytes> {
    state.repository.purge_payments().await;
    return Ok(Bytes::new());
}

pub async fn payments_summary(state: Arc<State>, content: Bytes) -> Result<Bytes> {
    let summary = state.repository.get_summary(content).await;
    return Ok(summary);
}
