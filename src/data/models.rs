use super::schema::*;
use crate::error::*;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// TODO: TMP ToSchema derive for documentation purposes
#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone, ToSchema)]
#[diesel(table_name = countries)]
pub struct Country {
    /// Country ID.
    pub id: Uuid,
    /// Country name in English, e.g. "Germany".
    pub name: String,
    /// Country border as a GeoJSON geometry (`Polygon` / `MultiPolygon`) for map rendering.
    pub geo_json: Option<serde_json::Value>,
    /// Flag emoji, e.g. "🇩🇪".
    pub flag: Option<String>,
    /// Capital city name.
    pub capital_city: Option<String>,
    /// Free-text description shown on the country page.
    pub description: Option<String>,
    /// Local currency, e.g. "EUR" (free text, not validated).
    pub currency: Option<String>,
    /// International dialling prefix, e.g. "+49".
    pub phone_code: Option<String>,
    /// Main cities for the country page, as stored JSON (typically an array of city names).
    pub top_cities: Option<serde_json::Value>,
}

#[derive(Insertable, Clone)]
#[diesel(table_name = countries)]
pub struct CreateCountry {
    /// Country name in English, e.g. "Germany".
    pub name: String,
    /// Country border as a GeoJSON geometry (`Polygon` / `MultiPolygon`) for map rendering.
    pub geo_json: Option<serde_json::Value>,
    /// Flag emoji, e.g. "🇩🇪".
    pub flag: Option<String>,
    /// Capital city name.
    pub capital_city: Option<String>,
    /// Free-text description shown on the country page.
    pub description: Option<String>,
    /// Local currency, e.g. "EUR" (free text, not validated).
    pub currency: Option<String>,
    /// International dialling prefix, e.g. "+49".
    pub phone_code: Option<String>,
    /// Main cities for the country page, as stored JSON (typically an array of city names).
    pub top_cities: Option<serde_json::Value>,
}

#[derive(AsChangeset)]
#[diesel(table_name = countries)]
pub struct UpdateCountry {
    /// Country name in English, e.g. "Germany"; `None` keeps the current value.
    pub name: Option<String>,
    /// Country border as a GeoJSON geometry (`Polygon` / `MultiPolygon`) for map rendering; `None` keeps the current value.
    pub geo_json: Option<serde_json::Value>,
    /// Flag emoji, e.g. "🇩🇪"; `None` keeps the current value.
    pub flag: Option<String>,
    /// Capital city name; `None` keeps the current value.
    pub capital_city: Option<String>,
    /// Free-text description shown on the country page; `None` keeps the current value.
    pub description: Option<String>,
    /// Local currency, e.g. "EUR" (free text, not validated); `None` keeps the current value.
    pub currency: Option<String>,
    /// International dialling prefix, e.g. "+49"; `None` keeps the current value.
    pub phone_code: Option<String>,
    /// Main cities for the country page, as stored JSON (typically an array of city names); `None` keeps the current value.
    pub top_cities: Option<serde_json::Value>,
}

impl Country {
    pub fn get_by_id(conn: &mut PgConnection, id: &Uuid) -> Result<Self, AppError> {
        let result = countries::table.find(id).get_result::<Country>(conn)?;

        Ok(result)
    }

    pub fn get_by_name(conn: &mut PgConnection, name: &str) -> Result<Self, AppError> {
        let result = countries::table
            .filter(countries::name.eq(name))
            .get_result::<Country>(conn)?;

        Ok(result)
    }

    pub fn get_all(conn: &mut PgConnection) -> Result<Vec<Self>, AppError> {
        let result = countries::table.load::<Country>(conn)?;

        Ok(result)
    }

    pub fn create(conn: &mut PgConnection, record: &CreateCountry) -> Result<Self, AppError> {
        let result = diesel::insert_into(countries::table)
            .values(record)
            .get_result::<Country>(conn)?;

        Ok(result)
    }

