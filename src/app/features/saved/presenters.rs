use super::entities::{SavedItem, SavedKind};
use actix_web::HttpResponse;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub trait SavedPresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    fn to_single_json(&self, item: SavedItem) -> HttpResponse;
    fn to_multi_json(&self, items: Vec<SavedItem>) -> HttpResponse;
    fn to_counts_json(&self, counts: Vec<(String, i64)>) -> HttpResponse;
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SavedItemContent {
    pub id: Uuid,
    pub kind: String,
    pub target_id: Uuid,
    pub note: Option<String>,
    pub list_name: Option<String>,
    pub created_at: NaiveDateTime,
}

impl From<SavedItem> for SavedItemContent {
    fn from(item: SavedItem) -> Self {
        Self {
            id: item.id,
            kind: item.kind,
            target_id: item.target_id,
            note: item.note,
            list_name: item.list_name,
            created_at: item.created_at,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct MultipleSavedItemsResponse {
    pub items: Vec<SavedItemContent>,
    pub total: i64,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SavedCountsContent {
    pub org: i64,
    pub person: i64,
    pub country: i64,
    pub corridor: i64,
    pub total: i64,
}

#[derive(Clone)]
pub struct SavedPresenterImpl {}

impl SavedPresenterImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl SavedPresenter for SavedPresenterImpl {
    fn to_http_res(&self) -> HttpResponse {
        HttpResponse::Ok().json("OK")
    }

    fn to_single_json(&self, item: SavedItem) -> HttpResponse {
        HttpResponse::Ok().json(SavedItemContent::from(item))
    }

    fn to_multi_json(&self, items: Vec<SavedItem>) -> HttpResponse {
        let items: Vec<SavedItemContent> = items.into_iter().map(SavedItemContent::from).collect();
        let total = items.len() as i64;

        HttpResponse::Ok().json(MultipleSavedItemsResponse { items, total })
    }

    fn to_counts_json(&self, counts: Vec<(String, i64)>) -> HttpResponse {
        let get = |kind: SavedKind| {
            counts
                .iter()
                .find(|(k, _)| k == kind.as_str())
                .map(|(_, c)| *c)
                .unwrap_or(0)
        };

        let content = SavedCountsContent {
            org: get(SavedKind::Org),
            person: get(SavedKind::Person),
            country: get(SavedKind::Country),
            corridor: get(SavedKind::Corridor),
            total: counts.iter().map(|(_, c)| c).sum(),
        };

        HttpResponse::Ok().json(content)
    }
}
