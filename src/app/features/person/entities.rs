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
    pub id: Uuid,
    pub name: String,
    pub bio: Option<String>,
    pub city: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub skills: Vec<Option<String>>,
    pub email: Option<String>,
    pub whatsapp: Option<String>,
    pub send_via: Option<String>,
    pub consent_given: bool,
    pub status: String,
    pub show_whatsapp: bool,
    pub show_email: bool,
    pub show_city: bool,
    pub allow_reviews: bool,
    pub recommended_by: Option<Uuid>,
    pub claimed_by: Option<Uuid>,
    pub moderation_note: Option<String>,
    pub rating_avg: Option<f64>,
    pub reviews_count: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Person {
    pub fn status_enum(&self) -> PersonStatus {
        PersonStatus::try_from(self.status.as_str()).unwrap_or(PersonStatus::Pending)
    }
}

#[derive(Insertable, Clone)]
#[diesel(table_name = people)]
pub struct CreatePerson {
    pub name: String,
    pub bio: Option<String>,
    pub city: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub skills: Vec<Option<String>>,
    pub email: Option<String>,
    pub whatsapp: Option<String>,
    pub send_via: Option<String>,
    pub consent_given: bool,
    pub status: String,
    pub show_whatsapp: bool,
    pub show_email: bool,
    pub show_city: bool,
    pub allow_reviews: bool,
    pub recommended_by: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = person_claim_tokens)]
pub struct PersonClaimToken {
    pub id: Uuid,
    pub person_id: Uuid,
    pub token: String,
    pub expires_at: NaiveDateTime,
    pub used_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = person_vouches)]
pub struct PersonVouch {
    pub id: Uuid,
    pub person_id: Uuid,
    pub user_id: Uuid,
    pub note: Option<String>,
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

            let status = person.status_enum();
            if !matches!(status, PersonStatus::Pending | PersonStatus::Awaiting) {
                return Err(AppError::UnprocessableEntity(json!({
                    "error": format!("Person is already {}", person.status)
                })));
            }

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
    pub city: Option<String>,
    pub skills: Option<Vec<String>>,
    pub language_ids: Option<Vec<Uuid>>,
    pub limit: i64,
    pub offset: i64,
}
