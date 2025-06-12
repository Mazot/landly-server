use crate::data::{
    schema::{organisations},
    models::{Country, OrganisationType},
};
use crate::error::*;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::NaiveDateTime;

#[derive(Debug, Associations, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(belongs_to(Country, foreign_key = location_country_id))]
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
}

impl Organisation {
    pub fn create(
        conn: &mut PgConnection,
        record: &CreateOrganisation,
    ) -> Result<Self, AppError> {
        let result = diesel::insert_into(organisations::table)
            .values(record)
            .get_result::<Organisation>(conn)?;

        Ok(result)
    }

    pub fn update (
        conn: &mut PgConnection,
        organisation_id: Uuid,
        record: &UpdateOrganisation,
    ) -> Result<Self, AppError> {
        let o = organisations::table
            .find(organisation_id);
        let result = diesel::update(o)
            .set(record)
            .get_result::<Organisation>(conn)?;

        Ok(result)
    }

    pub fn delete (
        conn: &mut PgConnection,
        organisation_id: Uuid,
    ) -> Result<(), AppError> {
        let o = organisations::table
            .find(organisation_id);
        diesel::delete(o).execute(conn)?;
        Ok(())
    }

    pub fn fetch_by_id(
        conn: &mut PgConnection,
        organisation_id: Uuid,
    ) -> Result<Self, AppError> {
        let result = organisations::table
            .find(organisation_id)
            .get_result::<Organisation>(conn)?;

        Ok(result)
    }

    pub fn fetch_by_country(
        conn: &mut PgConnection,
        country_id: Uuid,
    ) -> Result<Vec<Self>, AppError> {
        let result = organisations::table
            .filter(organisations::location_country_id.eq(country_id))
            .load::<Organisation>(conn)?;

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
}

#[derive(AsChangeset)]
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
}
