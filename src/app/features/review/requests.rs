use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateReviewRequest {
    /// Exactly one of organisation_id / person_id
    pub organisation_id: Option<Uuid>,
    pub person_id: Option<Uuid>,
    /// 1..=5
    pub rating: i32,
    /// Topic tag, e.g. "Anmeldung help"
    pub topic: Option<String>,
    pub text: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, ToSchema, IntoParams)]
pub struct ListReviewsQueryRequest {
    pub organisation_id: Option<Uuid>,
    pub person_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
