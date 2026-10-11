use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateReviewRequest {
    /// Exactly one of organisation_id / person_id
    pub organisation_id: Option<Uuid>,
    /// Reviewed person (the person must be confirmed/claimed and allow reviews).
    pub person_id: Option<Uuid>,
    /// 1..=5
    pub rating: i32,
    /// Topic tag, e.g. "Anmeldung help"
    pub topic: Option<String>,
    /// Review text.
    pub text: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, ToSchema, IntoParams)]
pub struct ListReviewsQueryRequest {
    /// List reviews of this organisation (give exactly one target).
    pub organisation_id: Option<Uuid>,
    /// List reviews of this person (give exactly one target).
    pub person_id: Option<Uuid>,
    /// Page size (default 20, max 100).
    pub limit: Option<i64>,
    /// Rows to skip (default 0, max 500).
    pub offset: Option<i64>,
}
