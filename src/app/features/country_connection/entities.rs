use crate::{
    data::schema::countries_connections,
    data::models::Country
};
use crate::error::*;
use serde::{Deserialize, Serialize};
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Debug, Associations, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
// #[diesel(belongs_to(Organisation, foreign_key = embassy_org_id))]
// #[diesel(belongs_to(Organisation, foreign_key = consulate_org_id))]
#[diesel(belongs_to(Country, foreign_key = location_country_id))]
#[diesel(table_name = countries_connections)]
pub struct CountryConnection {
    pub id: Uuid,
    pub embassy_org_id: Option<Uuid>,
    pub consulate_org_id: Option<Uuid>,
    pub location_country_id: Option<Uuid>,
    pub common_info: Option<String>,
}

#[derive(Insertable, Clone)]
#[diesel(table_name = countries_connections)]
pub struct CreateCountryConnection {
    pub embassy_org_id: Option<Uuid>,
    pub consulate_org_id: Option<Uuid>,
    pub common_info: Option<String>,
    pub location_country_id: Option<Uuid>,
}

#[derive(AsChangeset)]
#[diesel(table_name = countries_connections)]
pub struct UpdateCountryConnection {
    pub embassy_org_id: Option<Uuid>,
    pub consulate_org_id: Option<Uuid>,
    pub common_info: Option<String>,
    pub location_country_id: Option<Uuid>,
}

impl CountryConnection {
    pub fn create(
        conn: &mut PgConnection,
        record: &CreateCountryConnection,
    ) -> Result<Self, AppError> {
        let result = diesel::insert_into(countries_connections::table)
            .values(record)
            .returning(CountryConnection::as_select())
            .get_result::<CountryConnection>(conn)?;

        Ok(result)
    }

    pub fn update(
        conn: &mut PgConnection,
        connection_id: Uuid,
        record: &UpdateCountryConnection,
    ) -> Result<Self, AppError> {
        let c = countries_connections::table
            .find(connection_id);
        let result = diesel::update(c)
            .set(record)
            .returning(CountryConnection::as_select())
            .get_result::<CountryConnection>(conn)?;

        Ok(result)
    }

    pub fn delete(
        conn: &mut PgConnection,
        connection_id: Uuid,
    ) -> Result<(), AppError> {
        let c = countries_connections::table
            .find(connection_id);
        diesel::delete(c).execute(conn)?;

        Ok(())
    }

    pub fn fetch_by_id(
        conn: &mut PgConnection,
        connection_id: Uuid,
    ) -> Result<Self, AppError> {
        let result = countries_connections::table
            .find(connection_id)
            .select(CountryConnection::as_select())
            .get_result::<CountryConnection>(conn)?;

        Ok(result)
    }

    pub fn fetch_by_location_country(
        conn: &mut PgConnection,
        country_id: Uuid,
    ) -> Result<Vec<Self>, AppError> {
        let result = countries_connections::table
            .filter(countries_connections::location_country_id.eq(country_id))
            .select(CountryConnection::as_select())
            .load::<CountryConnection>(conn)?;

        Ok(result)
    }

    pub fn fetch_with_filters(
        conn: &mut PgConnection,
        embassy_org_id: Option<Uuid>,
        consulate_org_id: Option<Uuid>,
        location_country_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, AppError> {
        let mut query = countries_connections::table.into_boxed();

        if let Some(embassy_id) = embassy_org_id {
            query = query.filter(countries_connections::embassy_org_id.eq(embassy_id));
        }

        if let Some(consulate_id) = consulate_org_id {
            query = query.filter(countries_connections::consulate_org_id.eq(consulate_id));
        }

        if let Some(country_id) = location_country_id {
            query = query.filter(countries_connections::location_country_id.eq(country_id));
        }

        let result = query
            .limit(limit)
            .offset(offset)
            .select(CountryConnection::as_select())
            .load::<CountryConnection>(conn)?;

        Ok(result)
    }
}
