use crate::data::{models::OrganisationType, schema::organisations};
use crate::error::*;
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Moderation status stored in `organisations.status` (TEXT + CHECK constraint).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrganisationStatus {
    Pending,
    Live,
    Rejected,
}

impl OrganisationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrganisationStatus::Pending => "pending",
            OrganisationStatus::Live => "live",
            OrganisationStatus::Rejected => "rejected",
        }
    }
}

impl TryFrom<&str> for OrganisationStatus {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(OrganisationStatus::Pending),
            "live" => Ok(OrganisationStatus::Live),
            "rejected" => Ok(OrganisationStatus::Rejected),
            other => Err(AppError::UnprocessableEntity(
                json!({ "error": format!("Unknown organisation status: {}", other) }),
            )),
        }
    }
}

/// Provenance of the listing (`organisations.added_by`, TEXT + CHECK constraint).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddedBy {
    Official,
    Community,
    Volunteer,
}

impl AddedBy {
    pub fn as_str(&self) -> &'static str {
        match self {
            AddedBy::Official => "official",
            AddedBy::Community => "community",
            AddedBy::Volunteer => "volunteer",
        }
    }
}

impl TryFrom<&str> for AddedBy {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "official" => Ok(AddedBy::Official),
            "community" => Ok(AddedBy::Community),
            "volunteer" => Ok(AddedBy::Volunteer),
            other => Err(AppError::UnprocessableEntity(
                json!({ "error": format!("Unknown added_by value: {}", other) }),
            )),
        }
    }
}

/// Whether the service costs money (`organisations.cost`, TEXT + CHECK constraint).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cost {
    Free,
    Paid,
}

impl Cost {
    pub fn as_str(&self) -> &'static str {
        match self {
            Cost::Free => "free",
            Cost::Paid => "paid",
        }
    }
}

impl TryFrom<&str> for Cost {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "free" => Ok(Cost::Free),
            "paid" => Ok(Cost::Paid),
            other => Err(AppError::UnprocessableEntity(
                json!({ "error": format!("Unknown cost value: {}", other) }),
            )),
        }
    }
}

/// Where an organisation's point came from (`organisations.location_source`,
/// TEXT + CHECK constraint). Only `Google` points are Google content: they
/// are re-fetched before Google's caching window ends. The others are ours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationSource {
    /// Coordinates of `google_place_id`, looked up by the server.
    Google,
    /// Placed by the submitter on the map.
    UserPin,
    /// Moved by a moderator.
    Moderator,
}

impl LocationSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            LocationSource::Google => "google",
            LocationSource::UserPin => "user_pin",
            LocationSource::Moderator => "moderator",
        }
    }
}

impl TryFrom<&str> for LocationSource {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "google" => Ok(LocationSource::Google),
            "user_pin" => Ok(LocationSource::UserPin),
            "moderator" => Ok(LocationSource::Moderator),
            other => Err(AppError::UnprocessableEntity(
                json!({ "error": format!("Unknown location source: {}", other) }),
            )),
        }
    }
}

