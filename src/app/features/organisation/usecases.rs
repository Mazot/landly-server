use super::{
    entities::{AddedBy, Cost, Organisation, OrganisationStatus},
    presenters::OrganisationPresenter,
    repositories::{
        CreateOrganisationRepositoryInput, FetchOrganisationsRepositoryInput,
        OrganisationRepository, SearchOrganisationsRepositoryInput, SearchSort,
        UpdateOrganisationRepositoryInput,
    },
};
use crate::error::AppError;
use actix_web::HttpResponse;
use bigdecimal::BigDecimal;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct OrganisationUsecase {
    organisation_repo: Arc<dyn OrganisationRepository>,
    organisation_presenter: Arc<dyn OrganisationPresenter>,
}

impl OrganisationUsecase {
    pub fn new(
        organisation_repo: Arc<dyn OrganisationRepository>,
        organisation_presenter: Arc<dyn OrganisationPresenter>,
    ) -> Self {
        Self {
            organisation_repo,
            organisation_presenter,
        }
    }

    /// Update/delete are allowed for the creator or a moderator/admin.
    fn ensure_can_manage(
        &self,
        organisation: &Organisation,
        caller_user_id: Uuid,
    ) -> Result<(), AppError> {
        if organisation.created_by == Some(caller_user_id) {
            return Ok(());
        }

        let role = self.organisation_repo.fetch_user_role(caller_user_id)?;
        if role.is_moderator() {
            return Ok(());
        }

        Err(AppError::Forbidden(
            json!({ "error": "Only the creator or a moderator can modify this organisation" }),
        ))
    }

    pub fn create_organisation(
        &self,
        params: CreateOrganisationUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        if let Some(added_by) = params.added_by.as_deref() {
            AddedBy::try_from(added_by)?;
        }
        if let Some(cost) = params.cost.as_deref() {
            Cost::try_from(cost)?;
        }

        let new_organisation =
            self.organisation_repo
                .create_organisation(CreateOrganisationRepositoryInput {
                    name: params.name,
                    tel: params.tel,
                    email: params.email,
                    address: params.address,
                    description: params.description,
                    founder_country_id: params.founder_country_id,
                    location_country_id: params.location_country_id,
                    organisation_type_id: params.organisation_type_id,
                    latitude: params.latitude,
                    longitude: params.longitude,
                    created_by: params.created_by,
                    // New community submissions go through moderation.
                    status: OrganisationStatus::Pending.as_str().to_string(),
                    added_by: params.added_by,
                    city: params.city,
                    website: params.website,
                    telegram: params.telegram,
                    whatsapp: params.whatsapp,
                    services: params.services,
                    languages: params.languages,
                    opening_hours: params.opening_hours,
                    timezone: params.timezone,
                    cost: params.cost,
                    google_place_id: params.google_place_id,
                })?;
        let response = self.organisation_presenter.to_single_json(new_organisation);

        Ok(response)
    }

    pub fn update_organisation(
        &self,
        id: Uuid,
        caller_user_id: Uuid,
        params: UpdateOrganisationUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        if let Some(cost) = params.cost.as_deref() {
            Cost::try_from(cost)?;
        }

        let existing = self.organisation_repo.fetch_organisation(id)?;
        self.ensure_can_manage(&existing, caller_user_id)?;

        let updated_organisation = self.organisation_repo.update_organisation(
            id,
            UpdateOrganisationRepositoryInput {
                name: params.name,
                tel: params.tel,
                email: params.email,
                address: params.address,
                description: params.description,
                founder_country_id: params.founder_country_id,
                location_country_id: params.location_country_id,
                organisation_type_id: params.organisation_type_id,
                latitude: params.latitude,
                longitude: params.longitude,
                city: params.city,
                website: params.website,
                telegram: params.telegram,
                whatsapp: params.whatsapp,
                services: params.services,
                languages: params.languages,
                opening_hours: params.opening_hours,
                timezone: params.timezone,
                cost: params.cost,
            },
        )?;
        let response = self
            .organisation_presenter
            .to_single_json(updated_organisation);

        Ok(response)
    }

