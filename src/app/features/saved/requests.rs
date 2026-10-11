use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateSavedRequest {
    /// "org" | "person" | "country" | "corridor"
    pub kind: String,
    /// ID of the saved item (no FK — polymorphic). Each item can be saved once per user.
    pub target_id: Uuid,
    /// Private note, visible only to the owner
    pub note: Option<String>,
    /// Optional user-defined list the item belongs to, e.g. "Moving checklist".
    pub list_name: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, ToSchema, IntoParams)]
pub struct ListSavedQueryRequest {
    /// Filter by kind; all kinds when omitted
    pub kind: Option<String>,
}
