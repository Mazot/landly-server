use super::{
    presenters::CorridorPresenter,
    repositories::{CorridorRepository, CreateCorridorRepositoryInput},
};
use crate::error::AppError;
use actix_web::HttpResponse;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct CorridorUsecase {
    corridor_repo: Arc<dyn CorridorRepository>,
    corridor_presenter: Arc<dyn CorridorPresenter>,
}

impl CorridorUsecase {
    pub fn new(
        corridor_repo: Arc<dyn CorridorRepository>,
        corridor_presenter: Arc<dyn CorridorPresenter>,
    ) -> Self {
        Self {
            corridor_repo,
            corridor_presenter,
        }
    }

    pub fn create_corridor(
        &self,
        params: CreateCorridorUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        let corridor = self
            .corridor_repo
            .create_corridor(CreateCorridorRepositoryInput {
                user_id: params.user_id,
                from_country_id: params.from_country_id,
                to_country_id: params.to_country_id,
                is_default: params.is_default,
            })?;

        Ok(self.corridor_presenter.to_single_json(corridor))
    }

    pub fn list_corridors(&self, user_id: Uuid) -> Result<HttpResponse, AppError> {
        let corridors = self.corridor_repo.list_corridors(user_id)?;

        Ok(self.corridor_presenter.to_multi_json(corridors))
    }

    pub fn set_default_corridor(
        &self,
        corridor_id: Uuid,
        user_id: Uuid,
    ) -> Result<HttpResponse, AppError> {
        let corridor = self
            .corridor_repo
            .set_default_corridor(corridor_id, user_id)?;

        Ok(self.corridor_presenter.to_single_json(corridor))
    }

    pub fn delete_corridor(
        &self,
        corridor_id: Uuid,
        user_id: Uuid,
    ) -> Result<HttpResponse, AppError> {
        self.corridor_repo.delete_corridor(corridor_id, user_id)?;

        Ok(self.corridor_presenter.to_http_res())
    }

    pub fn fetch_corridor_stats(
        &self,
        corridor_id: Uuid,
        user_id: Uuid,
    ) -> Result<HttpResponse, AppError> {
        let stats = self
            .corridor_repo
            .fetch_corridor_stats(corridor_id, user_id)?;

        Ok(self.corridor_presenter.to_stats_json(stats))
    }
}

pub struct CreateCorridorUsecaseInput {
    pub user_id: Uuid,
    pub from_country_id: Uuid,
    pub to_country_id: Uuid,
    pub is_default: bool,
}
