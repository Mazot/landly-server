use super::{
    entities::{AddedBy, LocationSource, Organisation, OrganisationStatus},
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
use crate::utils::google_places::{PlaceLookup, validate_place_id};
use crate::utils::storage::StorageService;
use crate::utils::validation::{self as v, limits};
use actix_web::HttpResponse;
use bigdecimal::{BigDecimal, FromPrimitive, RoundingMode, ToPrimitive};
use chrono::NaiveDateTime;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct OrganisationUsecase {
    organisation_repo: Arc<dyn OrganisationRepository>,
    organisation_presenter: Arc<dyn OrganisationPresenter>,
    moderation_repo: Arc<dyn ModerationRepository>,
    /// Google Places lookups; None without `GOOGLE_PLACES_API_KEY`.
    place_lookup: Option<Arc<dyn PlaceLookup>>,
    /// Object storage, to remove an organisation's images when it is deleted.
    storage: Arc<dyn StorageService>,
}

/// The point a new submission is stored with, plus what Google said about it.
struct ResolvedLocation {
    latitude: Option<BigDecimal>,
    longitude: Option<BigDecimal>,
    source: Option<LocationSource>,
    refreshed_at: Option<NaiveDateTime>,
    /// Some(true/false) when a `google_place_id` was given: whether the
    /// server could confirm it with Google.
    google_verified: Option<bool>,
    google_business_status: Option<String>,
}

/// The user-supplied text of an organisation, borrowed from either the create
/// or the update input, checked against the size limits (audit M3).
struct OrgTextFields<'a> {
    name: Option<&'a str>,
    tel: Option<&'a str>,
    email: Option<&'a str>,
    address: Option<&'a str>,
    description: Option<&'a str>,
    city: Option<&'a str>,
    website: Option<&'a str>,
    telegram: Option<&'a str>,
    whatsapp: Option<&'a str>,
    timezone: Option<&'a str>,
    services: Option<&'a [String]>,
    languages: Option<&'a [String]>,
    opening_hours: Option<&'a serde_json::Value>,
}

impl OrgTextFields<'_> {
    fn validate(&self) -> Result<(), AppError> {
        v::max_chars("name", self.name, limits::ORG_NAME)?;
        v::max_chars("tel", self.tel, limits::PHONE)?;
        v::max_chars("email", self.email, limits::EMAIL)?;
        v::max_chars("address", self.address, limits::ADDRESS)?;
        v::max_chars("description", self.description, limits::DESCRIPTION)?;
        v::max_chars("city", self.city, limits::CITY)?;
        v::max_chars("website", self.website, limits::URL)?;
        v::max_chars("telegram", self.telegram, limits::HANDLE)?;
        v::max_chars("whatsapp", self.whatsapp, limits::HANDLE)?;
        v::max_chars("timezone", self.timezone, limits::TIMEZONE)?;
        v::tag_list(
            "services",
            self.services,
            limits::TAGS_PER_LIST,
            limits::TAG,
        )?;
        v::tag_list(
            "languages",
            self.languages,
            limits::TAGS_PER_LIST,
            limits::TAG,
        )?;
        v::json_object(
            "opening_hours",
            self.opening_hours,
            limits::JSON_DOCUMENT_BYTES,
        )
    }
}

/// Non-moderators may have at most this many organisations waiting for
/// moderation — a flood of submissions must not bury the queue.
const MAX_PENDING_PER_USER: i64 = 10;
/// ...and may submit at most this many per rolling 24 hours (every submission
/// with a Google place is a billed lookup).
const MAX_SUBMISSIONS_PER_DAY: i64 = 20;

impl OrganisationUsecase {
    pub fn new(
        organisation_repo: Arc<dyn OrganisationRepository>,
        organisation_presenter: Arc<dyn OrganisationPresenter>,
        moderation_repo: Arc<dyn ModerationRepository>,
        place_lookup: Option<Arc<dyn PlaceLookup>>,
        storage: Arc<dyn StorageService>,
    ) -> Self {
        Self {
            organisation_repo,
            organisation_presenter,
            moderation_repo,
            place_lookup,
            storage,
        }
    }

