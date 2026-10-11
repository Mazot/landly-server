use crate::data::schema::{people, people_to_languages, person_claim_tokens, person_vouches};
use crate::error::*;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Person status flow (TEXT + CHECK): pending (in moderation) → awaiting
/// (approved, claim link out) → confirmed / claimed, or declined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonStatus {
    Pending,
    Awaiting,
    Confirmed,
    Claimed,
    Declined,
}

impl PersonStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PersonStatus::Pending => "pending",
            PersonStatus::Awaiting => "awaiting",
            PersonStatus::Confirmed => "confirmed",
            PersonStatus::Claimed => "claimed",
            PersonStatus::Declined => "declined",
        }
    }

    /// Publicly visible in lists / with contacts unlockable.
    pub fn is_public(&self) -> bool {
        matches!(self, PersonStatus::Confirmed | PersonStatus::Claimed)
    }
}

impl TryFrom<&str> for PersonStatus {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(PersonStatus::Pending),
            "awaiting" => Ok(PersonStatus::Awaiting),
            "confirmed" => Ok(PersonStatus::Confirmed),
            "claimed" => Ok(PersonStatus::Claimed),
            "declined" => Ok(PersonStatus::Declined),
            other => Err(AppError::UnprocessableEntity(
                json!({ "error": format!("Unknown person status: {}", other) }),
            )),
        }
    }
}

/// How the recommender sends the claim link (TEXT + CHECK).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendVia {
    Email,
    Whatsapp,
}

impl SendVia {
    pub fn as_str(&self) -> &'static str {
        match self {
            SendVia::Email => "email",
            SendVia::Whatsapp => "whatsapp",
        }
    }
}

impl TryFrom<&str> for SendVia {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "email" => Ok(SendVia::Email),
            "whatsapp" => Ok(SendVia::Whatsapp),
            other => Err(AppError::UnprocessableEntity(
                json!({ "error": format!("Unknown send_via value: {}", other) }),
            )),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = people)]
pub struct Person {
    /// Person ID.
    pub id: Uuid,
    /// Display name, e.g. "Daria K.".
    pub name: String,
    /// Short free-text bio.
    pub bio: Option<String>,
    /// City; shown publicly only when `show_city` is on.
    pub city: Option<String>,
    /// Country the person lives in.
    pub location_country_id: Option<Uuid>,
    /// What they help with, free-text tags, e.g. ["Relocation", "Housing"].
    pub skills: Vec<Option<String>>,
    /// Hidden contact. Used to deliver the claim link; public only after confirm/claim with `show_email` on.
    pub email: Option<String>,
    /// Hidden contact. Used to deliver the claim link; public only after confirm/claim with `show_whatsapp` on.
    pub whatsapp: Option<String>,
    /// How the recommender sends the claim link: `email` or `whatsapp`.
    pub send_via: Option<String>,
    /// The person agreed to be recommended (required to create).
    pub consent_given: bool,
    /// `pending` (moderation) → `awaiting` (approved, waiting for the person) → `confirmed` / `claimed` (linked to an account), or `declined`. Only confirmed/claimed are public.
    pub status: String,
    /// Privacy toggle: publish WhatsApp once confirmed. Default false.
    pub show_whatsapp: bool,
    /// Privacy toggle: publish email once confirmed. Default false.
    pub show_email: bool,
    /// Privacy toggle: publish the city. Default true.
    pub show_city: bool,
    /// Whether reviews of this person are accepted. Default true.
    pub allow_reviews: bool,
    /// User who recommended the person; null if that account was deleted.
    pub recommended_by: Option<Uuid>,
    /// Account the person linked when claiming (status `claimed`).
    pub claimed_by: Option<Uuid>,
    /// Moderator note from request-changes / reject.
    pub moderation_note: Option<String>,
    /// Average review rating (1–5); null without reviews. Recomputed on every review create/delete.
    pub rating_avg: Option<f64>,
    /// Number of reviews.
    pub reviews_count: i64,
    /// Recommendation time (UTC, no timezone).
    pub created_at: NaiveDateTime,
    /// Last change (UTC, no timezone).
    pub updated_at: NaiveDateTime,
}

impl Person {
    pub fn status_enum(&self) -> PersonStatus {
        PersonStatus::try_from(self.status.as_str()).unwrap_or(PersonStatus::Pending)
    }

