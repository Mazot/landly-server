use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug, ToSchema, IntoParams)]
pub struct ModerationQueueQueryRequest {
    /// "org" | "person"; both when omitted
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ModerationDecisionRequest {
    /// "org" | "person"
    pub kind: String,
    pub target_id: Uuid,
    /// Note to the author; REQUIRED for request-changes
    pub note: Option<String>,
}
