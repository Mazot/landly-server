use super::{
    entities::{AddedBy, Cost, Organisation, OrganisationStatus},
    presenters::OrganisationPresenter,
    repositories::{
        CreateOrganisationRepositoryInput, FetchOrganisationsRepositoryInput,
        OrganisationRepository, SearchOrganisationsRepositoryInput, SearchSort,
        UpdateOrganisationRepositoryInput,
    },
};
use crate::app::features::moderation::entities::TargetKind;
use crate::app::features::moderation::repositories::{ModerationRepository, SubmittedEventInput};
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
    /// Returns which of the two granted access; the role wins, so a
    /// moderator editing their own organisation counts as a moderator.
    fn ensure_can_manage(
        &self,
        organisation: &Organisation,
        caller_user_id: Uuid,
    ) -> Result<ManagedBy, AppError> {
        let role = self.organisation_repo.fetch_user_role(caller_user_id)?;
        if role.is_moderator() {
            return Ok(ManagedBy::Moderator);
        }

        if organisation.created_by == Some(caller_user_id) {
            return Ok(ManagedBy::Creator);
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
                new_organisation.id,
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
        let managed_by = self.ensure_can_manage(&existing, caller_user_id)?;

        // A creator's edit of an already approved organisation must not go
        // live unreviewed — otherwise approval of a harmless entry could be
        // turned into publishing anything. It returns to the queue instead.
        let back_to_moderation = managed_by == ManagedBy::Creator
            && existing.status == OrganisationStatus::Live.as_str();

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
                status: back_to_moderation
                    .then(|| OrganisationStatus::Pending.as_str().to_string()),
            },
        )?;

        if back_to_moderation {
            let _ = self.moderation_repo.record_submitted(SubmittedEventInput {
                target_kind: TargetKind::Org,
                target_id: updated_organisation.id,
                flags: json!({ "editedAfterApproval": true }),
            });
        }

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

/// Who passed `ensure_can_manage`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ManagedBy {
    Creator,
    Moderator,
}

pub struct UpdateOrganisationUsecaseInput {
    /// Display name; `None` keeps the current value.
    pub name: Option<String>,
    /// Contact phone, free text (format is only sanity-checked into moderation flags); `None` keeps the current value.
    pub tel: Option<String>,
    /// Contact email; `None` keeps the current value.
    pub email: Option<String>,
    /// Street address, free text; `None` keeps the current value.
    pub address: Option<String>,
    /// Free-text description; `None` keeps the current value.
    pub description: Option<String>,
    /// Country the organisation is located in; `None` keeps the current value.
    pub location_country_id: Option<Uuid>,
    /// Organisation type (embassy, business, helper, community, volunteer, …); `None` keeps the current value.
    pub organisation_type_id: Option<Uuid>,
    /// Latitude in degrees (-90..90); `None` keeps the current value.
    pub latitude: Option<BigDecimal>,
    /// Longitude in degrees (-180..180); `None` keeps the current value.
    pub longitude: Option<BigDecimal>,
    /// Home country of the founders / community it serves (the corridor's "from" country); `None` keeps the current value.
    pub founder_country_id: Option<Uuid>,
    /// City name; `None` keeps the current value.
    pub city: Option<String>,
    /// Website URL; `None` keeps the current value.
    pub website: Option<String>,
    /// Telegram handle or link; `None` keeps the current value.
    pub telegram: Option<String>,
    /// WhatsApp number or link; `None` keeps the current value.
    pub whatsapp: Option<String>,
    /// Services offered, free-text tags; `None` keeps the current value.
    pub services: Option<Vec<String>>,
    /// Languages spoken, by name (e.g. "Russian"); matched by the search `languages` filter; `None` keeps the current value.
    pub languages: Option<Vec<String>>,
    /// Weekly hours as JSON: `{"mon": [["09:00","13:00"], ["14:00","18:00"]], …}` (keys mon..sun); `None` keeps the current value.
    pub opening_hours: Option<serde_json::Value>,
    /// IANA timezone of the place, e.g. "Europe/Berlin"; used with `opening_hours` for `openNow`; `None` keeps the current value.
    pub timezone: Option<String>,
    /// `free` or `paid`; `None` keeps the current value.
    pub cost: Option<String>,
}

