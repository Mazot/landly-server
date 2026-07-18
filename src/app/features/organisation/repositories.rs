use super::entities::{CreateOrganisation, Organisation, OrganisationStatus, UpdateOrganisation};
use crate::{
    app::features::user::entities::{User, UserRole},
    error::AppError,
    utils::{
        cache::{CacheKeys, CacheService, TypedCache},
        db::DbPool,
    },
};
use bigdecimal::{BigDecimal, ToPrimitive};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::Arc,
    time::Duration,
};
use uuid::Uuid;

/// Hard cap of rows pulled from the DB before in-Rust distance sorting /
/// open-now filtering. Keeps a huge bbox from loading the whole table.
const SEARCH_SCAN_CAP: i64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchSort {
    Nearest,
    Recent,
    Verified,
}

pub trait OrganisationRepository: Send + Sync + 'static {
    fn fetch_organisations(
        &self,
        params: FetchOrganisationsRepositoryInput,
    ) -> Result<Vec<Organisation>, AppError>;

    fn search_organisations(
        &self,
        params: SearchOrganisationsRepositoryInput,
    ) -> Result<Vec<(Organisation, Option<f64>)>, AppError>;

    fn create_organisation(
        &self,
        params: CreateOrganisationRepositoryInput,
    ) -> Result<Organisation, AppError>;

    fn delete_organisation(&self, id: Uuid) -> Result<(), AppError>;

    fn fetch_organisation(&self, id: Uuid) -> Result<Organisation, AppError>;

    fn update_organisation(
        &self,
        id: Uuid,
        params: UpdateOrganisationRepositoryInput,
    ) -> Result<Organisation, AppError>;

    fn increment_visits(&self, id: Uuid) -> Result<i64, AppError>;

    /// Role of a user, for ownership/RBAC checks in the usecase layer.
    fn fetch_user_role(&self, user_id: Uuid) -> Result<UserRole, AppError>;
}

#[derive(Clone)]
pub struct OrganisationRepositoryImpl {
    pool: DbPool,
    cache_service: TypedCache<Arc<dyn CacheService>>,
}
impl OrganisationRepositoryImpl {
    pub fn new(pool: DbPool, cache_service: TypedCache<Arc<dyn CacheService>>) -> Self {
        Self {
            pool,
            cache_service,
        }
    }

    fn generate_filters_hash(params: &FetchOrganisationsRepositoryInput) -> String {
        let mut hasher = DefaultHasher::new();
        params.name.hash(&mut hasher);
        params.tel.hash(&mut hasher);
        params.email.hash(&mut hasher);
        params.address.hash(&mut hasher);
        params.location_country_id.hash(&mut hasher);
        params.founder_country_id.hash(&mut hasher);
        params.organisation_type_id.hash(&mut hasher);
        params.limit.hash(&mut hasher);
        params.offset.hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }
}

/// Great-circle distance in km between two (lat, lng) points.
pub fn haversine_km(a: (f64, f64), b: (f64, f64)) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;

    let (lat1, lng1) = a;
    let (lat2, lng2) = b;

    let d_lat = (lat2 - lat1).to_radians();
    let d_lng = (lng2 - lng1).to_radians();

    let h = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lng / 2.0).sin().powi(2);

    2.0 * EARTH_RADIUS_KM * h.sqrt().asin()
}

fn coords_of(org: &Organisation) -> Option<(f64, f64)> {
    let lat = org.latitude.as_ref().and_then(BigDecimal::to_f64)?;
    let lng = org.longitude.as_ref().and_then(BigDecimal::to_f64)?;

    Some((lat, lng))
}

impl OrganisationRepository for OrganisationRepositoryImpl {
    fn fetch_organisations(
        &self,
        params: FetchOrganisationsRepositoryInput,
    ) -> Result<Vec<Organisation>, AppError> {
        use crate::data::schema::organisations;
        use diesel::prelude::*;

        let hash = Self::generate_filters_hash(&params);
        let cache_key = CacheKeys::organisations_list(&hash);

        if let Some(cached) = self.cache_service.get::<Vec<Organisation>>(&cache_key)? {
            return Ok(cached);
        }

        let connection = &mut self.pool.get()?;
        let query = {
            let mut query = organisations::table
                .filter(organisations::status.eq(OrganisationStatus::Live.as_str()))
                .into_boxed();

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
                let ids =
                    Organisation::fetch_ids_by_location_country(connection, location_country_id)?;
                query = query.filter(organisations::id.eq_any(ids));
            }

            if let Some(founder_country_id) = params.founder_country_id {
                let ids =
                    Organisation::fetch_ids_by_founder_country(connection, founder_country_id)?;
                query = query.filter(organisations::id.eq_any(ids));
            }

            if let Some(organisation_type_id) = params.organisation_type_id {
                let ids =
                    Organisation::fetch_ids_by_organisation_type(connection, organisation_type_id)?;
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
            Some(Duration::from_secs(5 * 60)), // Cache for 5 minutes
        );

        Ok(organisations)
    }