    /// Guards the public claim endpoints. The recommender receives the claim
    /// token at creation time, so the token alone does not prove the caller
    /// is the recommended person:
    /// - confirming is only possible after moderation approved the entry
    ///   (`awaiting`), never straight from `pending`;
    /// - the recommender can never claim the profile as their own account.
    ///
    /// Declining stays possible from `pending` too — a person can always
    /// refuse to be listed.
    pub fn ensure_claimable(&self, outcome: &ClaimOutcome) -> Result<(), AppError> {
        let status = self.status_enum();

        match outcome {
            ClaimOutcome::Decline => {
                if !matches!(status, PersonStatus::Pending | PersonStatus::Awaiting) {
                    return Err(AppError::UnprocessableEntity(json!({
                        "error": format!("Person is already {}", self.status)
                    })));
                }
            }
            ClaimOutcome::Confirm { claimed_by, .. } => {
                if status == PersonStatus::Pending {
                    return Err(AppError::UnprocessableEntity(json!({
                        "error": "This recommendation is still being reviewed by a moderator"
                    })));
                }
                if status != PersonStatus::Awaiting {
                    return Err(AppError::UnprocessableEntity(json!({
                        "error": format!("Person is already {}", self.status)
                    })));
                }
                if claimed_by.is_some() && *claimed_by == self.recommended_by {
                    return Err(AppError::Forbidden(json!({
                        "error": "The recommender cannot claim this profile"
                    })));
                }
            }
        }

        Ok(())
    }
}

#[derive(Insertable, Clone)]
#[diesel(table_name = people)]
pub struct CreatePerson {
    /// Display name, e.g. "Daria K.".
    pub name: String,
    /// Short free-text bio.
    pub bio: Option<String>,
    /// City; shown publicly only when `show_city` is on.
    pub city: Option<String>,
    /// Country the person lives in.
    pub location_country_id: Option<Uuid>,
    /// What they help with, free-text tags, e.g. ["Relocation", "Housing"].
    pub skills: Vec<Option<String>>,
    /// Hidden contact. Used to deliver the claim link; public only after confirm/claim with `show_email` on.
    pub email: Option<String>,
    /// Hidden contact. Used to deliver the claim link; public only after confirm/claim with `show_whatsapp` on.
    pub whatsapp: Option<String>,
    /// How the recommender sends the claim link: `email` or `whatsapp`.
    pub send_via: Option<String>,
    /// The person agreed to be recommended (required to create).
    pub consent_given: bool,
    /// Always `pending` for new recommendations.
    pub status: String,
    /// Privacy toggle: publish WhatsApp once confirmed. Default false.
    pub show_whatsapp: bool,
    /// Privacy toggle: publish email once confirmed. Default false.
    pub show_email: bool,
    /// Privacy toggle: publish the city. Default true.
    pub show_city: bool,
    /// Whether reviews of this person are accepted. Default true.
    pub allow_reviews: bool,
    /// User who recommended the person; null if that account was deleted.
    pub recommended_by: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = person_claim_tokens)]
pub struct PersonClaimToken {
    /// Token row ID.
    pub id: Uuid,
    /// Person the claim link is for.
    pub person_id: Uuid,
    /// Secret token from the claim link; it IS the credential for /person/claim/{token}. Never expose it.
    pub token: String,
    /// Expiry, 30 days after creation (UTC).
    pub expires_at: NaiveDateTime,
    /// When the token was used to confirm or decline; a used token cannot be reused.
    pub used_at: Option<NaiveDateTime>,
    /// Issue time (UTC).
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = person_vouches)]
pub struct PersonVouch {
    /// Vouch ID.
    pub id: Uuid,
    /// Person being vouched for.
    pub person_id: Uuid,
    /// User who vouched (one vouch per user and person).
    pub user_id: Uuid,
    /// Optional comment.
    pub note: Option<String>,
    /// Vouch time (UTC).
    pub created_at: NaiveDateTime,
}

/// Claim-link lifetime: long enough for a manual send + a busy week.
const CLAIM_TOKEN_TTL_DAYS: i64 = 30;

