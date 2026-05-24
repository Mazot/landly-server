use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Deserialize, Serialize, Debug, ToSchema, IntoParams)]
pub struct ImagesListQueryParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