    /// `official` is a trust badge ("run by the official representative"), so
    /// only moderators/admins — who can verify that — may give it or take it
    /// away (audit M4). Everyone else may resend the stored value unchanged,
    /// which is what an edit form does.
    fn ensure_added_by_allowed(
        is_moderator: bool,
        stored: Option<&str>,
        requested: Option<&str>,
    ) -> Result<(), AppError> {
        let official = AddedBy::Official.as_str();
        let Some(requested) = requested else {
            return Ok(());
        };
        let grants = requested == official && stored != Some(official);
        let removes = stored == Some(official) && requested != official;

        if !is_moderator && (grants || removes) {
            return Err(AppError::Forbidden(json!({
                "error": "Only a moderator can mark an organisation as official or change that mark"
            })));
        }
        Ok(())
    }

    /// Per-user quotas for new submissions; moderators and admins are exempt.
    /// Runs first, before any validation that costs a query or a Google call.
    fn ensure_submission_quota(&self, user_id: Uuid, is_moderator: bool) -> Result<(), AppError> {
        if is_moderator {
            return Ok(());
        }

        let pending = self
            .organisation_repo
            .count_pending_orgs_created_by(user_id)?;
        if pending >= MAX_PENDING_PER_USER {
            return Err(AppError::TooManyRequests(json!({
                "error": format!(
                    "You already have {pending} organisations waiting for moderation; \
                     please wait until they are reviewed"
                )
            })));
        }

        let since = chrono::Utc::now().naive_utc() - chrono::Duration::hours(24);
        let recent = self
            .organisation_repo
            .count_orgs_created_since(user_id, since)?;
        if recent >= MAX_SUBMISSIONS_PER_DAY {
            return Err(AppError::TooManyRequests(json!({
                "error": format!(
                    "You can submit at most {MAX_SUBMISSIONS_PER_DAY} organisations per day; \
                     please try again tomorrow"
                )
            })));
        }

        Ok(())
    }

    /// A point is a pair: a lone latitude or longitude can never be shown on
    /// the map (search needs both), and the DB CHECK would reject it anyway.
    fn ensure_point_is_pair(
        latitude: &Option<BigDecimal>,
        longitude: &Option<BigDecimal>,
    ) -> Result<(), AppError> {
        if latitude.is_some() != longitude.is_some() {
            return Err(AppError::UnprocessableEntity(
                json!({ "error": "latitude and longitude must be given together" }),
            ));
        }
        Ok(())
    }

    /// Whether an update really moves the point. Edit forms usually re-send
    /// the coordinates they loaded; compared at the column precision
    /// (DECIMAL(_, 8), which Postgres rounds half away from zero) that is
    /// no move, so a Google point keeps its source and refresh time.
    fn point_moved(
        existing: &Organisation,
        latitude: &Option<BigDecimal>,
        longitude: &Option<BigDecimal>,
    ) -> bool {
        let (Some(latitude), Some(longitude)) = (latitude, longitude) else {
            return false;
        };
        let at_db_precision = |v: &BigDecimal| v.with_scale_round(8, RoundingMode::HalfUp);
        let stored = existing
            .latitude
            .as_ref()
            .zip(existing.longitude.as_ref())
            .map(|(lat, lng)| (at_db_precision(lat), at_db_precision(lng)));

        stored != Some((at_db_precision(latitude), at_db_precision(longitude)))
    }