#[derive(Debug, Associations, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
// #[diesel(belongs_to(Country, foreign_key = location_country_id))]
// #[diesel(belongs_to(Country, foreign_key = founder_country_id))]
#[diesel(belongs_to(OrganisationType, foreign_key = organisation_type_id))]
#[diesel(table_name = organisations)]
pub struct Organisation {
    /// Organisation ID.
    pub id: Uuid,
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
    /// Submission time (UTC, no timezone).
    pub created_at: NaiveDateTime,
    /// Last edit (UTC, no timezone).
    pub updated_at: NaiveDateTime,
    /// Latitude in degrees (-90..90).
    pub latitude: Option<BigDecimal>,
    /// Longitude in degrees (-180..180).
    pub longitude: Option<BigDecimal>,
    /// Home country of the founders / community it serves (the corridor's "from" country).
    pub founder_country_id: Option<Uuid>,
    /// User who submitted it; null for seeded/legacy rows or when that user was deleted.
    pub created_by: Option<Uuid>,
    /// Verified badge. No API sets it — only changed directly in the database.
    pub verified: bool,
    /// Moderation status: `pending`, `live` or `rejected`. Only `live` rows appear in list/search.
    pub status: String,
    /// Moderator note from request-changes / reject.
    pub moderation_note: Option<String>,
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
    pub services: Vec<Option<String>>,
    /// Languages spoken, by name (e.g. "Russian"); matched by the search `languages` filter.
    pub languages: Vec<Option<String>>,
    /// Weekly hours as JSON: `{"mon": [["09:00","13:00"], ["14:00","18:00"]], …}` (keys mon..sun).
    pub opening_hours: Option<serde_json::Value>,
    /// IANA timezone of the place, e.g. "Europe/Berlin"; used with `opening_hours` for `openNow`.
    pub timezone: Option<String>,
    /// `free` or `paid`.
    pub cost: Option<String>,
    /// Google Places ID; the only Google content stored indefinitely (unique).
    pub google_place_id: Option<String>,
    /// Detail-page visit counter (POST /organisation/visit/{id}).
    pub visits_count: i64,
    /// Average review rating (1–5); null without reviews. Recomputed from reviews on every create/delete.
    pub rating_avg: Option<f64>,
    /// Number of reviews.
    pub reviews_count: i64,
    /// Where the point came from: `google`, `user_pin` or `moderator` (see `LocationSource`); null without a point.
    pub location_source: Option<String>,
    /// When a `google` point was last fetched from Google; null if never verified (no server key) or not a Google point.
    pub location_refreshed_at: Option<NaiveDateTime>,
}

impl Organisation {
    pub fn create(conn: &mut PgConnection, record: &CreateOrganisation) -> Result<Self, AppError> {
        let result = diesel::insert_into(organisations::table)
            .values(record)
            .get_result::<Organisation>(conn)?;

        Ok(result)
    }

    pub fn update(
        conn: &mut PgConnection,
        organisation_id: Uuid,
        record: &UpdateOrganisation,
    ) -> Result<Self, AppError> {
        let o = organisations::table.find(organisation_id);
        let result = diesel::update(o)
            .set(record)
            .get_result::<Organisation>(conn)?;

        Ok(result)
    }

    pub fn delete(conn: &mut PgConnection, organisation_id: Uuid) -> Result<(), AppError> {
        let o = organisations::table.find(organisation_id);
        diesel::delete(o).execute(conn)?;

        Ok(())
    }

    pub fn fetch_by_id(conn: &mut PgConnection, organisation_id: Uuid) -> Result<Self, AppError> {
        let result = organisations::table
            .find(organisation_id)
            .get_result::<Organisation>(conn)?;

        Ok(result)
    }

    /// Records a community check-in ("I was here, still active" + tip).
    pub fn checkin(
        conn: &mut PgConnection,
        organisation_id: Uuid,
        user_id: Uuid,
        still_active: bool,
        tip: Option<String>,
    ) -> Result<(), AppError> {
        use crate::data::schema::org_checkins;

        diesel::insert_into(org_checkins::table)
            .values((
                org_checkins::organisation_id.eq(organisation_id),
                org_checkins::user_id.eq(user_id),
                org_checkins::still_active.eq(still_active),
                org_checkins::tip.eq(tip),
            ))
            .execute(conn)?;

        Ok(())
    }

