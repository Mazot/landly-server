use super::entities::{ModerationEvent, QueueItem};
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
    /// `submitted`, `approve`, `request_changes` or `reject`.
    pub action: String,
    /// Note to the author (required for `request_changes`).
    pub note: Option<String>,
    /// Submit-time auto-checks (duplicate nearby, phone format, trust level)
    pub flags: Option<serde_json::Value>,
    /// Moderator who acted; null for the automatic `submitted` event.
    pub moderator_id: Option<Uuid>,
    /// When the event happened (UTC, no timezone).
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
    /// ID of the pending organisation or person.
    pub target_id: Uuid,
    /// Organisation or person name.
    pub name: String,
    /// Current status (always pending while in the queue).
    pub status: String,
    /// City, if given.
    pub city: Option<String>,
    /// Creator of the organisation / recommender of the person.
    pub submitted_by: Option<Uuid>,
    /// Submission time (UTC, no timezone).
    pub created_at: NaiveDateTime,
    /// Latest moderation event, including the submit-time auto-check flags.
    pub last_event: Option<ModerationEventContent>,
    /// Number of `open` user reports against this item.
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
    /// Pending organisations and people, with flags and open report counts.
    pub items: Vec<ModerationQueueItemContent>,
    /// Number of items returned.
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
