use crate::data::{models::OrganisationType, schema::organisations};
use crate::error::*;
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Moderation status stored in `organisations.status` (TEXT + CHECK constraint).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrganisationStatus {
    Pending,
    Live,
    Rejected,
}

impl OrganisationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrganisationStatus::Pending => "pending",
            OrganisationStatus::Live => "live",
            OrganisationStatus::Rejected => "rejected",
        }
    }
}

impl TryFrom<&str> for OrganisationStatus {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(OrganisationStatus::Pending),
            "live" => Ok(OrganisationStatus::Live),
            "rejected" => Ok(OrganisationStatus::Rejected),
            other => Err(AppError::UnprocessableEntity(
                json!({ "error": format!("Unknown organisation status: {}", other) }),
            )),
        }
    }
}

/// Provenance of the listing (`organisations.added_by`, TEXT + CHECK constraint).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddedBy {
    Official,
    Community,
    Volunteer,
}

impl AddedBy {
    pub fn as_str(&self) -> &'static str {
        match self {
            AddedBy::Official => "official",
            AddedBy::Community => "community",
            AddedBy::Volunteer => "volunteer",
        }
    }
}

impl TryFrom<&str> for AddedBy {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "official" => Ok(AddedBy::Official),
            "community" => Ok(AddedBy::Community),
            "volunteer" => Ok(AddedBy::Volunteer),
            other => Err(AppError::UnprocessableEntity(
                json!({ "error": format!("Unknown added_by value: {}", other) }),
            )),
        }
    }
}

/// Whether the service costs money (`organisations.cost`, TEXT + CHECK constraint).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cost {
    Free,
    Paid,
}

impl Cost {
    pub fn as_str(&self) -> &'static str {
        match self {
            Cost::Free => "free",
            Cost::Paid => "paid",
        }
    }
}

impl TryFrom<&str> for Cost {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "free" => Ok(Cost::Free),
            "paid" => Ok(Cost::Paid),
            other => Err(AppError::UnprocessableEntity(
                json!({ "error": format!("Unknown cost value: {}", other) }),
            )),
        }
    }
}

#[derive(Debug, Associations, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
// #[diesel(belongs_to(Country, foreign_key = location_country_id))]
// #[diesel(belongs_to(Country, foreign_key = founder_country_id))]
#[diesel(belongs_to(OrganisationType, foreign_key = organisation_type_id))]
#[diesel(table_name = organisations)]
pub struct Organisation {
    pub id: Uuid,
    pub name: String,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub description: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub latitude: Option<BigDecimal>,
    pub longitude: Option<BigDecimal>,
    pub founder_country_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub verified: bool,
    pub status: String,
    pub moderation_note: Option<String>,
    pub added_by: Option<String>,
    pub city: Option<String>,
    pub website: Option<String>,
    pub telegram: Option<String>,
    pub whatsapp: Option<String>,
    pub services: Vec<Option<String>>,
    pub languages: Vec<Option<String>>,
    pub opening_hours: Option<serde_json::Value>,
    pub timezone: Option<String>,
    pub cost: Option<String>,
    pub google_place_id: Option<String>,
    pub google_rating: Option<f64>,
    pub visits_count: i64,
    pub rating_avg: Option<f64>,
    pub reviews_count: i64,
}

impl Organisation {
    pub fn create(conn: &mut PgConnection, record: &CreateOrganisation) -> Result<Self, AppError> {
        let result = diesel::insert_into(organisations::table)
            .values(record)
            .get_result::<Organisation>(conn)?;

        Ok(result)
    }

    pub fn update(
        conn: &mut PgConnection,
        organisation_id: Uuid,
        record: &UpdateOrganisation,
    ) -> Result<Self, AppError> {
        let o = organisations::table.find(organisation_id);
        let result = diesel::update(o)
            .set(record)
            .get_result::<Organisation>(conn)?;

        Ok(result)
    }

    pub fn delete(conn: &mut PgConnection, organisation_id: Uuid) -> Result<(), AppError> {
        let o = organisations::table.find(organisation_id);
        diesel::delete(o).execute(conn)?;

        Ok(())
    }

    pub fn fetch_by_id(conn: &mut PgConnection, organisation_id: Uuid) -> Result<Self, AppError> {
        let result = organisations::table
            .find(organisation_id)
            .get_result::<Organisation>(conn)?;

        Ok(result)
    }

