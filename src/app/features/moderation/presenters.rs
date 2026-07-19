use super::repositories::{ModerationEvent, QueueItem};
use actix_web::HttpResponse;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub trait ModerationPresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    fn to_queue_json(&self, items: Vec<QueueItem>) -> HttpResponse;
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModerationEventContent {
    pub action: String,
    pub note: Option<String>,
    /// Submit-time auto-checks (duplicate nearby, phone format, trust level)
    pub flags: Option<serde_json::Value>,
    pub moderator_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
}

impl From<ModerationEvent> for ModerationEventContent {
    fn from(e: ModerationEvent) -> Self {
        Self {
            action: e.action,
            note: e.note,
            flags: e.flags,
            moderator_id: e.moderator_id,
            created_at: e.created_at,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModerationQueueItemContent {
    /// "org" or "person"
    pub kind: String,
    pub target_id: Uuid,
    pub name: String,
    pub status: String,
    pub city: Option<String>,
    pub submitted_by: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub last_event: Option<ModerationEventContent>,
    pub open_reports: i64,
}

impl From<QueueItem> for ModerationQueueItemContent {
    fn from(item: QueueItem) -> Self {
        Self {
            kind: item.kind.as_str().to_string(),
            target_id: item.target_id,
            name: item.name,
            status: item.status,
            city: item.city,
            submitted_by: item.submitted_by,
            created_at: item.created_at,
            last_event: item.last_event.map(ModerationEventContent::from),
            open_reports: item.open_reports,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct ModerationQueueResponse {
    pub items: Vec<ModerationQueueItemContent>,
    pub total: i64,
}

#[derive(Clone)]
pub struct ModerationPresenterImpl {}

impl ModerationPresenterImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl ModerationPresenter for ModerationPresenterImpl {
    fn to_http_res(&self) -> HttpResponse {
        HttpResponse::Ok().json("OK")
    }

    fn to_queue_json(&self, items: Vec<QueueItem>) -> HttpResponse {
        let items: Vec<ModerationQueueItemContent> = items
            .into_iter()
            .map(ModerationQueueItemContent::from)
            .collect();
        let total = items.len() as i64;

        HttpResponse::Ok().json(ModerationQueueResponse { items, total })
    }
}
