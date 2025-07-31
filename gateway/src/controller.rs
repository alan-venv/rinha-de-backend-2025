use bytes::Bytes;

use crate::{models::SummaryQuery, repository::Repository};

#[derive(Clone)]
pub struct Controller {
    repository: Repository,
}

impl Controller {
    pub fn new(repository: Repository) -> Controller {
        return Controller {
            repository: repository,
        };
    }

    pub async fn purge_payments(&self) {
        self.repository.purge_payments().await;
    }

    pub async fn get_summary(&self, query: SummaryQuery) -> Bytes {
        return self.repository.get_summary(query).await;
    }
}
