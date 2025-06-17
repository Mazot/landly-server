use super::db::DbPool;
use crate::app::features::organisation::{
    presenters::OrganisationPresenterImpl,
    repositories::OrganisationRepositoryImpl,
    usecases::OrganisationUsecase,
};
use crate::app::features::common::{
    presenters::CommonPresenterImpl,
    repositories::CommonRepositoryImpl,
    usecases::CommonUsecase,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct DiContainer {
    pub organisation_usecase: OrganisationUsecase,
    pub common_usecase: CommonUsecase,
}

impl DiContainer {
    pub fn new(pool: &DbPool) -> Self {
        let organisation_repo = OrganisationRepositoryImpl::new(pool.clone());
        let organisation_presenter = OrganisationPresenterImpl::new();

        let common_repo = CommonRepositoryImpl::new(pool.clone());
        let common_presenter = CommonPresenterImpl::new();

        Self {
            organisation_usecase: OrganisationUsecase::new(
                Arc::new(organisation_repo.clone()),
                Arc::new(organisation_presenter.clone()),
            ),
            common_usecase: CommonUsecase::new(
                Arc::new(common_repo.clone()),
                Arc::new(common_presenter.clone()),
            ),
        }
    }
}
