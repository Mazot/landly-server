use super::db::DbPool;
use crate::app::features::organisation::{
    presenters::OrganisationPresenterImpl,
    repositories::OrganisationRepositoryImpl,
    usecases::OrganisationUsecase,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct DiContainer {
    pub organisation_usecase: OrganisationUsecase,
}

impl DiContainer {
    pub fn new(pool: &DbPool) -> Self {
        let organisation_repo = OrganisationRepositoryImpl::new(pool.clone());
        let organisation_presenter = OrganisationPresenterImpl::new();

        Self {
            organisation_usecase: OrganisationUsecase::new(
                Arc::new(organisation_repo.clone()),
                Arc::new(organisation_presenter.clone()),
            ),
        }
    }
}
