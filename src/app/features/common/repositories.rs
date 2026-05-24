use crate::data::models::{Country, CreateOrganisationType, OrganisationType};
use crate::error::AppError;
use crate::utils::db::DbPool;
use serde_json::json;
use uuid::Uuid;

pub trait CommonRepository: Send + Sync + 'static {
    fn get_country(&self, params: GetCountryRepositoryInput) -> Result<Country, AppError>;

    fn get_all_countries(
        &self,
        params: GetAllCountriesRepositoryInput,
    ) -> Result<Vec<Country>, AppError>;

    fn get_organisation_type(&self, id: &Uuid) -> Result<OrganisationType, AppError>;

    fn get_all_organisation_types(&self) -> Result<Vec<OrganisationType>, AppError>;

    fn create_organisation_type(
        &self,
        params: CreateOrganisationTypeRepositoryInput,
    ) -> Result<OrganisationType, AppError>;
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

        match params.id {
            Some(id) => Country::get_by_id(connection, &id),
            None => match params.name {
                Some(name) => Country::get_by_name(connection, name.as_str()),
                None => Err(AppError::NotFound(
                    json!({ "error": "Empty request params" }),
                )),
            },
        }
    }

    fn get_all_countries(
        &self,
        params: GetAllCountriesRepositoryInput,
    ) -> Result<Vec<Country>, AppError> {
        use crate::data::schema::countries;
        use diesel::prelude::*;

        let connection = &mut self.pool.get()?;
        let mut query = countries::table.select(countries::all_columns).into_boxed();

        if let Some(ref name) = params.name {
            let pattern = format!("%{}%", name);
            query = query.filter(countries::name.ilike(pattern));
        }

        let countries_list = query
            .order(countries::name.asc())
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

    fn create_organisation_type(
        &self,
        params: CreateOrganisationTypeRepositoryInput,
    ) -> Result<OrganisationType, AppError> {
        let connection = &mut self.pool.get()?;
        let new_org_type = OrganisationType::create(
            connection,
            &CreateOrganisationType {
                org_type: params.org_type,
                color: Some(params.color),
                title: Some(params.title),
            },
        )?;

        Ok(new_org_type)
    }
}

pub struct GetCountryRepositoryInput {
    pub id: Option<Uuid>,
    pub name: Option<String>,
}

pub struct GetAllCountriesRepositoryInput {
    pub limit: i64,
    pub offset: i64,
    pub name: Option<String>,
}

pub struct CreateOrganisationTypeRepositoryInput {
    pub org_type: String,
    pub color: String,
    pub title: String,
}
