use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Deserialize, Serialize, Debug, ToSchema, IntoParams)]
pub struct ImagesListQueryParams {
    /// Page size (default 20, max 100).
    pub limit: Option<i64>,
    /// Rows to skip (default 0, max 500).
    pub offset: Option<i64>,
}
