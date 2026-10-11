use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateReportRequest {
    /// "org" | "person" | "conversation"
    pub target_kind: String,
    /// ID of the reported item (no FK — polymorphic).
    pub target_id: Uuid,
    /// Why the item is reported.
    pub reason: String,
}