    fn search_organisations(
        &self,
        params: SearchOrganisationsRepositoryInput,
    ) -> Result<Vec<(Organisation, Option<f64>)>, AppError> {
        use crate::data::schema::{organisation_types, organisations};
        use diesel::prelude::*;

        let connection = &mut self.pool.get()?;

        let mut query = organisations::table
            .filter(organisations::status.eq(OrganisationStatus::Live.as_str()))
            .filter(organisations::latitude.is_not_null())
            .filter(organisations::longitude.is_not_null())
            .into_boxed();

        // Effective bbox: explicit, or derived from origin + radius.
        let bbox = params.bbox.or_else(|| {
            let (lat, lng) = params.origin?;
            let radius = params.radius_km?;
            let lat_delta = radius / 111.0;
            let lng_delta = radius / (111.0 * lat.to_radians().cos().abs().max(0.01));

            Some((
                lat - lat_delta,
                lng - lng_delta,
                lat + lat_delta,
                lng + lng_delta,
            ))
        });

        if let Some((min_lat, min_lng, max_lat, max_lng)) = bbox {
            let to_dec = |v: f64| {
                BigDecimal::try_from(v).map_err(|_| {
                    AppError::UnprocessableEntity(
                        serde_json::json!({ "error": "Invalid bbox coordinate" }),
                    )
                })
            };

            query = query
                .filter(organisations::latitude.between(to_dec(min_lat)?, to_dec(max_lat)?))
                .filter(organisations::longitude.between(to_dec(min_lng)?, to_dec(max_lng)?));
        }

        if let Some(slugs) = &params.type_slugs {
            let type_ids = organisation_types::table
                .filter(organisation_types::slug.eq_any(slugs))
                .select(organisation_types::id)
                .load::<Uuid>(connection)?;
            query = query.filter(organisations::organisation_type_id.eq_any(type_ids));
        }

        if let Some(languages) = &params.languages {
            let langs: Vec<Option<String>> = languages.iter().cloned().map(Some).collect();
            query = query.filter(organisations::languages.overlaps_with(langs));
        }

        if params.verified_only {
            query = query.filter(organisations::verified.eq(true));
        }

        if let Some(min_rating) = params.min_rating {
            query = query.filter(organisations::rating_avg.ge(min_rating));
        }

        if let Some(added_by) = &params.added_by {
            query = query.filter(organisations::added_by.eq(added_by));
        }

        if let Some(cost) = &params.cost {
            query = query.filter(organisations::cost.eq(cost));
        }

        let mut organisations = query
            .limit(SEARCH_SCAN_CAP)
            .load::<Organisation>(connection)?;

        // open-now filtering happens in Rust: hours are JSONB + timezone.
        if params.open_now {
            organisations.retain(|org| {
                super::presenters::compute_open_now(&org.opening_hours, &org.timezone)
                    .unwrap_or(false)
            });
        }

        let mut items: Vec<(Organisation, Option<f64>)> = organisations
            .into_iter()
            .map(|org| {
                let distance = match (params.origin, coords_of(&org)) {
                    (Some(origin), Some(coords)) => Some(haversine_km(origin, coords)),
                    _ => None,
                };
                (org, distance)
            })
            .collect();

        // Radius post-filter (bbox is a square approximation).
        if let (Some(_), Some(radius)) = (params.origin, params.radius_km) {
            items.retain(|(_, d)| d.map(|d| d <= radius).unwrap_or(false));
        }

        match params.sort {
            SearchSort::Nearest => items.sort_by(|a, b| {
                let da = a.1.unwrap_or(f64::MAX);
                let db = b.1.unwrap_or(f64::MAX);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            }),
            SearchSort::Recent => items.sort_by_key(|item| std::cmp::Reverse(item.0.created_at)),
            SearchSort::Verified => items.sort_by(|a, b| {
                b.0.verified
                    .cmp(&a.0.verified)
                    .then_with(|| b.0.created_at.cmp(&a.0.created_at))
            }),
        }

        let offset = params.offset.max(0) as usize;
        let limit = params.limit.max(0) as usize;
        let items = items.into_iter().skip(offset).take(limit).collect();

        Ok(items)
    }

