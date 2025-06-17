use crate::data::models::{Country, OrganisationType};
use crate::error::AppError;
use crate::utils::db::DbPool;
use serde_json::json;
use uuid::Uuid;

pub trait CommonRepository: Send + Sync + 'static {
    fn get_country(
        &self,
        params: GetCountryRepositoryInput
    ) -> Result<Country, AppError>;

    fn get_all_countries(
        &self,
        params: GetAllCountriesRepositoryInput
    ) -> Result<Vec<Country>, AppError>;

    fn get_organisation_type(
        &self,
        id: &Uuid
    ) -> Result<OrganisationType, AppError>;

    fn get_all_organisation_types(
        &self
    ) -> Result<Vec<OrganisationType>, AppError>;
}

#[derive(Clone)]
pub struct CommonRepositoryImpl {
    pool: DbPool,
}
impl CommonRepositoryImpl {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

impl CommonRepository for CommonRepositoryImpl {
    fn get_country(&self, params: GetCountryRepositoryInput) -> Result<Country, AppError> {
        let connection = &mut self.pool.get()?;
        let country_result = match params.id {
            Some(id) => {
                Country::get_by_id(
                    connection,
                    &id
                )
            },
            None => {
                match params.name {
                    Some(name) => {
                        Country::get_by_name(
                            connection,
                            &name.as_str()
                        )
                    },
                    None => Err(AppError::NotFound(json!({ "error": "Empty request params" })))
                }
            }
        };

        country_result
    }

    fn get_all_countries(&self, params: GetAllCountriesRepositoryInput) -> Result<Vec<Country>, AppError> {
        use crate::data::schema::countries;
        use diesel::prelude::*;

        let connection = &mut self.pool.get()?;
        let countries_list = countries::table
            .select(countries::all_columns)
            .limit(params.limit)
            .offset(params.offset)
            .load::<Country>(connection)?;

        Ok(countries_list)
    }

    fn get_organisation_type(&self, id: &Uuid) -> Result<OrganisationType, AppError> {
        let connection = &mut self.pool.get()?;
        let org_type = OrganisationType::get_by_id(connection, id)?;

        Ok(org_type)
    }

    fn get_all_organisation_types(&self) -> Result<Vec<OrganisationType>, AppError> {
        let connection = &mut self.pool.get()?;
        let all_org_types = OrganisationType::get_all(connection)?;

        Ok(all_org_types)
    }
}

pub struct GetCountryRepositoryInput {
    pub id: Option<Uuid>,
    pub name: Option<String>,
}

pub struct GetAllCountriesRepositoryInput {
    pub limit: i64,
    pub offset: i64,
}
