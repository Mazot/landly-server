use super::entities::{CreateOrganisation, Organisation, UpdateOrganisation};
use crate::{
    error::AppError,
    utils::{cache::{CacheKeys, CacheService, TypedCache}, db::DbPool}
};
use std::{
    collections::hash_map::DefaultHasher,
    time::Duration,
    hash::{Hash, Hasher},
    sync::Arc,
};
use uuid::Uuid;
use bigdecimal::BigDecimal;

pub trait OrganisationRepository: Send + Sync + 'static {
    fn fetch_organisations(
        &self,
        params: FetchOrganisationsRepositoryInput
    ) -> Result<Vec<Organisation>, AppError>;

    fn create_organisation(
        &self,
        params: CreateOrganisationRepositoryInput
    ) -> Result<Organisation, AppError>;

    fn delete_organisation(
        &self,
        id: Uuid
    ) -> Result<(), AppError>;

    fn fetch_organisation(
        &self,
        id: Uuid
    ) -> Result<Organisation, AppError>;

    fn update_organisation(
        &self,
        id: Uuid,
        params: UpdateOrganisationRepositoryInput
    ) -> Result<Organisation, AppError>;
}

#[derive(Clone)]
pub struct OrganisationRepositoryImpl {
    pool: DbPool,
    cache_service: TypedCache<Arc<dyn CacheService>>,
}
impl OrganisationRepositoryImpl {
    pub fn new(pool: DbPool, cache_service: TypedCache<Arc<dyn CacheService>>) -> Self {
        Self { pool, cache_service }
    }

    fn generate_filters_hash(params: &FetchOrganisationsRepositoryInput) -> String {
        let mut hasher = DefaultHasher::new();
        params.name.hash(&mut hasher);
        params.tel.hash(&mut hasher);
        params.email.hash(&mut hasher);
        params.address.hash(&mut hasher);
        params.location_country_id.hash(&mut hasher);
        params.organisation_type_id.hash(&mut hasher);
        params.limit.hash(&mut hasher);
        params.offset.hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }
}

impl OrganisationRepository for OrganisationRepositoryImpl {
    fn fetch_organisations(&self, params: FetchOrganisationsRepositoryInput) -> Result<Vec<Organisation>, AppError> {
        use crate::data::schema::organisations;
        use diesel::prelude::*;

        let hash = Self::generate_filters_hash(&params);
        let cache_key = CacheKeys::organisations_list(&hash);

        if let Some(cached) = self.cache_service.get::<Vec<Organisation>>(&cache_key)? {
            return Ok(cached);
        }

        let connection = &mut self.pool.get()?;
        let query = {
            let mut query = organisations::table.into_boxed();

            if let Some(name) = &params.name {
                query = query.filter(organisations::name.ilike(format!("%{}%", name)));
            }

            if let Some(tel) = &params.tel {
                query = query.filter(organisations::tel.ilike(format!("%{}%", tel)));
            }

            if let Some(email) = &params.email {
                query = query.filter(organisations::email.ilike(format!("%{}%", email)));
            }

            if let Some(address) = &params.address {
                query = query.filter(organisations::address.ilike(format!("%{}%", address)));
            }

            if let Some(location_country_id) = params.location_country_id {
                let ids = Organisation::fetch_ids_by_location_country(connection, location_country_id)?;
                query = query.filter(organisations::id.eq_any(ids));
            }

            if let Some(organisation_type_id) = params.organisation_type_id {
                let ids = Organisation::fetch_ids_by_organisation_type(connection, organisation_type_id)?;
                query = query.filter(organisations::id.eq_any(ids));
            }

            query
        };

        let organisations = query
            .limit(params.limit)
            .offset(params.offset)
            .load::<Organisation>(connection)?;

        let _ = self.cache_service.set::<Vec<Organisation>>(
            &cache_key,
            &organisations,
            Some(Duration::from_secs(5 * 60)) // Cache for 5 minutes
        );

        Ok(organisations)
    }

    fn create_organisation(&self, params: CreateOrganisationRepositoryInput) -> Result<Organisation, AppError> {
        let connection = &mut self.pool.get()?;
        let new_organisation = Organisation::create(
            connection,
            &CreateOrganisation {
                name: params.name,
                tel: params.tel,
                email: params.email,
                address: params.address,
                description: params.description,
                location_country_id: params.location_country_id,
                organisation_type_id: params.organisation_type_id,
                latitude: params.latitude,
                longitude: params.longitude,
            }
        )?;

        // TODO: We should invalidate the cache from CacheInvalidationMiddleware
        // but for now we do it here to ensure the cache is cleared after creation.
        let _ = self.cache_service.invalidate_pattern(&CacheKeys::organisation_pattern());

        let cache_key = CacheKeys::organisation_by_id(&new_organisation.id);
        let _ = self.cache_service.set::<Organisation>(
            &cache_key,
            &new_organisation,
            None
        );

        Ok(new_organisation)
    }

    fn delete_organisation(&self, id: Uuid) -> Result<(), AppError> {
        let connection = &mut self.pool.get()?;
        Organisation::delete(connection, id)?;

        Ok(())
    }

    fn fetch_organisation(&self, id: Uuid) -> Result<Organisation, AppError> {
        let connection = &mut self.pool.get()?;
        let organisation = Organisation::fetch_by_id(connection, id)?;

        // TODO: We should invalidate the cache from CacheInvalidationMiddleware
        // but for now we do it here to ensure the cache is cleared after creation.
        let _ = self.cache_service.invalidate_pattern(&CacheKeys::organisation_pattern());

        Ok(organisation)
    }

    fn update_organisation(
        &self,
        id: Uuid,
        params: UpdateOrganisationRepositoryInput
    ) -> Result<Organisation, AppError> {
        let connection = &mut self.pool.get()?;
        let updated_organisation = Organisation::update(
            connection,
            id,
            &UpdateOrganisation {
                name: params.name,
                tel: params.tel,
                email: params.email,
                address: params.address,
                description: params.description,
                location_country_id: params.location_country_id,
                organisation_type_id: params.organisation_type_id,
                longitude: params.longitude,
                latitude: params.latitude,
                updated_at: chrono::Utc::now().naive_utc(),
            }
        )?;

        // TODO: We should invalidate the cache from CacheInvalidationMiddleware
        // but for now we do it here to ensure the cache is cleared after creation.
        let _ = self.cache_service.invalidate_pattern(&CacheKeys::organisation_pattern());

        let cache_key = CacheKeys::organisation_by_id(&updated_organisation.id);
        let _ = self.cache_service.set::<Organisation>(
            &cache_key,
            &updated_organisation,
            None
        );

        Ok(updated_organisation)
    }
}

pub struct UpdateOrganisationRepositoryInput {
    pub name: Option<String>,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub description: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
    pub latitude: Option<BigDecimal>,
    pub longitude: Option<BigDecimal>,
}

pub struct FetchOrganisationsRepositoryInput {
    pub name: Option<String>,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
    pub limit: i64,
    pub offset: i64,
}

pub struct CreateOrganisationRepositoryInput {
    pub name: String,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub description: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
    pub latitude: Option<BigDecimal>,
    pub longitude: Option<BigDecimal>,
}
