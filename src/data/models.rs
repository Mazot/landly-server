use super::schema::*;
use crate::error::*;
use serde::{Deserialize, Serialize, Serializer};
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = countries)]
pub struct Country {
    pub id: Uuid,
    pub name: String,
    pub geo_json: Option<serde_json::Value>,
    pub flag: Option<String>,
    pub capital_city: Option<String>,
    pub description: Option<String>,
}

#[derive(AsChangeset)]
#[diesel(table_name = countries)]
pub struct UpdateCountry {
    pub name: Option<String>,
    pub geo_json: Option<serde_json::Value>,
    pub flag: Option<String>,
    pub capital_city: Option<String>,
    pub description: Option<String>,
}

impl Country {
    pub fn create(
        conn: &mut PgConnection,
        record: &Country,
    ) -> Result<Self, AppError> {
        let result = diesel::insert_into(countries::table)
            .values(record)
            .get_result::<Country>(conn)?;

        Ok(result)
    }

    pub fn update (
        conn: &mut PgConnection,
        country_id: Uuid,
        record: &UpdateCountry,
    ) -> Result<Self, AppError> {
        let c = countries::table
            .find(country_id);
        let result = diesel::update(c)
            .set(record)
            .get_result::<Country>(conn)?;

        Ok(result)
    }

    pub fn delete (
        conn: &mut PgConnection,
        country_id: Uuid,
    ) -> Result<(), AppError> {
        let c = countries::table
            .find(country_id);
        diesel::delete(c).execute(conn)?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = languages)]
pub struct Language {
    pub id: Uuid,
    pub name: String,
    pub symbol: Option<String>,
}

impl Language {
    pub fn create(
        conn: &mut PgConnection,
        record: &Language,
    ) -> Result<Self, AppError> {
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
        let l = languages::table
            .find(language_id);
        let result = diesel::update(l)
            .set(record)
            .get_result::<Language>(conn)?;

        Ok(result)
    }

    pub fn delete(
        conn: &mut PgConnection,
        language_id: Uuid,
    ) -> Result<(), AppError> {
        let l = languages::table
            .find(language_id);
        diesel::delete(l).execute(conn)?;

        Ok(())
    }
}


#[derive(AsChangeset)]
#[diesel(table_name = languages)]
pub struct UpdateLanguage {
    pub name: Option<String>,
    pub symbol: Option<String>,
}

#[derive(Insertable, Clone)]
#[diesel(table_name = languages)]
pub struct CreateLanguage {
    pub name: Option<String>,
    pub symbol: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = organisation_types)]
pub struct OrganisationType {
    pub id: Uuid,
    #[diesel(column_name = type_)]
    pub org_type: String,
    pub color: Option<String>,
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
}

#[derive(Insertable, Clone)]
#[diesel(table_name = organisation_types)]
pub struct CreateOrganisationType {
    #[diesel(column_name = type_)]
    pub org_type: String,
    pub color: Option<String>,
}
