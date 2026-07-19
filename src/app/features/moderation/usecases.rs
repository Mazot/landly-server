use super::{
    presenters::ModerationPresenter,
    repositories::{ModerationAction, ModerationRepository, TargetKind},
};
use crate::error::AppError;
use actix_web::HttpResponse;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ModerationUsecase {
    moderation_repo: Arc<dyn ModerationRepository>,
    moderation_presenter: Arc<dyn ModerationPresenter>,
}

impl ModerationUsecase {
    pub fn new(
        moderation_repo: Arc<dyn ModerationRepository>,
        moderation_presenter: Arc<dyn ModerationPresenter>,
    ) -> Self {
        Self {
            moderation_repo,
            moderation_presenter,
        }
    }

    /// Every moderation endpoint requires the moderator (or admin) role.
    fn ensure_moderator(&self, caller_user_id: Uuid) -> Result<(), AppError> {
        let role = self.moderation_repo.fetch_user_role(caller_user_id)?;

        if role.is_moderator() {
            return Ok(());
        }

        Err(AppError::Forbidden(
            json!({ "error": "Moderator role required" }),
        ))
    }

    pub fn fetch_queue(
        &self,
        caller_user_id: Uuid,
        kind: Option<String>,
    ) -> Result<HttpResponse, AppError> {
        self.ensure_moderator(caller_user_id)?;

        let kind = kind.as_deref().map(TargetKind::try_from).transpose()?;
        let items = self.moderation_repo.fetch_queue(kind)?;

        Ok(self.moderation_presenter.to_queue_json(items))
    }

    pub fn approve(
        &self,
        caller_user_id: Uuid,
        kind: String,
        target_id: Uuid,
        note: Option<String>,
    ) -> Result<HttpResponse, AppError> {
        self.ensure_moderator(caller_user_id)?;
        let kind = TargetKind::try_from(kind.as_str())?;

        self.moderation_repo.moderate(
            kind,
            target_id,
            ModerationAction::Approve,
            note,
            caller_user_id,
        )?;

        Ok(self.moderation_presenter.to_http_res())
    }

    /// Request-changes keeps the item pending; the note to the author is
    /// mandatory — without it the author has nothing to act on.
    pub fn request_changes(
        &self,
        caller_user_id: Uuid,
        kind: String,
        target_id: Uuid,
        note: Option<String>,
    ) -> Result<HttpResponse, AppError> {
        self.ensure_moderator(caller_user_id)?;
        let kind = TargetKind::try_from(kind.as_str())?;

        let note = note.filter(|n| !n.trim().is_empty()).ok_or_else(|| {
            AppError::UnprocessableEntity(json!({
                "error": "A note for the author is required when requesting changes"
            }))
        })?;

        self.moderation_repo.moderate(
            kind,
            target_id,
            ModerationAction::RequestChanges,
            Some(note),
            caller_user_id,
        )?;

        Ok(self.moderation_presenter.to_http_res())
    }

    pub fn reject(
        &self,
        caller_user_id: Uuid,
        kind: String,
        target_id: Uuid,
        note: Option<String>,
    ) -> Result<HttpResponse, AppError> {
        self.ensure_moderator(caller_user_id)?;
        let kind = TargetKind::try_from(kind.as_str())?;

        self.moderation_repo.moderate(
            kind,
            target_id,
            ModerationAction::Reject,
            note,
            caller_user_id,
        )?;

        Ok(self.moderation_presenter.to_http_res())
    }
}
