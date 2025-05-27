use super::schema::*;
use serde::{Deserialize, Serialize, Serializer};
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = countries)]
pub struct Country {
    id: Uuid,
    pub name: String,
    geo_json: serde_json::Value,
    flag: Option<String>,
    capital_city: Option<String>,
    description: Option<String>,
}