    /// Community-signals block for the detail page: how many people came,
    /// what share confirmed "still active", when the last check-in was, and
    /// the most recent tips.
    pub fn community_signals(
        conn: &mut PgConnection,
        organisation_id: Uuid,
    ) -> Result<CommunitySignals, AppError> {
        use crate::data::schema::org_checkins;

        let total = org_checkins::table
            .filter(org_checkins::organisation_id.eq(organisation_id))
            .count()
            .get_result::<i64>(conn)?;

        let active = org_checkins::table
            .filter(org_checkins::organisation_id.eq(organisation_id))
            .filter(org_checkins::still_active.eq(true))
            .count()
            .get_result::<i64>(conn)?;

        let last_checkin_at = org_checkins::table
            .filter(org_checkins::organisation_id.eq(organisation_id))
            .select(diesel::dsl::max(org_checkins::created_at))
            .first::<Option<NaiveDateTime>>(conn)?;

        let tips = org_checkins::table
            .filter(org_checkins::organisation_id.eq(organisation_id))
            .filter(org_checkins::tip.is_not_null())
            .order(org_checkins::created_at.desc())
            .select(org_checkins::tip)
            .limit(5)
            .load::<Option<String>>(conn)?
            .into_iter()
            .flatten()
            .collect();

        Ok(CommunitySignals {
            people_came: total,
            still_active_pct: if total > 0 {
                Some((active as f64 / total as f64 * 100.0).round())
            } else {
                None
            },
            last_checkin_at,
            tips,
        })
    }

    /// Submit-time auto-check: does a live org with a very similar name exist
    /// within ~1 km of these coordinates?
    /// Another organisation (not `exclude_id` — the check runs right after
    /// the new row is inserted) with the same name within ~1 km.
    pub fn has_duplicate_nearby(
        conn: &mut PgConnection,
        exclude_id: Uuid,
        name: &str,
        latitude: Option<&BigDecimal>,
        longitude: Option<&BigDecimal>,
    ) -> Result<bool, AppError> {
        let (Some(lat), Some(lng)) = (latitude, longitude) else {
            return Ok(false);
        };

        // ~1 km box around the point (1 degree latitude ≈ 111 km).
        let delta = BigDecimal::try_from(0.01).map_err(|_| AppError::InternalServerError)?;

        let count = organisations::table
            .filter(organisations::id.ne(exclude_id))
            .filter(organisations::name.ilike(name))
            .filter(organisations::latitude.between(lat - &delta, lat + &delta))
            .filter(organisations::longitude.between(lng - &delta, lng + &delta))
            .count()
            .get_result::<i64>(conn)?;

        Ok(count > 0)
    }

    pub fn increment_visits(
        conn: &mut PgConnection,
        organisation_id: Uuid,
    ) -> Result<i64, AppError> {
        let visits = diesel::update(organisations::table.find(organisation_id))
            .set(organisations::visits_count.eq(organisations::visits_count + 1))
            .returning(organisations::visits_count)
            .get_result::<i64>(conn)?;

        Ok(visits)
    }

    pub fn fetch_by_location_country(
        conn: &mut PgConnection,
        country_id: Uuid,
    ) -> Result<Vec<Self>, AppError> {
        let result = organisations::table
            .filter(organisations::location_country_id.eq(country_id))
            .load::<Organisation>(conn)?;

        Ok(result)
    }

    pub fn fetch_ids_by_organisation_type(
        conn: &mut PgConnection,
        organisation_type_id: Uuid,
    ) -> Result<Vec<Uuid>, AppError> {
        let result = organisations::table
            .filter(organisations::organisation_type_id.eq(organisation_type_id))
            .select(organisations::id)
            .load::<Uuid>(conn)?;

        Ok(result)
    }

    pub fn fetch_ids_by_location_country(
        conn: &mut PgConnection,
        location_country_id: Uuid,
    ) -> Result<Vec<Uuid>, AppError> {
        let result = organisations::table
            .filter(organisations::location_country_id.eq(location_country_id))
            .select(organisations::id)
            .load::<Uuid>(conn)?;

        Ok(result)
    }

    pub fn fetch_ids_by_founder_country(
        conn: &mut PgConnection,
        founder_country_id: Uuid,
    ) -> Result<Vec<Uuid>, AppError> {
        let result = organisations::table
            .filter(organisations::founder_country_id.eq(founder_country_id))
            .select(organisations::id)
            .load::<Uuid>(conn)?;

        Ok(result)
    }
}

/// Aggregated community check-in signals (never stored, always computed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunitySignals {
    /// Number of community check-ins.
    pub people_came: i64,
    /// Share of check-ins confirming the place is still active, 0–100; None without check-ins.
    pub still_active_pct: Option<f64>,
    /// Time of the latest check-in (UTC).
    pub last_checkin_at: Option<NaiveDateTime>,
    /// Up to 5 latest check-in tips, newest first.
    pub tips: Vec<String>,
}