pub struct CreateOrganisationUsecaseInput {
    /// Display name.
    pub name: String,
    /// Contact phone, free text (format is only sanity-checked into moderation flags).
    pub tel: Option<String>,
    /// Contact email.
    pub email: Option<String>,
    /// Street address, free text.
    pub address: Option<String>,
    /// Free-text description.
    pub description: Option<String>,
    /// Country the organisation is located in.
    pub location_country_id: Option<Uuid>,
    /// Organisation type (embassy, business, helper, community, volunteer, …).
    pub organisation_type_id: Option<Uuid>,
    /// Latitude in degrees (-90..90).
    pub latitude: Option<BigDecimal>,
    /// Longitude in degrees (-180..180).
    pub longitude: Option<BigDecimal>,
    /// Home country of the founders / community it serves (the corridor's "from" country).
    pub founder_country_id: Option<Uuid>,
    /// Submitting user (from the JWT).
    pub created_by: Uuid,
    /// Who added it: `official`, `community` or `volunteer`.
    pub added_by: Option<String>,
    /// City name.
    pub city: Option<String>,
    /// Website URL.
    pub website: Option<String>,
    /// Telegram handle or link.
    pub telegram: Option<String>,
    /// WhatsApp number or link.
    pub whatsapp: Option<String>,
    /// Services offered, free-text tags.
    pub services: Vec<String>,
    /// Languages spoken, by name (e.g. "Russian"); matched by the search `languages` filter.
    pub languages: Vec<String>,
    /// Weekly hours as JSON: `{"mon": [["09:00","13:00"], ["14:00","18:00"]], …}` (keys mon..sun).
    pub opening_hours: Option<serde_json::Value>,
    /// IANA timezone of the place, e.g. "Europe/Berlin"; used with `opening_hours` for `openNow`.
    pub timezone: Option<String>,
    /// `free` or `paid`.
    pub cost: Option<String>,
    /// Google Places ID when imported from Google.
    pub google_place_id: Option<String>,
}

pub struct FetchOrganisationsUsecaseInput {
    /// Case-insensitive substring filter on the name.
    pub name: Option<String>,
    /// Case-insensitive substring filter on the phone.
    pub tel: Option<String>,
    /// Case-insensitive substring filter on the email.
    pub email: Option<String>,
    /// Case-insensitive substring filter on the address.
    pub address: Option<String>,
    /// Only organisations located in this country.
    pub location_country_id: Option<Uuid>,
    /// Only this organisation type.
    pub organisation_type_id: Option<Uuid>,
    /// Only organisations founded by people from this country.
    pub founder_country_id: Option<Uuid>,
    /// Page size.
    pub limit: i64,
    /// Rows to skip.
    pub offset: i64,
}

