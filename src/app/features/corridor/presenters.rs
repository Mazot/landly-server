use super::entities::{Corridor, CorridorStats};
use actix_web::HttpResponse;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub trait CorridorPresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    fn to_single_json(&self, item: Corridor) -> HttpResponse;
    fn to_multi_json(&self, items: Vec<Corridor>) -> HttpResponse;
    fn to_stats_json(&self, stats: CorridorStats) -> HttpResponse;
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CorridorContent {
    /// Corridor ID.
    pub id: Uuid,
    /// Home country the user comes from.
    pub from_country_id: Uuid,
    /// Destination country the map opens to; must differ from `from_country_id`.
    pub to_country_id: Uuid,
    /// Whether this is the user's default corridor (at most one per user).
    pub is_default: bool,
    /// Creation time (UTC, no timezone).
    pub created_at: NaiveDateTime,
}

impl From<Corridor> for CorridorContent {
    fn from(c: Corridor) -> Self {
        Self {
            id: c.id,
            from_country_id: c.from_country_id,
            to_country_id: c.to_country_id,
            is_default: c.is_default,
            created_at: c.created_at,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct MultipleCorridorsResponse {
    /// The caller's corridors, default first, then oldest first.
    pub items: Vec<CorridorContent>,
    /// Number of items returned.
    pub total: i64,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CorridorTypeCount {
    /// Organisation type slug (`"other"` when the type has no slug).
    pub slug: String,
    /// Live organisations of this type in the destination country.
    pub count: i64,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CorridorStatsContent {
    /// Corridor ID.
    pub corridor_id: Uuid,
    /// Home country the user comes from.
    pub from_country_id: Uuid,
    /// Destination country the map opens to; must differ from `from_country_id`.
    pub to_country_id: Uuid,
    /// Live organisations located in the destination country.
    pub total_places: i64,
    /// Of those, how many were created during the last 7 days.
    pub new_this_week: i64,
    /// Live organisations per organisation type.
    pub by_type: Vec<CorridorTypeCount>,
}

impl From<CorridorStats> for CorridorStatsContent {
    fn from(stats: CorridorStats) -> Self {
        Self {
            corridor_id: stats.corridor.id,
            from_country_id: stats.corridor.from_country_id,
            to_country_id: stats.corridor.to_country_id,
            total_places: stats.total_places,
            new_this_week: stats.new_this_week,
            by_type: stats
                .by_type
                .into_iter()
                .map(|(slug, count)| CorridorTypeCount { slug, count })
                .collect(),
        }
    }
}

#[derive(Clone)]
pub struct CorridorPresenterImpl {}

impl CorridorPresenterImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl CorridorPresenter for CorridorPresenterImpl {
    fn to_http_res(&self) -> HttpResponse {
        HttpResponse::Ok().json("OK")
    }

    fn to_single_json(&self, item: Corridor) -> HttpResponse {
        HttpResponse::Ok().json(CorridorContent::from(item))
    }

    fn to_multi_json(&self, items: Vec<Corridor>) -> HttpResponse {
        let items: Vec<CorridorContent> = items.into_iter().map(CorridorContent::from).collect();
        let total = items.len() as i64;

        HttpResponse::Ok().json(MultipleCorridorsResponse { items, total })
    }

    fn to_stats_json(&self, stats: CorridorStats) -> HttpResponse {
        HttpResponse::Ok().json(CorridorStatsContent::from(stats))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_corridor() -> Corridor {
        Corridor {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            from_country_id: Uuid::new_v4(),
            to_country_id: Uuid::new_v4(),
            is_default: true,
            created_at: chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        }
    }

    #[test]
    fn test_corridor_content_from_corridor() {
        let corridor = test_corridor();
        let id = corridor.id;

        let content = CorridorContent::from(corridor);

        assert_eq!(content.id, id);
        assert!(content.is_default);
    }

    #[test]
    fn test_corridor_content_does_not_expose_user_id() {
        let corridor = test_corridor();
        let content = CorridorContent::from(corridor);
        let json = serde_json::to_string(&content).unwrap();

        assert!(!json.contains("userId"));
        assert!(!json.contains("user_id"));
    }

    #[test]
    fn test_corridor_stats_content() {
        let corridor = test_corridor();
        let corridor_id = corridor.id;
        let stats = CorridorStats {
            corridor,
            total_places: 234,
            new_this_week: 12,
            by_type: vec![("embassy".to_string(), 4), ("business".to_string(), 142)],
        };

        let content = CorridorStatsContent::from(stats);

        assert_eq!(content.corridor_id, corridor_id);
        assert_eq!(content.total_places, 234);
        assert_eq!(content.new_this_week, 12);
        assert_eq!(content.by_type.len(), 2);
        assert_eq!(content.by_type[0].slug, "embassy");
        assert_eq!(content.by_type[0].count, 4);
    }

    #[test]
    fn test_presenter_responses_are_ok() {
        let presenter = CorridorPresenterImpl::new();

        assert!(
            presenter
                .to_single_json(test_corridor())
                .status()
                .is_success()
        );
        assert!(
            presenter
                .to_multi_json(vec![test_corridor()])
                .status()
                .is_success()
        );
    }
}
