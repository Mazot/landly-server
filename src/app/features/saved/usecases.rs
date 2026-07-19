use super::{
    entities::{CreateSavedItem, SavedKind},
    presenters::SavedPresenter,
    repositories::SavedRepository,
};
use crate::error::AppError;
use actix_web::HttpResponse;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct SavedUsecase {
    saved_repo: Arc<dyn SavedRepository>,
    saved_presenter: Arc<dyn SavedPresenter>,
}

impl SavedUsecase {
    pub fn new(
        saved_repo: Arc<dyn SavedRepository>,
        saved_presenter: Arc<dyn SavedPresenter>,
    ) -> Self {
        Self {
            saved_repo,
            saved_presenter,
        }
    }

    /// Saves a bookmark; the (user, kind, target) uniqueness lives in the DB.
    pub fn create_saved(&self, params: CreateSavedUsecaseInput) -> Result<HttpResponse, AppError> {
        let kind = SavedKind::try_from(params.kind.as_str())?;

        let item = self.saved_repo.create_saved(CreateSavedItem {
            user_id: params.user_id,
            kind: kind.as_str().to_string(),
            target_id: params.target_id,
            note: params.note,
            list_name: params.list_name,
        })?;

        Ok(self.saved_presenter.to_single_json(item))
    }

    pub fn delete_saved(&self, item_id: Uuid, user_id: Uuid) -> Result<HttpResponse, AppError> {
        self.saved_repo.delete_saved(item_id, user_id)?;

        Ok(self.saved_presenter.to_http_res())
    }

    pub fn list_saved(
        &self,
        user_id: Uuid,
        kind: Option<String>,
    ) -> Result<HttpResponse, AppError> {
        let kind = kind.as_deref().map(SavedKind::try_from).transpose()?;
        let items = self.saved_repo.list_saved(user_id, kind)?;

        Ok(self.saved_presenter.to_multi_json(items))
    }

    pub fn counts_saved(&self, user_id: Uuid) -> Result<HttpResponse, AppError> {
        let counts = self.saved_repo.counts_saved(user_id)?;

        Ok(self.saved_presenter.to_counts_json(counts))
    }
}

pub struct CreateSavedUsecaseInput {
    pub user_id: Uuid,
    pub kind: String,
    pub target_id: Uuid,
    pub note: Option<String>,
    pub list_name: Option<String>,
}
