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
