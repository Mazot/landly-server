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

/// Turns the sparse `(kind, count)` rows the repository groups by into the
/// dense badge payload: every kind is present, missing ones read 0.
impl From<Vec<(String, i64)>> for SavedCountsContent {
    fn from(counts: Vec<(String, i64)>) -> Self {
        let get = |kind: SavedKind| {
            counts
                .iter()
                .find(|(k, _)| k == kind.as_str())
                .map(|(_, c)| *c)
                .unwrap_or(0)
        };

        Self {
            org: get(SavedKind::Org),
            person: get(SavedKind::Person),
            country: get(SavedKind::Country),
            corridor: get(SavedKind::Corridor),
            total: counts.iter().map(|(_, c)| c).sum(),
        }
    }
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
        HttpResponse::Ok().json(SavedCountsContent::from(counts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_item(kind: SavedKind) -> SavedItem {
        SavedItem {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            kind: kind.as_str().to_string(),
            target_id: Uuid::new_v4(),
            note: Some("worth a visit".to_string()),
            list_name: Some("Berlin".to_string()),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }

    #[test]
    fn test_saved_item_content_does_not_expose_user_id() {
        let content = SavedItemContent::from(test_item(SavedKind::Org));
        let json = serde_json::to_string(&content).unwrap();

        assert!(!json.contains("userId"));
        assert!(!json.contains("user_id"));
    }

    /// Kinds the repository did not group are badges reading 0, not missing
    /// fields — the Saved tab renders all four unconditionally.
    #[test]
    fn test_counts_fill_missing_kinds_with_zero() {
        let counts = SavedCountsContent::from(vec![("org".to_string(), 3)]);

        assert_eq!(counts.org, 3);
        assert_eq!(counts.person, 0);
        assert_eq!(counts.country, 0);
        assert_eq!(counts.corridor, 0);
        assert_eq!(counts.total, 3);
    }

    #[test]
    fn test_counts_sum_every_kind() {
        let counts = SavedCountsContent::from(vec![
            ("org".to_string(), 3),
            ("person".to_string(), 1),
            ("country".to_string(), 4),
            ("corridor".to_string(), 2),
        ]);

        assert_eq!(counts.person, 1);
        assert_eq!(counts.country, 4);
        assert_eq!(counts.corridor, 2);
        assert_eq!(counts.total, 10);
    }

    /// A kind the Rust enum does not know (e.g. added by a later migration)
    /// must still be counted in the total rather than silently dropped.
    #[test]
    fn test_counts_total_includes_unknown_kinds() {
        let counts = SavedCountsContent::from(vec![
            ("org".to_string(), 1),
            ("conversation".to_string(), 5),
        ]);

        assert_eq!(counts.org, 1);
        assert_eq!(counts.total, 6);
    }

    #[test]
    fn test_counts_of_an_empty_shelf() {
        let counts = SavedCountsContent::from(vec![]);

        assert_eq!(counts.total, 0);
        assert_eq!(counts.org, 0);
    }

    #[test]
    fn test_presenter_responses_are_ok() {
        let presenter = SavedPresenterImpl::new();

        assert!(presenter.to_http_res().status().is_success());
        assert!(
            presenter
                .to_single_json(test_item(SavedKind::Person))
                .status()
                .is_success()
        );
        assert!(
            presenter
                .to_multi_json(vec![test_item(SavedKind::Country)])
                .status()
                .is_success()
        );
        assert!(
            presenter
                .to_counts_json(vec![("org".to_string(), 1)])
                .status()
                .is_success()
        );
    }
}