    pub fn fetch_organisations(
        &self,
        params: FetchOrganisationsUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        let organisations =
            self.organisation_repo
                .fetch_organisations(FetchOrganisationsRepositoryInput {
                    name: params.name,
                    tel: params.tel,
                    email: params.email,
                    location_country_id: params.location_country_id,
                    organisation_type_id: params.organisation_type_id,
                    founder_country_id: params.founder_country_id,
                    address: params.address,
                    limit: params.limit,
                    offset: params.offset,
                })?;
        let response = self.organisation_presenter.to_multi_json(organisations);

        Ok(response)
    }

    pub fn search_organisations(
        &self,
        params: SearchOrganisationsUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        if let Some(added_by) = params.added_by.as_deref() {
            AddedBy::try_from(added_by)?;
        }
        if let Some(cost) = params.cost.as_deref() {
            Cost::try_from(cost)?;
        }

        let sort = match params.sort.as_deref() {
            None => {
                if params.origin.is_some() {
                    SearchSort::Nearest
                } else {
                    SearchSort::Recent
                }
            }
            Some("nearest") => {
                if params.origin.is_none() {
                    return Err(AppError::UnprocessableEntity(
                        json!({ "error": "sort=nearest requires lat and lng" }),
                    ));
                }
                SearchSort::Nearest
            }
            Some("recent") => SearchSort::Recent,
            Some("verified") => SearchSort::Verified,
            Some(other) => {
                return Err(AppError::UnprocessableEntity(
                    json!({ "error": format!("Unknown sort: {}", other) }),
                ));
            }
        };

        let items =
            self.organisation_repo
                .search_organisations(SearchOrganisationsRepositoryInput {
                    bbox: params.bbox,
                    origin: params.origin,
                    radius_km: params.radius_km,
                    type_slugs: params.type_slugs,
                    open_now: params.open_now,
                    languages: params.languages,
                    verified_only: params.verified_only,
                    min_rating: params.min_rating,
                    added_by: params.added_by,
                    cost: params.cost,
                    sort,
                    limit: params.limit,
                    offset: params.offset,
                })?;

        Ok(self.organisation_presenter.to_search_json(items))
    }

    pub fn delete_organisation(
        &self,
        id: Uuid,
        caller_user_id: Uuid,
    ) -> Result<HttpResponse, AppError> {
        let existing = self.organisation_repo.fetch_organisation(id)?;
        self.ensure_can_manage(&existing, caller_user_id)?;

        self.organisation_repo.delete_organisation(id)?;
        let response = self.organisation_presenter.to_http_res();

        Ok(response)
    }

    pub fn fetch_organisation(&self, id: Uuid) -> Result<HttpResponse, AppError> {
        let organisation = self.organisation_repo.fetch_organisation(id)?;
        let response = self.organisation_presenter.to_single_json(organisation);

        Ok(response)
    }

    pub fn visit_organisation(&self, id: Uuid) -> Result<HttpResponse, AppError> {
        let visits = self.organisation_repo.increment_visits(id)?;

        Ok(self.organisation_presenter.to_visits_json(visits))
    }
}

pub struct UpdateOrganisationUsecaseInput {
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

pub struct CreateOrganisationUsecaseInput {
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

pub struct FetchOrganisationsUsecaseInput {
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

pub struct SearchOrganisationsUsecaseInput {
    pub bbox: Option<(f64, f64, f64, f64)>,
    pub origin: Option<(f64, f64)>,
    pub radius_km: Option<f64>,
    pub type_slugs: Option<Vec<String>>,
    pub open_now: bool,
    pub languages: Option<Vec<String>>,
    pub verified_only: bool,
    pub min_rating: Option<f64>,
    pub added_by: Option<String>,
    pub cost: Option<String>,
    pub sort: Option<String>,
    pub limit: i64,
    pub offset: i64,
}
