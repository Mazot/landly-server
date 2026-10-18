use crate::app::features::user::entities::{User, UserRole};
use crate::data::models::{Country, CreateOrganisationType, Language, OrganisationType};
use crate::error::AppError;
use crate::utils::cache::{CacheKeys, CacheService, TypedCache};
use crate::utils::db::DbPool;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

pub trait CommonRepository: Send + Sync + 'static {
    fn get_country(&self, params: GetCountryRepositoryInput) -> Result<Country, AppError>;

    fn get_all_countries(
        &self,
        params: GetAllCountriesRepositoryInput,
    ) -> Result<Vec<Country>, AppError>;

    /// Country + live-organisation counts grouped by org type slug
    /// (design: country-full.jsx).
    fn get_country_detail(&self, id: Uuid) -> Result<(Country, Vec<(String, i64)>), AppError>;

    /// Languages ordered by name; their ids are what `language_ids` /
    /// `languages_ids` on people and users reference.
    fn get_all_languages(
        &self,
        params: GetAllLanguagesRepositoryInput,
    ) -> Result<Vec<Language>, AppError>;

    fn get_organisation_type(&self, id: &Uuid) -> Result<OrganisationType, AppError>;

    fn get_all_organisation_types(&self) -> Result<Vec<OrganisationType>, AppError>;

    fn create_organisation_type(
        &self,
        params: CreateOrganisationTypeRepositoryInput,
    ) -> Result<OrganisationType, AppError>;

    /// Role of a user, for RBAC checks in the usecase layer.
    fn fetch_user_role(&self, user_id: Uuid) -> Result<UserRole, AppError>;
}

#[derive(Clone)]
pub struct CommonRepositoryImpl {
    pool: DbPool,
    cache_service: TypedCache<Arc<dyn CacheService>>,
}
impl CommonRepositoryImpl {
    pub fn new(pool: DbPool, cache_service: TypedCache<Arc<dyn CacheService>>) -> Self {
        Self {
            pool,
            cache_service,
        }
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

    fn get_country_detail(&self, id: Uuid) -> Result<(Country, Vec<(String, i64)>), AppError> {
        use crate::data::schema::{organisation_types, organisations};
        use diesel::prelude::*;

        let connection = &mut self.pool.get()?;
        let country = Country::get_by_id(connection, &id)?;

        let rows: Vec<(Option<String>, i64)> = organisations::table
            .filter(organisations::location_country_id.eq(id))
            .filter(organisations::status.eq("live"))
            .left_join(organisation_types::table)
            .group_by(organisation_types::slug)
            .select((
                organisation_types::slug.nullable(),
                diesel::dsl::count_star(),
            ))
            .load(connection)?;

        let by_type = rows
            .into_iter()
            .map(|(slug, count)| (slug.unwrap_or_else(|| "other".to_string()), count))
            .collect();

        Ok((country, by_type))
    }

    fn get_all_languages(
        &self,
        params: GetAllLanguagesRepositoryInput,
    ) -> Result<Vec<Language>, AppError> {
        use crate::data::schema::languages;
        use diesel::prelude::*;

        // Languages are seeded outside the API (scripts/crates country_parser),
        // so there is no write path here to invalidate
        // `CacheKeys::common_languages_pattern()` from; the cached
        // GET /common/languages response expires with its TTL.
        let connection = &mut self.pool.get()?;
        let mut query = languages::table.select(languages::all_columns).into_boxed();

        if let Some(ref name) = params.name {
            let pattern = format!("%{}%", name);
            query = query.filter(languages::name.ilike(pattern));
        }

        let languages_list = query
            .order((languages::name.asc(), languages::id.asc()))
            .limit(params.limit)
            .offset(params.offset)
            .load::<Language>(connection)?;

        Ok(languages_list)
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
                slug: params.slug,
            },
        )?;

        // The GET /common/org_types response is cached by the request-reply
        // middleware; drop it so the new type is visible immediately.
        let _ = self
            .cache_service
            .invalidate_pattern(&CacheKeys::common_org_types_pattern());

        Ok(new_org_type)
    }

    fn fetch_user_role(&self, user_id: Uuid) -> Result<UserRole, AppError> {
        let connection = &mut self.pool.get()?;

        User::fetch_role(connection, user_id)
    }
}

pub struct GetCountryRepositoryInput {
    /// Look up by ID (takes precedence over `name`).
    pub id: Option<Uuid>,
    /// Look up by exact name when no ID is given.
    pub name: Option<String>,
}

pub struct GetAllCountriesRepositoryInput {
    /// Page size (already clamped by the controller).
    pub limit: i64,
    /// Rows to skip.
    pub offset: i64,
    /// Case-insensitive substring filter on the name.
    pub name: Option<String>,
}

pub struct GetAllLanguagesRepositoryInput {
    /// Page size (already clamped by the controller).
    pub limit: i64,
    /// Rows to skip.
    pub offset: i64,
    /// Case-insensitive substring filter on the name.
    pub name: Option<String>,
}

pub struct CreateOrganisationTypeRepositoryInput {
    /// Type key.
    pub org_type: String,
    /// Hex colour.
    pub color: String,
    /// Human-readable name.
    pub title: String,
    /// Optional unique slug.
    pub slug: Option<String>,
}