impl Person {
    /// Creates the person, its language links and a claim token in one
    /// transaction. Returns the person together with the raw claim token —
    /// the only moment it leaves the system (given back to the recommender
    /// for manual sending until a mailer exists).
    pub fn create_with_claim_token(
        conn: &mut PgConnection,
        record: &CreatePerson,
        language_ids: &[Uuid],
    ) -> Result<(Self, String), AppError> {
        conn.transaction::<(Person, String), AppError, _>(|conn| {
            let person = diesel::insert_into(people::table)
                .values(record)
                .get_result::<Person>(conn)?;

            if !language_ids.is_empty() {
                let rows: Vec<_> = language_ids
                    .iter()
                    .map(|lang_id| {
                        (
                            people_to_languages::person_id.eq(person.id),
                            people_to_languages::language_id.eq(*lang_id),
                        )
                    })
                    .collect();

                diesel::insert_into(people_to_languages::table)
                    .values(rows)
                    .execute(conn)?;
            }

            let token = Self::issue_claim_token(conn, person.id)?;

            Ok((person, token))
        })
    }

    fn issue_claim_token(conn: &mut PgConnection, person_id: Uuid) -> Result<String, AppError> {
        // Two v4 UUIDs => 244 bits of randomness; opaque and unguessable.
        let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let expires_at =
            chrono::Utc::now().naive_utc() + chrono::Duration::days(CLAIM_TOKEN_TTL_DAYS);

        diesel::insert_into(person_claim_tokens::table)
            .values((
                person_claim_tokens::person_id.eq(person_id),
                person_claim_tokens::token.eq(&token),
                person_claim_tokens::expires_at.eq(expires_at),
            ))
            .execute(conn)?;

        Ok(token)
    }

    pub fn fetch_by_id(conn: &mut PgConnection, id: Uuid) -> Result<Self, AppError> {
        let person = people::table.find(id).first::<Person>(conn)?;

        Ok(person)
    }

    /// Public list: only confirmed/claimed people.
    pub fn list_public(
        conn: &mut PgConnection,
        filters: &ListPeopleFilters,
    ) -> Result<Vec<Self>, AppError> {
        let mut query = people::table
            .filter(people::status.eq_any(vec![
                PersonStatus::Confirmed.as_str(),
                PersonStatus::Claimed.as_str(),
            ]))
            .into_boxed();

        if let Some(city) = &filters.city {
            query = query.filter(people::city.ilike(format!("%{}%", city)));
        }

        if let Some(skills) = &filters.skills {
            let skills: Vec<Option<String>> = skills.iter().cloned().map(Some).collect();
            query = query.filter(people::skills.overlaps_with(skills));
        }

        if let Some(language_ids) = &filters.language_ids {
            let person_ids = people_to_languages::table
                .filter(people_to_languages::language_id.eq_any(language_ids))
                .select(people_to_languages::person_id)
                .load::<Uuid>(conn)?;
            query = query.filter(people::id.eq_any(person_ids));
        }

        let result = query
            .order(people::created_at.desc())
            .limit(filters.limit)
            .offset(filters.offset)
            .load::<Person>(conn)?;

        Ok(result)
    }

    pub fn languages_of(conn: &mut PgConnection, person_id: Uuid) -> Result<Vec<Uuid>, AppError> {
        let ids = people_to_languages::table
            .filter(people_to_languages::person_id.eq(person_id))
            .select(people_to_languages::language_id)
            .load::<Uuid>(conn)?;

        Ok(ids)
    }

    pub fn vouch(
        conn: &mut PgConnection,
        person_id: Uuid,
        user_id: Uuid,
        note: Option<String>,
    ) -> Result<PersonVouch, AppError> {
        let vouch = diesel::insert_into(person_vouches::table)
            .values((
                person_vouches::person_id.eq(person_id),
                person_vouches::user_id.eq(user_id),
                person_vouches::note.eq(note),
            ))
            .get_result::<PersonVouch>(conn)?;

        Ok(vouch)
    }

    pub fn vouch_count(conn: &mut PgConnection, person_id: Uuid) -> Result<i64, AppError> {
        let count = person_vouches::table
            .filter(person_vouches::person_id.eq(person_id))
            .count()
            .get_result::<i64>(conn)?;

        Ok(count)
    }

    /// Resolves a live (unused, unexpired) claim token to its person.
    pub fn find_by_claim_token(
        conn: &mut PgConnection,
        token: &str,
    ) -> Result<(Self, PersonClaimToken), AppError> {
        let claim = person_claim_tokens::table
            .filter(person_claim_tokens::token.eq(token))
            .first::<PersonClaimToken>(conn)
            .optional()?
            .ok_or_else(|| AppError::NotFound(json!({ "error": "Unknown claim token" })))?;

        if claim.used_at.is_some() {
            return Err(AppError::UnprocessableEntity(
                json!({ "error": "Claim token already used" }),
            ));
        }
        if claim.expires_at < chrono::Utc::now().naive_utc() {
            return Err(AppError::UnprocessableEntity(
                json!({ "error": "Claim token expired" }),
            ));
        }

        let person = Self::fetch_by_id(conn, claim.person_id)?;

        Ok((person, claim))
    }