#[derive(Insertable, Clone)]
#[diesel(table_name = organisations)]
pub struct CreateOrganisation {
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
    /// User who submitted it; null for seeded/legacy rows or when that user was deleted.
    pub created_by: Option<Uuid>,
    /// Always `pending` for API submissions.
    pub status: String,
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
    pub services: Vec<Option<String>>,
    /// Languages spoken, by name (e.g. "Russian"); matched by the search `languages` filter.
    pub languages: Vec<Option<String>>,
    /// Weekly hours as JSON: `{"mon": [["09:00","13:00"], ["14:00","18:00"]], …}` (keys mon..sun).
    pub opening_hours: Option<serde_json::Value>,
    /// IANA timezone of the place, e.g. "Europe/Berlin"; used with `opening_hours` for `openNow`.
    pub timezone: Option<String>,
    /// `free` or `paid`.
    pub cost: Option<String>,
    /// Google Places ID (unique); the only Google content stored indefinitely.
    pub google_place_id: Option<String>,
    /// Where the point came from (`LocationSource::as_str`); null without a point.
    pub location_source: Option<String>,
    /// When a `google` point was fetched from Google; null if not verified.
    pub location_refreshed_at: Option<NaiveDateTime>,
}

#[derive(AsChangeset, Clone)]
#[diesel(table_name = organisations)]
pub struct UpdateOrganisation {
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
    /// Set to now on every update.
    pub updated_at: NaiveDateTime,
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
    pub services: Option<Vec<Option<String>>>,
    /// Languages spoken, by name (e.g. "Russian"); matched by the search `languages` filter; `None` keeps the current value.
    pub languages: Option<Vec<Option<String>>>,
    /// Weekly hours as JSON: `{"mon": [["09:00","13:00"], ["14:00","18:00"]], …}` (keys mon..sun); `None` keeps the current value.
    pub opening_hours: Option<serde_json::Value>,
    /// IANA timezone of the place, e.g. "Europe/Berlin"; used with `opening_hours` for `openNow`; `None` keeps the current value.
    pub timezone: Option<String>,
    /// `free` or `paid`; `None` keeps the current value.
    pub cost: Option<String>,
    /// Only set when an edit has to go back through moderation.
    pub status: Option<String>,
    /// Set together with a moved point: `user_pin` (creator) or `moderator`; `None` keeps the current value.
    pub location_source: Option<String>,
    /// `Some(None)` clears it when a point is moved by hand (it is no longer Google's); `None` keeps it.
    pub location_refreshed_at: Option<Option<NaiveDateTime>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_organisation_status_roundtrip() {
        for status in [
            OrganisationStatus::Pending,
            OrganisationStatus::Live,
            OrganisationStatus::Rejected,
        ] {
            assert_eq!(
                OrganisationStatus::try_from(status.as_str()).unwrap(),
                status
            );
        }
        assert!(OrganisationStatus::try_from("draft").is_err());
    }

    #[test]
    fn test_added_by_roundtrip() {
        for added_by in [AddedBy::Official, AddedBy::Community, AddedBy::Volunteer] {
            assert_eq!(AddedBy::try_from(added_by.as_str()).unwrap(), added_by);
        }
        assert!(AddedBy::try_from("bot").is_err());
    }

    #[test]
    fn test_cost_roundtrip() {
        assert_eq!(Cost::try_from("free").unwrap(), Cost::Free);
        assert_eq!(Cost::try_from("paid").unwrap(), Cost::Paid);
        assert!(Cost::try_from("donation").is_err());
    }

    #[test]
    fn test_enum_errors_are_unprocessable_entity() {
        match OrganisationStatus::try_from("nope") {
            Err(AppError::UnprocessableEntity(_)) => (),
            other => panic!("expected UnprocessableEntity, got {:?}", other.err()),
        }
    }
}