pub struct SearchOrganisationsUsecaseInput {
    /// (min_lat, min_lng, max_lat, max_lng), validated by the controller.
    pub bbox: Option<(f64, f64, f64, f64)>,
    /// (lat, lng) search origin, validated by the controller.
    pub origin: Option<(f64, f64)>,
    /// Search radius around the origin in km; narrows the bbox derived from it and filters by Haversine distance.
    pub radius_km: Option<f64>,
    /// Only these organisation type slugs.
    pub type_slugs: Option<Vec<String>>,
    /// Only places open right now (computed from hours + timezone).
    pub open_now: bool,
    /// Places speaking at least one of these languages.
    pub languages: Option<Vec<String>>,
    /// Only verified places.
    pub verified_only: bool,
    /// Minimum `rating_avg`.
    pub min_rating: Option<f64>,
    /// Only `official`, `community` or `volunteer` entries.
    pub added_by: Option<String>,
    /// Only `free` or only `paid` places.
    pub cost: Option<String>,
    /// `nearest` (default with an origin), `recent` (default without), or `verified`.
    pub sort: Option<String>,
    /// Page size (already clamped by the controller).
    pub limit: i64,
    /// Rows to skip.
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
        updated_status: std::sync::Mutex<Vec<Option<String>>>,
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
            params: UpdateOrganisationRepositoryInput,
        ) -> Result<Organisation, AppError> {
            self.updated_status
                .lock()
                .unwrap()
                .push(params.status.clone());

            let mut organisation = self.organisation.clone();
            if let Some(status) = params.status {
                organisation.status = status;
            }

            Ok(organisation)
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
            _exclude_id: Uuid,
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
            Ok(())
        }
        fn fetch_queue(
            &self,
            _kind: Option<crate::app::features::moderation::entities::TargetKind>,
        ) -> Result<Vec<crate::app::features::moderation::entities::QueueItem>, AppError> {
            unreachable!()
        }
        fn moderate(
            &self,
            _kind: crate::app::features::moderation::entities::TargetKind,
            _target_id: Uuid,
            _action: crate::app::features::moderation::entities::ModerationAction,
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
        usecase_with_repo(stub_repo(organisation, caller_role))
    }

    fn usecase_with_repo(repo: Arc<StubRepo>) -> OrganisationUsecase {
        OrganisationUsecase::new(
            repo,
            Arc::new(OrganisationPresenterImpl::new()),
            Arc::new(UnreachableModerationRepo),
        )
    }

    fn empty_update() -> UpdateOrganisationUsecaseInput {
        UpdateOrganisationUsecaseInput {
            name: Some("Renamed".to_string()),
            tel: None,
            email: None,
            address: None,
            description: None,
            location_country_id: None,
            organisation_type_id: None,
            latitude: None,
            longitude: None,
            founder_country_id: None,
            city: None,
            website: None,
            telegram: None,
            whatsapp: None,
            services: None,
            languages: None,
            opening_hours: None,
            timezone: None,
            cost: None,
        }
    }

    fn stub_repo(organisation: Organisation, caller_role: UserRole) -> Arc<StubRepo> {
        Arc::new(StubRepo {
            organisation,
            caller_role,
            updated_status: std::sync::Mutex::new(vec![]),
        })
    }

    /// Regression: after approval the creator could rewrite a live
    /// organisation and the change was published without moderation.
    #[test]
    fn test_creator_edit_of_live_org_returns_to_moderation() {
        let owner = Uuid::new_v4();
        let repo = stub_repo(test_organisation(Some(owner)), UserRole::User);

        usecase_with_repo(repo.clone())
            .update_organisation(Uuid::new_v4(), owner, empty_update())
            .unwrap();

        assert_eq!(
            *repo.updated_status.lock().unwrap(),
            vec![Some(OrganisationStatus::Pending.as_str().to_string())]
        );
    }

    /// Review regression: a moderator who created the organisation was
    /// classified as a plain creator and their edit sent it back to pending.
    #[test]
    fn test_moderator_editing_own_org_keeps_it_live() {
        let moderator = Uuid::new_v4();
        let repo = stub_repo(test_organisation(Some(moderator)), UserRole::Moderator);

        usecase_with_repo(repo.clone())
            .update_organisation(Uuid::new_v4(), moderator, empty_update())
            .unwrap();

        assert_eq!(*repo.updated_status.lock().unwrap(), vec![None]);
    }

    #[test]
    fn test_moderator_edit_keeps_org_live() {
        let repo = stub_repo(test_organisation(Some(Uuid::new_v4())), UserRole::Moderator);

        usecase_with_repo(repo.clone())
            .update_organisation(Uuid::new_v4(), Uuid::new_v4(), empty_update())
            .unwrap();

        assert_eq!(*repo.updated_status.lock().unwrap(), vec![None]);
    }

    #[test]
    fn test_creator_edit_of_pending_org_keeps_status() {
        let owner = Uuid::new_v4();
        let mut org = test_organisation(Some(owner));
        org.status = OrganisationStatus::Pending.as_str().to_string();
        let repo = stub_repo(org, UserRole::User);

        usecase_with_repo(repo.clone())
            .update_organisation(Uuid::new_v4(), owner, empty_update())
            .unwrap();

        assert_eq!(*repo.updated_status.lock().unwrap(), vec![None]);
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