    /// Confirm/decline via claim token: burns the token and moves the person
    /// to `confirmed` (or `claimed` when an authenticated user claims it),
    /// applying optional privacy toggles — all in one transaction.
    pub fn resolve_claim(
        conn: &mut PgConnection,
        token: &str,
        outcome: ClaimOutcome,
    ) -> Result<Self, AppError> {
        conn.transaction::<Person, AppError, _>(|conn| {
            let (person, claim) = Self::find_by_claim_token(conn, token)?;

            person.ensure_claimable(&outcome)?;

            diesel::update(person_claim_tokens::table.find(claim.id))
                .set(person_claim_tokens::used_at.eq(chrono::Utc::now().naive_utc()))
                .execute(conn)?;

            let updated = match outcome {
                ClaimOutcome::Decline => diesel::update(people::table.find(person.id))
                    .set((
                        people::status.eq(PersonStatus::Declined.as_str()),
                        people::updated_at.eq(chrono::Utc::now().naive_utc()),
                    ))
                    .get_result::<Person>(conn)?,
                ClaimOutcome::Confirm {
                    claimed_by,
                    show_whatsapp,
                    show_email,
                    show_city,
                    allow_reviews,
                } => {
                    let new_status = if claimed_by.is_some() {
                        PersonStatus::Claimed
                    } else {
                        PersonStatus::Confirmed
                    };

                    diesel::update(people::table.find(person.id))
                        .set((
                            people::status.eq(new_status.as_str()),
                            people::claimed_by.eq(claimed_by),
                            people::consent_given.eq(true),
                            people::show_whatsapp.eq(show_whatsapp.unwrap_or(person.show_whatsapp)),
                            people::show_email.eq(show_email.unwrap_or(person.show_email)),
                            people::show_city.eq(show_city.unwrap_or(person.show_city)),
                            people::allow_reviews.eq(allow_reviews.unwrap_or(person.allow_reviews)),
                            people::updated_at.eq(chrono::Utc::now().naive_utc()),
                        ))
                        .get_result::<Person>(conn)?
                }
            };

            Ok(updated)
        })
    }

