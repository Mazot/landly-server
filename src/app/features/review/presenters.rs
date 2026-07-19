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
