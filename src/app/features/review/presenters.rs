use super::entities::Review;
use actix_web::HttpResponse;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub trait ReviewPresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    fn to_single_json(&self, review: Review) -> HttpResponse;
    fn to_multi_json(&self, reviews: Vec<Review>) -> HttpResponse;
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReviewContent {
    pub id: Uuid,
    pub author_id: Uuid,
    pub organisation_id: Option<Uuid>,
    pub person_id: Option<Uuid>,
    pub rating: i32,
    pub topic: Option<String>,
    pub text: Option<String>,
    pub created_at: NaiveDateTime,
}

impl From<Review> for ReviewContent {
    fn from(r: Review) -> Self {
        Self {
            id: r.id,
            author_id: r.author_id,
            organisation_id: r.organisation_id,
            person_id: r.person_id,
            rating: r.rating,
            topic: r.topic,
            text: r.text,
            created_at: r.created_at,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct MultipleReviewsResponse {
    pub items: Vec<ReviewContent>,
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

    fn to_single_json(&self, review: Review) -> HttpResponse {
        HttpResponse::Ok().json(ReviewContent::from(review))
    }

    fn to_multi_json(&self, reviews: Vec<Review>) -> HttpResponse {
        let items: Vec<ReviewContent> = reviews.into_iter().map(ReviewContent::from).collect();
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
        let content = ReviewContent::from(test_review(Some(org_id), None));

        assert_eq!(content.organisation_id, Some(org_id));
        assert!(content.person_id.is_none());
        assert_eq!(content.rating, 4);
        assert_eq!(content.topic.as_deref(), Some("Anmeldung help"));
    }

    #[test]
    fn test_review_content_is_camel_cased() {
        let content = ReviewContent::from(test_review(None, Some(Uuid::new_v4())));
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
                .to_single_json(test_review(Some(Uuid::new_v4()), None))
                .status()
                .is_success()
        );
        assert!(presenter.to_multi_json(vec![]).status().is_success());
    }
}