    fn create_organisation(
        &self,
        params: CreateOrganisationRepositoryInput,
    ) -> Result<Organisation, AppError> {
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
                founder_country_id: params.founder_country_id,
                created_by: Some(params.created_by),
                status: params.status,
                added_by: params.added_by,
                city: params.city,
                website: params.website,
                telegram: params.telegram,
                whatsapp: params.whatsapp,
                services: params.services.into_iter().map(Some).collect(),
                languages: params.languages.into_iter().map(Some).collect(),
                opening_hours: params.opening_hours,
                timezone: params.timezone,
                cost: params.cost,
                google_place_id: params.google_place_id,
            },
        )?;

        // TODO: We should invalidate the cache from CacheInvalidationMiddleware
        // but for now we do it here to ensure the cache is cleared after creation.
        let _ = self
            .cache_service
            .invalidate_pattern(&CacheKeys::organisation_pattern());

        let cache_key = CacheKeys::organisation_by_id(&new_organisation.id);
        let _ = self
            .cache_service
            .set::<Organisation>(&cache_key, &new_organisation, None);

        Ok(new_organisation)
    }

    fn delete_organisation(&self, id: Uuid) -> Result<(), AppError> {
        let connection = &mut self.pool.get()?;
        Organisation::delete(connection, id)?;

        let _ = self
            .cache_service
            .invalidate_pattern(&CacheKeys::organisation_pattern());

        Ok(())
    }

    fn fetch_organisation(&self, id: Uuid) -> Result<Organisation, AppError> {
        let cache_key = CacheKeys::organisation_by_id(&id);

        if let Some(cached) = self.cache_service.get::<Organisation>(&cache_key)? {
            return Ok(cached);
        }

        let connection = &mut self.pool.get()?;
        let organisation = Organisation::fetch_by_id(connection, id)?;

        self.cache_service
            .set::<Organisation>(&cache_key, &organisation, None)?;

        Ok(organisation)
    }

    fn update_organisation(
        &self,
        id: Uuid,
        params: UpdateOrganisationRepositoryInput,
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
                founder_country_id: params.founder_country_id,
                city: params.city,
                website: params.website,
                telegram: params.telegram,
                whatsapp: params.whatsapp,
                services: params.services.map(|s| s.into_iter().map(Some).collect()),
                languages: params.languages.map(|l| l.into_iter().map(Some).collect()),
                opening_hours: params.opening_hours,
                timezone: params.timezone,
                cost: params.cost,
                updated_at: chrono::Utc::now().naive_utc(),
            },
        )?;

        // TODO: We should invalidate the cache from CacheInvalidationMiddleware
        // but for now we do it here to ensure the cache is cleared after creation.
        let _ = self
            .cache_service
            .invalidate_pattern(&CacheKeys::organisation_pattern());

        let cache_key = CacheKeys::organisation_by_id(&updated_organisation.id);
        let _ = self
            .cache_service
            .set::<Organisation>(&cache_key, &updated_organisation, None);

        Ok(updated_organisation)
    }

    fn increment_visits(&self, id: Uuid) -> Result<i64, AppError> {
        let connection = &mut self.pool.get()?;
        let visits = Organisation::increment_visits(connection, id)?;

        // Counters must not go stale in the per-id cache; drop just that key.
        let _ = self
            .cache_service
            .delete(&CacheKeys::organisation_by_id(&id));

        Ok(visits)
    }

    fn fetch_user_role(&self, user_id: Uuid) -> Result<UserRole, AppError> {
        let connection = &mut self.pool.get()?;

        User::fetch_role(connection, user_id)
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
    pub founder_country_id: Option<Uuid>,
    pub city: Option<String>,
    pub website: Option<String>,
    pub telegram: Option<String>,
    pub whatsapp: Option<String>,
    pub services: Option<Vec<String>>,
    pub languages: Option<Vec<String>>,
    pub opening_hours: Option<serde_json::Value>,
    pub timezone: Option<String>,
    pub cost: Option<String>,
}

pub struct FetchOrganisationsRepositoryInput {
    pub name: Option<String>,
    pub tel: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub organisation_type_id: Option<Uuid>,
    pub founder_country_id: Option<Uuid>,
    pub limit: i64,
    pub offset: i64,
}

pub struct SearchOrganisationsRepositoryInput {
    /// (min_lat, min_lng, max_lat, max_lng)
    pub bbox: Option<(f64, f64, f64, f64)>,
    /// (lat, lng) — enables distance computation and `nearest` sort
    pub origin: Option<(f64, f64)>,
    pub radius_km: Option<f64>,
    pub type_slugs: Option<Vec<String>>,
    pub open_now: bool,
    pub languages: Option<Vec<String>>,
    pub verified_only: bool,
    pub min_rating: Option<f64>,
    pub added_by: Option<String>,
    pub cost: Option<String>,
    pub sort: SearchSort,
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
    pub founder_country_id: Option<Uuid>,
    pub created_by: Uuid,
    pub status: String,
    pub added_by: Option<String>,
    pub city: Option<String>,
    pub website: Option<String>,
    pub telegram: Option<String>,
    pub whatsapp: Option<String>,
    pub services: Vec<String>,
    pub languages: Vec<String>,
    pub opening_hours: Option<serde_json::Value>,
    pub timezone: Option<String>,
    pub cost: Option<String>,
    pub google_place_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_zero_distance() {
        let berlin = (52.52, 13.405);
        assert!(haversine_km(berlin, berlin) < 1e-9);
    }

    #[test]
    fn test_haversine_berlin_munich() {
        let berlin = (52.52, 13.405);
        let munich = (48.1351, 11.582);
        let d = haversine_km(berlin, munich);

        // Real-world distance is ~504 km
        assert!((d - 504.0).abs() < 10.0, "got {}", d);
    }
}
