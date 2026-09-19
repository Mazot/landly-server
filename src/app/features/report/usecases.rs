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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::features::report::entities::Report;
    use crate::app::features::report::presenters::ReportPresenterImpl;
    use std::sync::Mutex;

    #[derive(Default)]
    struct StubRepo {
        created: Mutex<Vec<(ReportTargetKind, String)>>,
    }

    impl ReportRepository for StubRepo {
        fn create_report(
            &self,
            reporter_id: Uuid,
            target_kind: ReportTargetKind,
            target_id: Uuid,
            reason: &str,
        ) -> Result<Report, AppError> {
            self.created
                .lock()
                .unwrap()
                .push((target_kind, reason.to_string()));

            Ok(Report {
                id: Uuid::new_v4(),
                reporter_id: Some(reporter_id),
                target_kind: target_kind.as_str().to_string(),
                target_id,
                reason: reason.to_string(),
                status: "open".to_string(),
                created_at: chrono::Utc::now().naive_utc(),
            })
        }
    }

    fn usecase() -> (ReportUsecase, Arc<StubRepo>) {
        let repo = Arc::new(StubRepo::default());
        let usecase = ReportUsecase::new(repo.clone(), Arc::new(ReportPresenterImpl::new()));

        (usecase, repo)
    }

    #[test]
    fn test_create_report_accepts_every_known_kind() {
        for kind in ["org", "person", "conversation"] {
            let (usecase, repo) = usecase();

            assert!(
                usecase
                    .create_report(
                        Uuid::new_v4(),
                        kind.to_string(),
                        Uuid::new_v4(),
                        "spam".to_string(),
                    )
                    .is_ok()
            );
            assert_eq!(repo.created.lock().unwrap()[0].0.as_str(), kind);
        }
    }

    #[test]
    fn test_create_report_rejects_unknown_kind() {
        let (usecase, repo) = usecase();

        match usecase.create_report(
            Uuid::new_v4(),
            "corridor".to_string(),
            Uuid::new_v4(),
            "spam".to_string(),
        ) {
            Err(AppError::UnprocessableEntity(_)) => (),
            other => panic!("expected UnprocessableEntity, got {:?}", other.err()),
        }
        assert!(repo.created.lock().unwrap().is_empty());
    }

    /// A moderator needs something to act on, so a blank reason is a 422 —
    /// whitespace included.
    #[test]
    fn test_create_report_rejects_a_blank_reason() {
        for reason in ["", "   ", "\n\t"] {
            let (usecase, repo) = usecase();

            match usecase.create_report(
                Uuid::new_v4(),
                "org".to_string(),
                Uuid::new_v4(),
                reason.to_string(),
            ) {
                Err(AppError::UnprocessableEntity(_)) => (),
                other => panic!(
                    "expected UnprocessableEntity for {:?}, got {:?}",
                    reason,
                    other.err()
                ),
            }
            assert!(repo.created.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn test_create_report_trims_the_reason() {
        let (usecase, repo) = usecase();

        usecase
            .create_report(
                Uuid::new_v4(),
                "org".to_string(),
                Uuid::new_v4(),
                "  closed down  ".to_string(),
            )
            .unwrap();

        assert_eq!(repo.created.lock().unwrap()[0].1, "closed down");
    }
}
