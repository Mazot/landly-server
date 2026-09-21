use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateReportRequest {
    /// "org" | "person" | "conversation"
    pub target_kind: String,
    pub target_id: Uuid,
    pub reason: String,
}