    pub fn set_status(
        conn: &mut PgConnection,
        person_id: Uuid,
        status: PersonStatus,
        moderation_note: Option<String>,
    ) -> Result<Self, AppError> {
        let updated = diesel::update(people::table.find(person_id))
            .set((
                people::status.eq(status.as_str()),
                people::moderation_note.eq(moderation_note),
                people::updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .get_result::<Person>(conn)?;

        Ok(updated)
    }
}

pub enum ClaimOutcome {
    Confirm {
        claimed_by: Option<Uuid>,
        show_whatsapp: Option<bool>,
        show_email: Option<bool>,
        show_city: Option<bool>,
        allow_reviews: Option<bool>,
    },
    Decline,
}

pub struct ListPeopleFilters {
    /// Case-insensitive substring filter on the city.
    pub city: Option<String>,
    /// Only people with at least one of these skills.
    pub skills: Option<Vec<String>>,
    /// Only people speaking at least one of these languages.
    pub language_ids: Option<Vec<Uuid>>,
    /// Page size (already clamped by the controller).
    pub limit: i64,
    /// Rows to skip.
    pub offset: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_person_status_round_trip() {
        for status in [
            PersonStatus::Pending,
            PersonStatus::Awaiting,
            PersonStatus::Confirmed,
            PersonStatus::Claimed,
            PersonStatus::Declined,
        ] {
            assert_eq!(PersonStatus::try_from(status.as_str()).unwrap(), status);
        }
    }

    /// The exact strings are pinned by the CHECK constraint on people.status.
    #[test]
    fn test_person_status_as_str() {
        assert_eq!(PersonStatus::Pending.as_str(), "pending");
        assert_eq!(PersonStatus::Awaiting.as_str(), "awaiting");
        assert_eq!(PersonStatus::Confirmed.as_str(), "confirmed");
        assert_eq!(PersonStatus::Claimed.as_str(), "claimed");
        assert_eq!(PersonStatus::Declined.as_str(), "declined");
    }

    #[test]
    fn test_person_status_rejects_unknown() {
        match PersonStatus::try_from("live") {
            Err(AppError::UnprocessableEntity(_)) => (),
            other => panic!("expected UnprocessableEntity, got {:?}", other),
        }
        assert!(PersonStatus::try_from("").is_err());
    }

    /// `is_public` gates both the public list and the contact unlock in
    /// `PersonContent::from_gated` — only confirmed/claimed may pass.
    #[test]
    fn test_only_confirmed_and_claimed_are_public() {
        assert!(PersonStatus::Confirmed.is_public());
        assert!(PersonStatus::Claimed.is_public());

        for status in [
            PersonStatus::Pending,
            PersonStatus::Awaiting,
            PersonStatus::Declined,
        ] {
            assert!(!status.is_public(), "{:?} must not be public", status);
        }
    }

    #[test]
    fn test_send_via_round_trip() {
        for send_via in [SendVia::Email, SendVia::Whatsapp] {
            assert_eq!(SendVia::try_from(send_via.as_str()).unwrap(), send_via);
        }
        assert_eq!(SendVia::Email.as_str(), "email");
        assert_eq!(SendVia::Whatsapp.as_str(), "whatsapp");
    }

    #[test]
    fn test_send_via_rejects_unknown() {
        match SendVia::try_from("telegram") {
            Err(AppError::UnprocessableEntity(_)) => (),
            other => panic!("expected UnprocessableEntity, got {:?}", other),
        }
    }

    fn test_person(status: &str) -> Person {
        Person {
            id: Uuid::new_v4(),
            name: "Daria K.".to_string(),
            bio: None,
            city: None,
            location_country_id: None,
            skills: vec![],
            email: None,
            whatsapp: None,
            send_via: None,
            consent_given: true,
            status: status.to_string(),
            show_whatsapp: false,
            show_email: false,
            show_city: false,
            allow_reviews: true,
            recommended_by: None,
            claimed_by: None,
            moderation_note: None,
            rating_avg: None,
            reviews_count: 0,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }

    fn confirm(claimed_by: Option<Uuid>) -> ClaimOutcome {
        ClaimOutcome::Confirm {
            claimed_by,
            show_whatsapp: Some(true),
            show_email: Some(true),
            show_city: None,
            allow_reviews: None,
        }
    }

    /// Regression: the recommender got the claim token in the create
    /// response and could confirm a still-pending person, skipping moderation.
    #[test]
    fn test_confirm_requires_moderation_approval() {
        match test_person("pending").ensure_claimable(&confirm(None)) {
            Err(AppError::UnprocessableEntity(_)) => (),
            other => panic!("expected UnprocessableEntity, got {:?}", other),
        }

        assert!(
            test_person("awaiting")
                .ensure_claimable(&confirm(None))
                .is_ok()
        );
    }

    #[test]
    fn test_recommender_cannot_claim_the_profile() {
        let recommender = Uuid::new_v4();
        let mut person = test_person("awaiting");
        person.recommended_by = Some(recommender);

        match person.ensure_claimable(&confirm(Some(recommender))) {
            Err(AppError::Forbidden(_)) => (),
            other => panic!("expected Forbidden, got {:?}", other),
        }

        assert!(
            person
                .ensure_claimable(&confirm(Some(Uuid::new_v4())))
                .is_ok()
        );
    }

    #[test]
    fn test_decline_allowed_before_confirmation_only() {
        for status in ["pending", "awaiting"] {
            assert!(
                test_person(status)
                    .ensure_claimable(&ClaimOutcome::Decline)
                    .is_ok()
            );
        }
        for status in ["confirmed", "claimed", "declined"] {
            assert!(
                test_person(status)
                    .ensure_claimable(&ClaimOutcome::Decline)
                    .is_err()
            );
            assert!(
                test_person(status)
                    .ensure_claimable(&confirm(None))
                    .is_err()
            );
        }
    }

    #[test]
    fn test_status_enum_reads_the_stored_string() {
        assert_eq!(test_person("claimed").status_enum(), PersonStatus::Claimed);
    }

    /// An unreadable status must degrade to the most restrictive state, never
    /// to a public one — otherwise a bad row would leak contacts.
    #[test]
    fn test_status_enum_falls_back_to_pending() {
        let person = test_person("something-unexpected");

        assert_eq!(person.status_enum(), PersonStatus::Pending);
        assert!(!person.status_enum().is_public());
    }
}
