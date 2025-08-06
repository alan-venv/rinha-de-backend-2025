use crate::{repository::Repository, service::Service};

#[derive(Clone)]
pub struct State {
    pub repository: Repository,
    pub service: Service,
}

impl State {
    pub fn new(repository: Repository, service: Service) -> State {
        return State {
            repository,
            service,
        };
    }
}
