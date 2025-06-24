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
use crate::app::features::country_connection::{
    repositories::CountryConnectionRepositoryImpl,
    presenters::CountryConnectionPresenterImpl,
    usecases::CountryConnectionUsecase,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct DiContainer {
    pub organisation_usecase: OrganisationUsecase,
    pub common_usecase: CommonUsecase,
    pub country_connection_usecase: CountryConnectionUsecase,
}

impl DiContainer {
    pub fn new(pool: &DbPool) -> Self {
        let organisation_repo = OrganisationRepositoryImpl::new(pool.clone());
        let organisation_presenter = OrganisationPresenterImpl::new();

        let common_repo = CommonRepositoryImpl::new(pool.clone());
        let common_presenter = CommonPresenterImpl::new();

        let country_connection_repo = CountryConnectionRepositoryImpl::new(pool.clone());
        let country_connection_presenter = CountryConnectionPresenterImpl::new();

        Self {
            organisation_usecase: OrganisationUsecase::new(
                Arc::new(organisation_repo.clone()),
                Arc::new(organisation_presenter.clone()),
            ),
            common_usecase: CommonUsecase::new(
                Arc::new(common_repo.clone()),
                Arc::new(common_presenter.clone()),
            ),
            country_connection_usecase: CountryConnectionUsecase::new(
                Arc::new(country_connection_repo.clone()),
                Arc::new(country_connection_presenter.clone()),
            ),
        }
    }
}
