use super::entities::Report;
use actix_web::HttpResponse;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub trait ReportPresenter: Send + Sync + 'static {
    fn to_single_json(&self, report: Report) -> HttpResponse;
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReportContent {
    pub id: Uuid,
    pub target_kind: String,
    pub target_id: Uuid,
    pub reason: String,
    pub status: String,
    pub created_at: NaiveDateTime,
}

impl From<Report> for ReportContent {
    fn from(r: Report) -> Self {
        Self {
            id: r.id,
            target_kind: r.target_kind,
            target_id: r.target_id,
            reason: r.reason,
            status: r.status,
            created_at: r.created_at,
        }
    }
}

#[derive(Clone)]
pub struct ReportPresenterImpl {}

impl ReportPresenterImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl ReportPresenter for ReportPresenterImpl {
    fn to_single_json(&self, report: Report) -> HttpResponse {
        HttpResponse::Ok().json(ReportContent::from(report))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_report() -> Report {
        Report {
            id: Uuid::new_v4(),
            reporter_id: Some(Uuid::new_v4()),
            target_kind: "org".to_string(),
            target_id: Uuid::new_v4(),
            reason: "closed down".to_string(),
            status: "open".to_string(),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }

    /// The reporter stays anonymous towards the reported party: the id is
    /// never part of the response payload.
    #[test]
    fn test_report_content_does_not_expose_the_reporter() {
        let content = ReportContent::from(test_report());
        let json = serde_json::to_string(&content).unwrap();

        assert!(!json.contains("reporterId"));
        assert!(!json.contains("reporter_id"));
    }

    #[test]
    fn test_report_content_keeps_kind_and_status() {
        let content = ReportContent::from(test_report());

        assert_eq!(content.target_kind, "org");
        assert_eq!(content.status, "open");
        assert_eq!(content.reason, "closed down");
    }

    #[test]
    fn test_presenter_response_is_ok() {
        assert!(
            ReportPresenterImpl::new()
                .to_single_json(test_report())
                .status()
                .is_success()
        );
    }
}
