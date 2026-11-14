use super::entities::{MAX_RATING, MIN_RATING, Review};
use crate::app::features::user::{entities::PublicUser, presenters::PublicUserContent};
use actix_web::HttpResponse;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub trait ReviewPresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    fn to_single_json(&self, review: Review, author: Option<PublicUser>) -> HttpResponse;
    fn to_multi_json(&self, reviews: Vec<(Review, Option<PublicUser>)>) -> HttpResponse;
    /// Count, average and per-rating breakdown from `(rating, count)` rows.
    fn to_summary_json(&self, rating_counts: Vec<(i32, i64)>) -> HttpResponse;
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReviewContent {
    /// Review ID.
    pub id: Uuid,
    /// User who wrote the review (one review per author and target).
    pub author_id: Uuid,
    /// Reviewed organisation; exactly one of `organisation_id` / `person_id` is set.
    pub organisation_id: Option<Uuid>,
    /// Reviewed person; exactly one of `organisation_id` / `person_id` is set.
    pub person_id: Option<Uuid>,
    /// Rating from 1 to 5.
    pub rating: i32,
    /// Optional topic tag, e.g. "Anmeldung help".
    pub topic: Option<String>,
    /// Review text.
    pub text: Option<String>,
    /// Publication time (UTC, no timezone).
    pub created_at: NaiveDateTime,
    /// Who wrote it — their public face; null when the account is gone.
    pub author: Option<PublicUserContent>,
}

impl ReviewContent {
    pub fn new(r: Review, author: Option<PublicUser>) -> Self {
        Self {
            id: r.id,
            author_id: r.author_id,
            organisation_id: r.organisation_id,
            person_id: r.person_id,
            rating: r.rating,
            topic: r.topic,
            text: r.text,
            created_at: r.created_at,
            author: author.map(PublicUserContent::from),
        }
    }
}

/// `GET /review/summary`: how a target is rated, with the breakdown the
/// rating bars show.
#[derive(Deserialize, Serialize, ToSchema, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSummaryContent {
    /// Number of reviews.
    pub count: i64,
    /// Average rating; null without reviews.
    pub rating_avg: Option<f64>,
    /// One row per rating, highest first (5 … 1), zeros included.
    pub by_rating: Vec<RatingCountContent>,
}

#[derive(Deserialize, Serialize, ToSchema, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RatingCountContent {
    /// The rating (1–5).
    pub rating: i32,
    /// Reviews that gave it.
    pub count: i64,
}

impl From<Vec<(i32, i64)>> for ReviewSummaryContent {
    fn from(rating_counts: Vec<(i32, i64)>) -> Self {
        let by_rating: Vec<RatingCountContent> = (MIN_RATING..=MAX_RATING)
            .rev()
            .map(|rating| RatingCountContent {
                rating,
                count: rating_counts
                    .iter()
                    .find(|(r, _)| *r == rating)
                    .map_or(0, |(_, count)| *count),
            })
            .collect();
        let count: i64 = by_rating.iter().map(|row| row.count).sum();
        let sum: i64 = by_rating
            .iter()
            .map(|row| i64::from(row.rating) * row.count)
            .sum();

        Self {
            count,
            rating_avg: (count > 0).then(|| sum as f64 / count as f64),
            by_rating,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct MultipleReviewsResponse {
    /// Reviews of the target, newest first.
    pub items: Vec<ReviewContent>,
    /// Number of items returned.
    pub total: i64,
}

#[derive(Clone)]
pub struct ReviewPresenterImpl {}

impl ReviewPresenterImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl ReviewPresenter for ReviewPresenterImpl {
    fn to_http_res(&self) -> HttpResponse {
        HttpResponse::Ok().json("OK")
    }

    fn to_single_json(&self, review: Review, author: Option<PublicUser>) -> HttpResponse {
        HttpResponse::Ok().json(ReviewContent::new(review, author))
    }

    fn to_summary_json(&self, rating_counts: Vec<(i32, i64)>) -> HttpResponse {
        HttpResponse::Ok().json(ReviewSummaryContent::from(rating_counts))
    }

    fn to_multi_json(&self, reviews: Vec<(Review, Option<PublicUser>)>) -> HttpResponse {
        let items: Vec<ReviewContent> = reviews
            .into_iter()
            .map(|(review, author)| ReviewContent::new(review, author))
            .collect();
        let total = items.len() as i64;

        HttpResponse::Ok().json(MultipleReviewsResponse { items, total })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_review(organisation_id: Option<Uuid>, person_id: Option<Uuid>) -> Review {
        Review {
            id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            organisation_id,
            person_id,
            rating: 4,
            topic: Some("Anmeldung help".to_string()),
            text: Some("Fast and friendly".to_string()),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }

    #[test]
    fn test_review_content_keeps_the_target() {
        let org_id = Uuid::new_v4();
        let content = ReviewContent::new(test_review(Some(org_id), None), None);

        assert_eq!(content.organisation_id, Some(org_id));
        assert!(content.person_id.is_none());
        assert_eq!(content.rating, 4);
        assert_eq!(content.topic.as_deref(), Some("Anmeldung help"));
    }

    #[test]
    fn test_review_content_is_camel_cased() {
        let content = ReviewContent::new(test_review(None, Some(Uuid::new_v4())), None);
        let json = serde_json::to_string(&content).unwrap();

        assert!(json.contains("authorId"));
        assert!(json.contains("personId"));
        assert!(!json.contains("author_id"));
    }

    #[test]
    fn test_presenter_responses_are_ok() {
        let presenter = ReviewPresenterImpl::new();

        assert!(presenter.to_http_res().status().is_success());
        assert!(
            presenter
                .to_single_json(test_review(Some(Uuid::new_v4()), None), None)
                .status()
                .is_success()
        );
        assert!(presenter.to_multi_json(vec![]).status().is_success());
        assert!(presenter.to_summary_json(vec![]).status().is_success());
    }

    /// The author is their public face only: username, name, avatar colour.
    #[test]
    fn test_review_content_carries_the_public_author() {
        let review = test_review(Some(Uuid::new_v4()), None);
        let author = PublicUser {
            id: review.author_id,
            username: "pavel".to_string(),
            name: Some("Pavel K.".to_string()),
            avatar_color: Some("#4F7F5C".to_string()),
        };

        let json = serde_json::to_value(ReviewContent::new(review, Some(author))).unwrap();
        assert_eq!(json["author"]["username"], "pavel");
        assert_eq!(json["author"]["name"], "Pavel K.");
        assert_eq!(json["author"]["avatarColor"], "#4F7F5C");
        assert!(json["author"].get("id").is_none());
        assert!(json["author"].get("email").is_none());
    }

    /// Every rating gets a row, highest first; the average comes from the rows.
    #[test]
    fn test_summary_breaks_ratings_down_from_five_to_one() {
        let summary = ReviewSummaryContent::from(vec![(5, 3), (3, 1)]);

        assert_eq!(summary.count, 4);
        assert_eq!(summary.rating_avg, Some(4.5));
        assert_eq!(
            summary
                .by_rating
                .iter()
                .map(|row| (row.rating, row.count))
                .collect::<Vec<_>>(),
            vec![(5, 3), (4, 0), (3, 1), (2, 0), (1, 0)]
        );

        let empty = ReviewSummaryContent::from(vec![]);
        assert_eq!(empty.count, 0);
        assert_eq!(empty.rating_avg, None);
        assert_eq!(empty.by_rating.len(), 5);
    }
}