    /// Decides the stored point of a submission:
    /// - with `google_place_id` and a server key: Google's coordinates (the
    ///   client's are ignored), source `google`, refreshed now;
    /// - with `google_place_id` but no key: the client's point (from its
    ///   Places Autocomplete), source `google`, unverified (`refreshed_at`
    ///   null) — a disabled integration, like Google OAuth without its keys;
    /// - without it: the client's pin, source `user_pin`.
    async fn resolve_new_location(
        &self,
        place_id: Option<&str>,
        latitude: Option<BigDecimal>,
        longitude: Option<BigDecimal>,
    ) -> Result<ResolvedLocation, AppError> {
        Self::ensure_point_is_pair(&latitude, &longitude)?;

        let Some(place_id) = place_id else {
            let source = latitude.is_some().then_some(LocationSource::UserPin);
            return Ok(ResolvedLocation {
                latitude,
                longitude,
                source,
                refreshed_at: None,
                google_verified: None,
                google_business_status: None,
            });
        };
        validate_place_id(place_id)?;

        // A place that already has an organisation is refused here, before
        // the (billed) Google call. Same answer as the unique index gives
        // when two submissions race past this check.
        if self.organisation_repo.exists_by_google_place_id(place_id)? {
            return Err(AppError::UnprocessableEntity(json!({
                "error": "A record with these values already exists",
                "constraint": "organisations_google_place_id_key",
            })));
        }

        let Some(lookup) = &self.place_lookup else {
            let source = latitude.is_some().then_some(LocationSource::Google);
            return Ok(ResolvedLocation {
                latitude,
                longitude,
                source,
                refreshed_at: None,
                google_verified: Some(false),
                google_business_status: None,
            });
        };

        let place = lookup.lookup(place_id).await?;
        let decimal = |v: f64| BigDecimal::from_f64(v).ok_or(AppError::InternalServerError);
        Ok(ResolvedLocation {
            latitude: Some(decimal(place.latitude)?),
            longitude: Some(decimal(place.longitude)?),
            source: Some(LocationSource::Google),
            refreshed_at: Some(chrono::Utc::now().naive_utc()),
            google_verified: Some(true),
            google_business_status: place.business_status,
        })
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
    pub async fn create_organisation(
        &self,
        params: CreateOrganisationUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        if let Some(added_by) = params.added_by.as_deref() {
            AddedBy::try_from(added_by)?;
        }
        OrgTextFields {
            name: Some(&params.name),
            tel: params.tel.as_deref(),
            email: params.email.as_deref(),
            address: params.address.as_deref(),
            description: params.description.as_deref(),
            city: params.city.as_deref(),
            website: params.website.as_deref(),
            telegram: params.telegram.as_deref(),
            whatsapp: params.whatsapp.as_deref(),
            timezone: params.timezone.as_deref(),
            services: Some(&params.services),
            languages: Some(&params.languages),
            opening_hours: params.opening_hours.as_ref(),
        }
        .validate()?;
        let is_moderator = self
            .organisation_repo
            .fetch_user_role(params.created_by)?
            .is_moderator();
        Self::ensure_added_by_allowed(is_moderator, None, params.added_by.as_deref())?;
        self.ensure_submission_quota(params.created_by, is_moderator)?;
        let location = self
            .resolve_new_location(
                params.google_place_id.as_deref(),
                params.latitude,
                params.longitude,
            )
            .await?;

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
                    latitude: location.latitude,
                    longitude: location.longitude,
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
                    google_place_id: params.google_place_id,
                    location_source: location.source.map(|s| s.as_str().to_string()),
                    location_refreshed_at: location.refreshed_at,
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
                "googlePlaceVerified": location.google_verified,
                "googleBusinessStatus": location.google_business_status,
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
        if let Some(added_by) = params.added_by.as_deref() {
            AddedBy::try_from(added_by)?;
        }
        OrgTextFields {
            name: params.name.as_deref(),
            tel: params.tel.as_deref(),
            email: params.email.as_deref(),
            address: params.address.as_deref(),
            description: params.description.as_deref(),
            city: params.city.as_deref(),
            website: params.website.as_deref(),
            telegram: params.telegram.as_deref(),
            whatsapp: params.whatsapp.as_deref(),
            timezone: params.timezone.as_deref(),
            services: params.services.as_deref(),
            languages: params.languages.as_deref(),
            opening_hours: params.opening_hours.as_ref(),
        }
        .validate()?;

        Self::ensure_point_is_pair(&params.latitude, &params.longitude)?;

        let existing = self.organisation_repo.fetch_organisation(id)?;
        let managed_by = self.ensure_can_manage(&existing, caller_user_id)?;
        Self::ensure_added_by_allowed(
            managed_by == ManagedBy::Moderator,
            existing.added_by.as_deref(),
            params.added_by.as_deref(),
        )?;

        // A point moved by hand is ours, not Google's: it is never refreshed
        // from Google again.
        let moved = Self::point_moved(&existing, &params.latitude, &params.longitude);
        let location_source = moved.then(|| match managed_by {
            ManagedBy::Moderator => LocationSource::Moderator.as_str().to_string(),
            ManagedBy::Creator => LocationSource::UserPin.as_str().to_string(),
        });

        // A creator's edit of an already approved organisation must not go
        // live unreviewed — otherwise approval of a harmless entry could be
        // turned into publishing anything. It returns to the queue instead.
        let back_to_moderation = managed_by == ManagedBy::Creator
            && existing.status == OrganisationStatus::Live.as_str();

        // An owner's edit never rewrites the badge it merely resent, and a
        // changed one is written only while the row still is not `official`
        // (checked in the UPDATE itself, so a moderator's concurrent change
        // cannot be overwritten).
        let is_moderator = managed_by == ManagedBy::Moderator;
        let added_by = params
            .added_by
            .filter(|requested| is_moderator || existing.added_by.as_deref() != Some(requested));
        let guard_official_badge = !is_moderator && added_by.is_some();

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
                added_by,
                guard_official_badge,
                status: back_to_moderation
                    .then(|| OrganisationStatus::Pending.as_str().to_string()),
                location_source,
                location_refreshed_at: moved.then_some(None),
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
        for (field, value) in [
            ("name", &params.name),
            ("tel", &params.tel),
            ("email", &params.email),
            ("address", &params.address),
        ] {
            v::max_chars(field, value.as_deref(), limits::FILTER)?;
        }
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
    /// Live places near a point with a name like the one being typed: the add-a-place form
    /// warns before a duplicate is submitted. Pending submissions of others stay private.
    pub fn find_duplicates(
        &self,
        params: FindDuplicatesUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        v::max_chars("name", Some(&params.name), limits::ORG_NAME)?;
        let similar = self.organisation_repo.find_similar_live_nearby(
            &params.name,
            params.latitude,
            params.longitude,
            DUPLICATE_HINTS,
        )?;
        let items = similar
            .into_iter()
            .map(|organisation| {
                let distance = organisation
                    .latitude
                    .as_ref()
                    .zip(organisation.longitude.as_ref())
                    .and_then(|(lat, lng)| Some((lat.to_f64()?, lng.to_f64()?)))
                    .map(|point| {
                        super::repositories::haversine_km(
                            (params.latitude, params.longitude),
                            point,
                        )
                    });
                (organisation, distance)
            })
            .collect();

        Ok(self.organisation_presenter.to_search_json(items))
    }

    pub fn search_organisations(
        &self,
        params: SearchOrganisationsUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        // `types` are organisation-type slugs (up to `SLUG` long), `languages`
        // are language names.
        for (field, list, max_len) in [
            ("types", &params.type_slugs, limits::SLUG),
            ("languages", &params.languages, limits::TAG),
        ] {
            v::tag_list(field, list.as_deref(), limits::FILTER_LIST_ITEMS, max_len)?;
        }
        if let Some(added_by) = params.added_by.as_deref() {
            AddedBy::try_from(added_by)?;
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
                    location_country_id: params.location_country_id,
                    founder_country_id: params.founder_country_id,
                    sort,
                    limit: params.limit,
                    offset: params.offset,
                })?;

        Ok(self.organisation_presenter.to_search_json(items))
    }

    /// Deletes the organisation and its images. The rows go first (they are
    /// the source of truth and cascade in the database); the stored objects
    /// are removed afterwards, best-effort — the bucket is publicly readable,
    /// so an object left behind stays reachable by its old URL until the
    /// `orphan_images` job removes it (audit M7).
    pub async fn delete_organisation(
        &self,
        id: Uuid,
        caller_user_id: Uuid,
    ) -> Result<HttpResponse, AppError> {
        let existing = self.organisation_repo.fetch_organisation(id)?;
        self.ensure_can_manage(&existing, caller_user_id)?;

        let image_keys = self.organisation_repo.list_image_keys(id)?;
        self.organisation_repo.delete_organisation(id)?;

        for key in image_keys {
            if let Err(e) = self.storage.delete(&key).await {
                log::error!(
                    "organisation {id} deleted but its object `{key}` could not be removed from \
                     storage ({e:?}); run orphan_images to clean it up"
                );
            }
        }
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
        v::max_chars("tip", tip.as_deref(), limits::CHECKIN_TIP)?;
        // A blank tip means "no tip": it must not replace a useful earlier one.
        let tip = tip.filter(|t| !t.trim().is_empty());
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
    /// Who added it: `official`, `community` or `volunteer`; only moderators/admins can set or change `official`; `None` keeps the current value.
    pub added_by: Option<String>,
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

/// How many similar places the duplicate hint lists at most.
const DUPLICATE_HINTS: i64 = 3;

pub struct FindDuplicatesUsecaseInput {
    /// The name being typed.
    pub name: String,
    /// Where the new place is (validated by the controller).
    pub latitude: f64,
    pub longitude: f64,
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
    /// Only places located in this country (the corridor's destination).
    pub location_country_id: Option<Uuid>,
    /// Only places founded by people from this country (the corridor's origin).
    pub founder_country_id: Option<Uuid>,
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

        fn find_similar_live_nearby(
            &self,
            _name: &str,
            _latitude: f64,
            _longitude: f64,
            _limit: i64,
        ) -> Result<Vec<Organisation>, AppError> {
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

        fn list_image_keys(&self, _organisation_id: Uuid) -> Result<Vec<String>, AppError> {
            unimplemented!()
        }

        fn count_pending_orgs_created_by(&self, _user_id: Uuid) -> Result<i64, AppError> {
            unimplemented!()
        }

        fn count_orgs_created_since(
            &self,
            _user_id: Uuid,
            _since: chrono::NaiveDateTime,
        ) -> Result<i64, AppError> {
            unimplemented!()
        }

        fn exists_by_google_place_id(&self, _place_id: &str) -> Result<bool, AppError> {
            unimplemented!()
        }

        fn count_live_orgs_created_by(&self, _user_id: Uuid) -> Result<i64, AppError> {
            unimplemented!()
        }
    }

    #[test]
    fn test_point_moved_compares_at_the_column_precision() {
        use bigdecimal::{BigDecimal, FromPrimitive};
        use std::str::FromStr;
        let dec = |v: &str| Some(BigDecimal::from_str(v).unwrap());
        let mut org = test_organisation(None);
        org.latitude = dec("52.51627500");
        org.longitude = dec("13.37770400");

        // What a client sends back after a JSON round-trip through f64.
        let float = |v: f64| Some(BigDecimal::from_f64(v).unwrap());
        assert!(!OrganisationUsecase::point_moved(
            &org,
            &float(52.516275),
            &float(13.377704)
        ));
        assert!(!OrganisationUsecase::point_moved(
            &org,
            &dec("52.516275004"),
            &dec("13.377704")
        ));
        assert!(OrganisationUsecase::point_moved(
            &org,
            &dec("52.51627501"),
            &dec("13.377704")
        ));
        assert!(
            OrganisationUsecase::point_moved(&org, &dec("52.516275005"), &dec("13.377704")),
            "half up, like Postgres"
        );
        assert!(
            !OrganisationUsecase::point_moved(&org, &None, &None),
            "no point sent"
        );

        org.latitude = None;
        org.longitude = None;
        assert!(
            OrganisationUsecase::point_moved(&org, &float(1.0), &float(2.0)),
            "first point counts as a move"
        );
    }

    /// Audit M4: `official` is granted and removed by moderators only;
    /// resending the stored value is what an edit form does and is fine.
    #[test]
    fn test_added_by_official_truth_table() {
        let allowed = OrganisationUsecase::ensure_added_by_allowed;
        // (is_moderator, stored, requested, ok)
        let cases = [
            (false, None, None, true),
            (false, None, Some("community"), true),
            (false, Some("community"), Some("volunteer"), true),
            (false, None, Some("official"), false),
            (false, Some("community"), Some("official"), false),
            (false, Some("official"), Some("official"), true),
            (false, Some("official"), Some("community"), false),
            (false, Some("official"), None, true),
            (true, None, Some("official"), true),
            (true, Some("official"), Some("community"), true),
        ];
        for (moderator, stored, requested, ok) in cases {
            assert_eq!(
                allowed(moderator, stored, requested).is_ok(),
                ok,
                "moderator={moderator} stored={stored:?} requested={requested:?}"
            );
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
            google_place_id: None,
            visits_count: 0,
            rating_avg: None,
            reviews_count: 0,
            location_source: None,
            location_refreshed_at: None,
        }
    }

    struct UnreachableModerationRepo;

    impl crate::app::features::moderation::repositories::ModerationRepository
        for UnreachableModerationRepo
    {
        fn fetch_submitter(
            &self,
            _kind: crate::app::features::moderation::entities::TargetKind,
            _target_id: Uuid,
        ) -> Result<Option<Uuid>, AppError> {
            unreachable!()
        }

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
            None,
            Arc::new(crate::utils::storage::NoOpStorageService),
        )
    }

    fn empty_update() -> UpdateOrganisationUsecaseInput {
        UpdateOrganisationUsecaseInput {
            added_by: None,
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
