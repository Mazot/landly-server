use super::{
    entities::ReportTargetKind, presenters::ReportPresenter, repositories::ReportRepository,
};
use crate::error::AppError;
use actix_web::HttpResponse;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ReportUsecase {
    report_repo: Arc<dyn ReportRepository>,
    report_presenter: Arc<dyn ReportPresenter>,
}

impl ReportUsecase {
    pub fn new(
        report_repo: Arc<dyn ReportRepository>,
        report_presenter: Arc<dyn ReportPresenter>,
    ) -> Self {
        Self {
            report_repo,
            report_presenter,
        }
    }

    /// Files a report; it lands in the moderation queue counters as `open`.
    pub fn create_report(
        &self,
        reporter_id: Uuid,
        target_kind: String,
        target_id: Uuid,
        reason: String,
    ) -> Result<HttpResponse, AppError> {
        let kind = ReportTargetKind::try_from(target_kind.as_str())?;

        let reason = reason.trim();
        if reason.is_empty() {
            return Err(AppError::UnprocessableEntity(
                json!({ "error": "reason is required" }),
            ));
        }

        let report = self
            .report_repo
            .create_report(reporter_id, kind, target_id, reason)?;

        Ok(self.report_presenter.to_single_json(report))
    }
}
