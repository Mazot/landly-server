use super::{
    entities::{AddedBy, Cost, Organisation, OrganisationStatus},
    presenters::OrganisationPresenter,
    repositories::{
        CreateOrganisationRepositoryInput, FetchOrganisationsRepositoryInput,
        OrganisationRepository, SearchOrganisationsRepositoryInput, SearchSort,
        UpdateOrganisationRepositoryInput,
    },
};
use crate::app::features::moderation::repositories::{
    ModerationRepository, SubmittedEventInput, TargetKind,
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
    moderation_repo: Arc<dyn ModerationRepository>,
}

impl OrganisationUsecase {
    pub fn new(
        organisation_repo: Arc<dyn OrganisationRepository>,
        organisation_presenter: Arc<dyn OrganisationPresenter>,
        moderation_repo: Arc<dyn ModerationRepository>,
    ) -> Self {
        Self {
            organisation_repo,
            organisation_presenter,
            moderation_repo,
        }
    }

    /// Loose phone sanity check for the moderation auto-flags: "+" followed
    /// by 8..=15 digits (spaces/dashes tolerated). Not a validator — just a
    /// signal for the moderator.
    fn phone_looks_valid(tel: &str) -> bool {
        let digits: String = tel.chars().filter(|c| c.is_ascii_digit()).collect();

        tel.trim_start().starts_with('+') && (8..=15).contains(&digits.len())
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

    /// Creates a community submission: validated, owned by the caller and
    /// entered into moderation as `pending` (invisible to list/search).
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

        // Submit-time auto-checks for the moderation queue (best-effort:
        // a failed flag write must not fail the submission).
        let duplicate_nearby = self
            .organisation_repo
            .has_duplicate_nearby(
                &new_organisation.name,
                new_organisation.latitude.as_ref(),
                new_organisation.longitude.as_ref(),
            )
            .unwrap_or(false);
        let live_orgs_by_creator = self
            .organisation_repo
            .count_live_orgs_created_by(params.created_by)
            .unwrap_or(0);
        let _ = self.moderation_repo.record_submitted(SubmittedEventInput {
            target_kind: TargetKind::Org,
            target_id: new_organisation.id,
            flags: json!({
                "duplicateNearby": duplicate_nearby,
                "phoneLooksValid": new_organisation.tel.as_deref().map(Self::phone_looks_valid),
                "creatorLiveOrgs": live_orgs_by_creator,
                "trustedVolunteer": live_orgs_by_creator >= 3,
            }),
        });

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

    /// Map geo-search. Sort defaults to `nearest` when an origin point is
    /// given, otherwise `recent`; `sort=nearest` without an origin is a 422.
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

    /// Detail payload: the organisation plus its community check-in signals.
    pub fn fetch_organisation(&self, id: Uuid) -> Result<HttpResponse, AppError> {
        let organisation = self.organisation_repo.fetch_organisation(id)?;
        let signals = self.organisation_repo.fetch_community_signals(id)?;
        let response = self
            .organisation_presenter
            .to_single_with_community_json(organisation, signals);

        Ok(response)
    }

    /// Community check-in: "I was here"; optionally still-active + a tip.
    pub fn checkin_organisation(
        &self,
        id: Uuid,
        user_id: Uuid,
        still_active: bool,
        tip: Option<String>,
    ) -> Result<HttpResponse, AppError> {
        // Only live orgs accept check-ins.
        let organisation = self.organisation_repo.fetch_organisation(id)?;
        if organisation.status != OrganisationStatus::Live.as_str() {
            return Err(AppError::UnprocessableEntity(
                json!({ "error": "Only live organisations accept check-ins" }),
            ));
        }

        self.organisation_repo
            .checkin(id, user_id, still_active, tip)?;

        self.fetch_organisation(id)
    }

    /// Public visit counter: increments and returns the new total.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::features::organisation::presenters::OrganisationPresenterImpl;
    use crate::app::features::organisation::repositories::{
        FetchOrganisationsRepositoryInput, SearchOrganisationsRepositoryInput,
    };
    use crate::app::features::user::entities::UserRole;

    /// Minimal repository stub: serves one organisation and one caller role,
    /// which is all `ensure_can_manage` touches.
    struct StubRepo {
        organisation: Organisation,
        caller_role: UserRole,
    }

    impl OrganisationRepository for StubRepo {
        fn fetch_organisation(&self, _id: Uuid) -> Result<Organisation, AppError> {
            Ok(self.organisation.clone())
        }

        fn fetch_user_role(&self, _user_id: Uuid) -> Result<UserRole, AppError> {
            Ok(self.caller_role)
        }

        fn fetch_organisations(
            &self,
            _params: FetchOrganisationsRepositoryInput,
        ) -> Result<Vec<Organisation>, AppError> {
            unimplemented!()
        }

        fn search_organisations(
            &self,
            _params: SearchOrganisationsRepositoryInput,
        ) -> Result<Vec<(Organisation, Option<f64>)>, AppError> {
            unimplemented!()
        }

        fn create_organisation(
            &self,
            _params: CreateOrganisationRepositoryInput,
        ) -> Result<Organisation, AppError> {
            unimplemented!()
        }

        fn delete_organisation(&self, _id: Uuid) -> Result<(), AppError> {
            unimplemented!()
        }

        fn update_organisation(
            &self,
            _id: Uuid,
            _params: UpdateOrganisationRepositoryInput,
        ) -> Result<Organisation, AppError> {
            unimplemented!()
        }

        fn increment_visits(&self, _id: Uuid) -> Result<i64, AppError> {
            unimplemented!()
        }

        fn checkin(
            &self,
            _id: Uuid,
            _user_id: Uuid,
            _still_active: bool,
            _tip: Option<String>,
        ) -> Result<(), AppError> {
            unimplemented!()
        }

        fn fetch_community_signals(
            &self,
            _id: Uuid,
        ) -> Result<crate::app::features::organisation::entities::CommunitySignals, AppError>
        {
            unimplemented!()
        }

        fn has_duplicate_nearby(
            &self,
            _name: &str,
            _latitude: Option<&BigDecimal>,
            _longitude: Option<&BigDecimal>,
        ) -> Result<bool, AppError> {
            unimplemented!()
        }

        fn count_live_orgs_created_by(&self, _user_id: Uuid) -> Result<i64, AppError> {
            unimplemented!()
        }
    }

    fn test_organisation(created_by: Option<Uuid>) -> Organisation {
        Organisation {
            id: Uuid::new_v4(),
            name: "Org".to_string(),
            tel: None,
            email: None,
            address: None,
            description: None,
            location_country_id: None,
            organisation_type_id: None,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            latitude: None,
            longitude: None,
            founder_country_id: None,
            created_by,
            verified: false,
            status: "live".to_string(),
            moderation_note: None,
            added_by: None,
            city: None,
            website: None,
            telegram: None,
            whatsapp: None,
            services: vec![],
            languages: vec![],
            opening_hours: None,
            timezone: None,
            cost: None,
            google_place_id: None,
            google_rating: None,
            visits_count: 0,
            rating_avg: None,
            reviews_count: 0,
        }
    }

    struct UnreachableModerationRepo;

    impl crate::app::features::moderation::repositories::ModerationRepository
        for UnreachableModerationRepo
    {
        fn record_submitted(
            &self,
            _input: crate::app::features::moderation::repositories::SubmittedEventInput,
        ) -> Result<(), AppError> {
            unreachable!()
        }
        fn fetch_queue(
            &self,
            _kind: Option<crate::app::features::moderation::repositories::TargetKind>,
        ) -> Result<Vec<crate::app::features::moderation::repositories::QueueItem>, AppError>
        {
            unreachable!()
        }
        fn moderate(
            &self,
            _kind: crate::app::features::moderation::repositories::TargetKind,
            _target_id: Uuid,
            _action: crate::app::features::moderation::repositories::ModerationAction,
            _note: Option<String>,
            _moderator_id: Uuid,
        ) -> Result<(), AppError> {
            unreachable!()
        }
        fn fetch_user_role(&self, _user_id: Uuid) -> Result<UserRole, AppError> {
            unreachable!()
        }
    }

    fn usecase_with(organisation: Organisation, caller_role: UserRole) -> OrganisationUsecase {
        OrganisationUsecase::new(
            Arc::new(StubRepo {
                organisation,
                caller_role,
            }),
            Arc::new(OrganisationPresenterImpl::new()),
            Arc::new(UnreachableModerationRepo),
        )
    }

    #[test]
    fn test_ensure_can_manage_allows_creator() {
        let owner = Uuid::new_v4();
        let org = test_organisation(Some(owner));
        let usecase = usecase_with(org.clone(), UserRole::User);

        assert!(usecase.ensure_can_manage(&org, owner).is_ok());
    }

    #[test]
    fn test_ensure_can_manage_rejects_stranger() {
        let org = test_organisation(Some(Uuid::new_v4()));
        let usecase = usecase_with(org.clone(), UserRole::User);

        match usecase.ensure_can_manage(&org, Uuid::new_v4()) {
            Err(AppError::Forbidden(_)) => (),
            other => panic!("expected Forbidden, got {:?}", other.err()),
        }
    }

    #[test]
    fn test_ensure_can_manage_allows_moderator_and_admin() {
        let org = test_organisation(Some(Uuid::new_v4()));

        for role in [UserRole::Moderator, UserRole::Admin] {
            let usecase = usecase_with(org.clone(), role);
            assert!(
                usecase.ensure_can_manage(&org, Uuid::new_v4()).is_ok(),
                "{:?} must be able to manage any organisation",
                role
            );
        }
    }

    #[test]
    fn test_ensure_can_manage_rejects_stranger_on_orphan_org() {
        // created_by is NULL for pre-v2 rows: only moderators may touch them.
        let org = test_organisation(None);
        let usecase = usecase_with(org.clone(), UserRole::User);

        assert!(usecase.ensure_can_manage(&org, Uuid::new_v4()).is_err());
    }
}