    pub fn increment_visits(
        conn: &mut PgConnection,
        organisation_id: Uuid,
    ) -> Result<i64, AppError> {
        let visits = diesel::update(organisations::table.find(organisation_id))
            .set(organisations::visits_count.eq(organisations::visits_count + 1))
            .returning(organisations::visits_count)
            .get_result::<i64>(conn)?;

        Ok(visits)
    }

    pub fn fetch_by_location_country(
        conn: &mut PgConnection,
        country_id: Uuid,
    ) -> Result<Vec<Self>, AppError> {
        let result = organisations::table
            .filter(organisations::location_country_id.eq(country_id))
            .load::<Organisation>(conn)?;

        Ok(result)
    }

    pub fn fetch_ids_by_organisation_type(
        conn: &mut PgConnection,
        organisation_type_id: Uuid,
    ) -> Result<Vec<Uuid>, AppError> {
        let result = organisations::table
            .filter(organisations::organisation_type_id.eq(organisation_type_id))
            .select(organisations::id)
            .load::<Uuid>(conn)?;

        Ok(result)
    }

    pub fn fetch_ids_by_location_country(
        conn: &mut PgConnection,
        location_country_id: Uuid,
    ) -> Result<Vec<Uuid>, AppError> {
        let result = organisations::table
            .filter(organisations::location_country_id.eq(location_country_id))
            .select(organisations::id)
            .load::<Uuid>(conn)?;

        Ok(result)
    }

    pub fn fetch_ids_by_founder_country(
        conn: &mut PgConnection,
        founder_country_id: Uuid,
    ) -> Result<Vec<Uuid>, AppError> {
        let result = organisations::table
            .filter(organisations::founder_country_id.eq(founder_country_id))
            .select(organisations::id)
            .load::<Uuid>(conn)?;

        Ok(result)
    }
}

#[derive(Insertable, Clone)]
#[diesel(table_name = organisations)]
pub struct CreateOrganisation {
    pub name: String,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub description: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
    pub latitude: Option<BigDecimal>,
    pub longitude: Option<BigDecimal>,
    pub founder_country_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub status: String,
    pub added_by: Option<String>,
    pub city: Option<String>,
    pub website: Option<String>,
    pub telegram: Option<String>,
    pub whatsapp: Option<String>,
    pub services: Vec<Option<String>>,
    pub languages: Vec<Option<String>>,
    pub opening_hours: Option<serde_json::Value>,
    pub timezone: Option<String>,
    pub cost: Option<String>,
    pub google_place_id: Option<String>,
}

#[derive(AsChangeset, Clone)]
#[diesel(table_name = organisations)]
pub struct UpdateOrganisation {
    pub name: Option<String>,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub description: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
    pub updated_at: NaiveDateTime,
    pub latitude: Option<BigDecimal>,
    pub longitude: Option<BigDecimal>,
    pub founder_country_id: Option<Uuid>,
    pub city: Option<String>,
    pub website: Option<String>,
    pub telegram: Option<String>,
    pub whatsapp: Option<String>,
    pub services: Option<Vec<Option<String>>>,
    pub languages: Option<Vec<Option<String>>>,
    pub opening_hours: Option<serde_json::Value>,
    pub timezone: Option<String>,
    pub cost: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_organisation_status_roundtrip() {
        for status in [
            OrganisationStatus::Pending,
            OrganisationStatus::Live,
            OrganisationStatus::Rejected,
        ] {
            assert_eq!(
                OrganisationStatus::try_from(status.as_str()).unwrap(),
                status
            );
        }
        assert!(OrganisationStatus::try_from("draft").is_err());
    }

    #[test]
    fn test_added_by_roundtrip() {
        for added_by in [AddedBy::Official, AddedBy::Community, AddedBy::Volunteer] {
            assert_eq!(AddedBy::try_from(added_by.as_str()).unwrap(), added_by);
        }
        assert!(AddedBy::try_from("bot").is_err());
    }

    #[test]
    fn test_cost_roundtrip() {
        assert_eq!(Cost::try_from("free").unwrap(), Cost::Free);
        assert_eq!(Cost::try_from("paid").unwrap(), Cost::Paid);
        assert!(Cost::try_from("donation").is_err());
    }

    #[test]
    fn test_enum_errors_are_unprocessable_entity() {
        match OrganisationStatus::try_from("nope") {
            Err(AppError::UnprocessableEntity(_)) => (),
            other => panic!("expected UnprocessableEntity, got {:?}", other.err()),
        }
    }
}