    pub fn update(
        conn: &mut PgConnection,
        country_id: Uuid,
        record: &UpdateCountry,
    ) -> Result<Self, AppError> {
        let c = countries::table.find(country_id);
        let result = diesel::update(c).set(record).get_result::<Country>(conn)?;

        Ok(result)
    }

    pub fn delete(conn: &mut PgConnection, country_id: Uuid) -> Result<(), AppError> {
        let c = countries::table.find(country_id);
        diesel::delete(c).execute(conn)?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = languages)]
pub struct Language {
    /// Language ID.
    pub id: Uuid,
    /// Language name, e.g. "Russian".
    pub name: String,
    /// Short code, e.g. "ru".
    pub symbol: Option<String>,
}

impl Language {
    pub fn create(conn: &mut PgConnection, record: &Language) -> Result<Self, AppError> {
        let result = diesel::insert_into(languages::table)
            .values(record)
            .get_result::<Language>(conn)?;

        Ok(result)
    }

    pub fn update(
        conn: &mut PgConnection,
        language_id: Uuid,
        record: &UpdateLanguage,
    ) -> Result<Self, AppError> {
        let l = languages::table.find(language_id);
        let result = diesel::update(l).set(record).get_result::<Language>(conn)?;

        Ok(result)
    }

    pub fn delete(conn: &mut PgConnection, language_id: Uuid) -> Result<(), AppError> {
        let l = languages::table.find(language_id);
        diesel::delete(l).execute(conn)?;

        Ok(())
    }
}

#[derive(AsChangeset)]
#[diesel(table_name = languages)]
pub struct UpdateLanguage {
    /// New name; `None` keeps the current one.
    pub name: Option<String>,
    /// New short code; `None` keeps the current one.
    pub symbol: Option<String>,
}

#[derive(Insertable, Clone)]
#[diesel(table_name = languages)]
pub struct CreateLanguage {
    /// Language name, e.g. "Russian".
    pub name: Option<String>,
    /// Short code, e.g. "ru".
    pub symbol: Option<String>,
}

// TODO: TMP ToSchema derive for documentation purposes
#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone, ToSchema)]
#[diesel(table_name = organisation_types)]
pub struct OrganisationType {
    /// Organisation type ID.
    pub id: Uuid,
    /// Type key (`type` column), e.g. "embassy".
    #[diesel(column_name = type_)]
    pub org_type: String,
    /// Map pin / badge colour as a hex string, e.g. "#4A6D8C".
    pub color: Option<String>,
    /// Human-readable name, e.g. "Embassy".
    pub title: Option<String>,
    /// Stable machine key used by filters and stats (`embassy`, `business`, `helper`, `community`, `volunteer`); unique when set.
    pub slug: Option<String>,
}

impl OrganisationType {
    pub fn create(
        conn: &mut PgConnection,
        record: &CreateOrganisationType,
    ) -> Result<Self, AppError> {
        let result = diesel::insert_into(organisation_types::table)
            .values(record)
            .get_result::<OrganisationType>(conn)?;

        Ok(result)
    }

    pub fn get_by_id(conn: &mut PgConnection, id: &Uuid) -> Result<Self, AppError> {
        let result = organisation_types::table
            .find(id)
            .get_result::<OrganisationType>(conn)?;

        Ok(result)
    }

    pub fn get_all(conn: &mut PgConnection) -> Result<Vec<Self>, AppError> {
        let result = organisation_types::table.load::<OrganisationType>(conn)?;

        Ok(result)
    }
}

#[derive(Insertable, Clone)]
#[diesel(table_name = organisation_types)]
pub struct CreateOrganisationType {
    /// Type key (`type` column), e.g. "embassy".
    #[diesel(column_name = type_)]
    pub org_type: String,
    /// Map pin / badge colour as a hex string, e.g. "#4A6D8C".
    pub color: Option<String>,
    /// Human-readable name, e.g. "Embassy".
    pub title: Option<String>,
    /// Stable machine key used by filters and stats (`embassy`, `business`, `helper`, `community`, `volunteer`); unique when set.
    pub slug: Option<String>,
}
