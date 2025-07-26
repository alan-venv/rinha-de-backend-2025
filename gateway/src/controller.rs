use chrono::{DateTime, Utc};

use crate::{models::SummaryResponse, repository::Repository};

#[derive(Clone)]
pub struct Controller {
    repository: Repository,
}

type Data = Option<DateTime<Utc>>;

impl Controller {
    pub fn new(repository: Repository) -> Controller {
        return Controller {
            repository: repository,
        };
    }

    pub async fn purge_payments(&self) {
        self.repository.purge_payments().await;
    }

    pub async fn get_summary(&self, from: Data, to: Data) -> SummaryResponse {
        return self.repository.get_summary(from, to).await;
    }
}
