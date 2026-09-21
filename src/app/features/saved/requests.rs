use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateSavedRequest {
    /// "org" | "person" | "country" | "corridor"
    pub kind: String,
    pub target_id: Uuid,
    /// Private note, visible only to the owner
    pub note: Option<String>,
    pub list_name: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, ToSchema, IntoParams)]
pub struct ListSavedQueryRequest {
    /// Filter by kind; all kinds when omitted
    pub kind: Option<String>,
}
