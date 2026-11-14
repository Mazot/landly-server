use super::entities::{Person, RecommendedPerson};
use actix_web::HttpResponse;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub trait PersonPresenter: Send + Sync + 'static {
    fn to_http_res(&self) -> HttpResponse;
    fn to_single_json(&self, person: Person, vouches: i64, language_ids: Vec<Uuid>)
    -> HttpResponse;
    fn to_created_json(&self, person: Person, claim_url: String) -> HttpResponse;
    fn to_multi_json(&self, items: Vec<(Person, i64)>) -> HttpResponse;
    fn to_claim_preview_json(&self, person: Person) -> HttpResponse;
    /// The caller's own recommendations, with the moderator's note.
    fn to_mine_json(&self, items: Vec<RecommendedPerson>) -> HttpResponse;
}

/// Public person payload. Contacts (email/whatsapp) are serialized ONLY when
/// the person is confirmed/claimed AND the matching privacy toggle is on;
/// the city respects show_city the same way.
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PersonContent {
    /// Person ID.
    pub id: Uuid,
    /// Display name, e.g. "Daria K.".
    pub name: String,
    /// Short free-text bio.
    pub bio: Option<String>,
    /// City; null unless `show_city` is on.
    pub city: Option<String>,
    /// Country the person lives in.
    pub location_country_id: Option<Uuid>,
    /// What they help with, free-text tags, e.g. ["Relocation", "Housing"].
    pub skills: Vec<String>,
    /// Languages the person speaks.
    pub language_ids: Vec<Uuid>,
    /// `pending` (moderation) → `awaiting` (approved, waiting for the person) → `confirmed` / `claimed` (linked to an account), or `declined`. Only confirmed/claimed are public.
    pub status: String,
    /// Present only when confirmed/claimed and `show_email` is on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Present only when confirmed/claimed and `show_whatsapp` is on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whatsapp: Option<String>,
    /// Whether reviews of this person are accepted. Default true.
    pub allow_reviews: bool,
    /// Average review rating (1–5); null without reviews. Recomputed on every review create/delete.
    pub rating_avg: Option<f64>,
    /// Number of reviews.
    pub reviews_count: i64,
    /// Number of users who vouched for this person.
    pub vouches: i64,
    /// Recommendation time (UTC, no timezone).
    pub created_at: NaiveDateTime,
}

impl PersonContent {
    /// The single choke point for contact privacy.
    pub fn from_gated(person: Person, vouches: i64, language_ids: Vec<Uuid>) -> Self {
        let contacts_unlocked = person.status_enum().is_public();

        Self {
            id: person.id,
            name: person.name,
            bio: person.bio,
            city: if person.show_city { person.city } else { None },
            location_country_id: person.location_country_id,
            skills: person.skills.into_iter().flatten().collect(),
            language_ids,
            status: person.status,
            email: if contacts_unlocked && person.show_email {
                person.email
            } else {
                None
            },
            whatsapp: if contacts_unlocked && person.show_whatsapp {
                person.whatsapp
            } else {
                None
            },
            allow_reviews: person.allow_reviews,
            rating_avg: person.rating_avg,
            reviews_count: person.reviews_count,
            vouches,
            created_at: person.created_at,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct MultiplePeopleResponse {
    /// Public (confirmed/claimed) people on this page.
    pub items: Vec<PersonContent>,
    /// Number of items returned.
    pub total: i64,
}

/// A recommendation as its recommender sees it (`GET /person/mine`): the
/// public payload (contacts gated exactly like everywhere else) plus what only
/// the recommender is told — the moderator's note and the last change.
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MyPersonContent {
    #[serde(flatten)]
    pub person: PersonContent,
    /// Moderator's note to the recommender (request-changes / reject).
    pub moderation_note: Option<String>,
    /// Last change: moderation, or the person answering the claim link (UTC, no timezone).
    pub updated_at: NaiveDateTime,
    /// `declined` because a moderator rejected it — not because the person turned it down.
    pub rejected_by_moderator: bool,
}

impl From<RecommendedPerson> for MyPersonContent {
    fn from(item: RecommendedPerson) -> Self {
        let RecommendedPerson {
            mut person,
            vouches,
            language_ids,
            rejected_by_moderator,
        } = item;
        let moderation_note = person.moderation_note.take();
        let updated_at = person.updated_at;

        Self {
            person: PersonContent::from_gated(person, vouches, language_ids),
            moderation_note,
            updated_at,
            rejected_by_moderator,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct MyPeopleResponse {
    /// The caller's recommendations on this page, newest first.
    pub items: Vec<MyPersonContent>,
    /// Number of items returned.
    pub total: i64,
}

/// Create response: person + the claim link the recommender sends manually
/// (`send_via` says how) until a mailer exists.
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PersonCreatedContent {
    /// The created recommendation (status `pending`).
    pub person: PersonContent,
    /// Relative claim link `/claim/{token}` for the recommender to send; valid 30 days.
    pub claim_url: String,
    /// How the recommender sends the claim link: `email` or `whatsapp`.
    pub send_via: Option<String>,
}

/// What the claimed person sees on GET /claim/{token}: enough to decide,
/// including the contacts the recommender entered about them.
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClaimPreviewContent {
    /// Display name, e.g. "Daria K.".
    pub name: String,
    /// Short free-text bio.
    pub bio: Option<String>,
    /// City as entered by the recommender.
    pub city: Option<String>,
    /// What they help with, free-text tags, e.g. ["Relocation", "Housing"].
    pub skills: Vec<String>,
    /// `pending` (moderation) → `awaiting` (approved, waiting for the person) → `confirmed` / `claimed` (linked to an account), or `declined`. Only confirmed/claimed are public.
    pub status: String,
    /// Email the recommender entered (shown to the person being claimed only).
    pub email: Option<String>,
    /// WhatsApp the recommender entered (shown to the person being claimed only).
    pub whatsapp: Option<String>,
    /// Privacy toggle: publish WhatsApp once confirmed. Default false.
    pub show_whatsapp: bool,
    /// Privacy toggle: publish email once confirmed. Default false.
    pub show_email: bool,
    /// Privacy toggle: publish the city. Default true.
    pub show_city: bool,
    /// Whether reviews of this person are accepted. Default true.
    pub allow_reviews: bool,
}

#[derive(Clone)]
pub struct PersonPresenterImpl {}

impl PersonPresenterImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl PersonPresenter for PersonPresenterImpl {
    fn to_http_res(&self) -> HttpResponse {
        HttpResponse::Ok().json("OK")
    }

    fn to_single_json(
        &self,
        person: Person,
        vouches: i64,
        language_ids: Vec<Uuid>,
    ) -> HttpResponse {
        HttpResponse::Ok().json(PersonContent::from_gated(person, vouches, language_ids))
    }

    fn to_created_json(&self, person: Person, claim_url: String) -> HttpResponse {
        let send_via = person.send_via.clone();
        let content = PersonCreatedContent {
            person: PersonContent::from_gated(person, 0, vec![]),
            claim_url,
            send_via,
        };

        HttpResponse::Ok().json(content)
    }

    fn to_multi_json(&self, items: Vec<(Person, i64)>) -> HttpResponse {
        let items: Vec<PersonContent> = items
            .into_iter()
            .map(|(person, vouches)| PersonContent::from_gated(person, vouches, vec![]))
            .collect();
        let total = items.len() as i64;

        HttpResponse::Ok().json(MultiplePeopleResponse { items, total })
    }

    fn to_mine_json(&self, items: Vec<RecommendedPerson>) -> HttpResponse {
        let items: Vec<MyPersonContent> = items.into_iter().map(MyPersonContent::from).collect();
        let total = items.len() as i64;

        HttpResponse::Ok().json(MyPeopleResponse { items, total })
    }

    fn to_claim_preview_json(&self, person: Person) -> HttpResponse {
        HttpResponse::Ok().json(ClaimPreviewContent {
            name: person.name,
            bio: person.bio,
            city: person.city,
            skills: person.skills.into_iter().flatten().collect(),
            status: person.status,
            email: person.email,
            whatsapp: person.whatsapp,
            show_whatsapp: person.show_whatsapp,
            show_email: person.show_email,
            show_city: person.show_city,
            allow_reviews: person.allow_reviews,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_person(status: &str) -> Person {
        Person {
            id: Uuid::new_v4(),
            name: "Daria K.".to_string(),
            bio: Some("relocation help".to_string()),
            city: Some("Berlin".to_string()),
            location_country_id: None,
            skills: vec![Some("Relocation".to_string()), Some("Housing".to_string())],
            email: Some("daria@example.com".to_string()),
            whatsapp: Some("+49111222333".to_string()),
            send_via: Some("whatsapp".to_string()),
            consent_given: true,
            status: status.to_string(),
            show_whatsapp: true,
            show_email: true,
            show_city: true,
            allow_reviews: true,
            recommended_by: Some(Uuid::new_v4()),
            claimed_by: None,
            moderation_note: None,
            rating_avg: Some(5.0),
            reviews_count: 27,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }

    /// Core privacy invariant: contacts never leak before confirmation,
    /// whatever the toggles say.
    #[test]
    fn test_contacts_hidden_until_confirmed() {
        for status in ["pending", "awaiting", "declined"] {
            let content = PersonContent::from_gated(test_person(status), 0, vec![]);
            assert!(content.email.is_none(), "email leaked for {}", status);
            assert!(content.whatsapp.is_none(), "whatsapp leaked for {}", status);

            let json = serde_json::to_string(&content).unwrap();
            assert!(!json.contains("daria@example.com"));
            assert!(!json.contains("+49111222333"));
        }
    }

    #[test]
    fn test_contacts_visible_when_confirmed_and_allowed() {
        for status in ["confirmed", "claimed"] {
            let content = PersonContent::from_gated(test_person(status), 3, vec![]);
            assert_eq!(content.email.as_deref(), Some("daria@example.com"));
            assert_eq!(content.whatsapp.as_deref(), Some("+49111222333"));
            assert_eq!(content.vouches, 3);
        }
    }

    /// The recommender's own list adds the moderator's note, but opens no contact
    /// the public payload keeps closed.
    #[test]
    fn test_mine_carries_the_note_and_keeps_contacts_gated() {
        let mut person = test_person("pending");
        person.moderation_note = Some("Add a city".to_string());
        let updated_at = person.updated_at;

        let content = MyPersonContent::from(RecommendedPerson {
            person,
            vouches: 2,
            language_ids: vec![],
            rejected_by_moderator: false,
        });
        assert_eq!(content.moderation_note.as_deref(), Some("Add a city"));
        assert!(!content.rejected_by_moderator);
        assert_eq!(content.updated_at, updated_at);
        assert_eq!(content.person.vouches, 2);
        assert!(content.person.email.is_none());
        assert!(content.person.whatsapp.is_none());

        // Flattened: the person's fields sit next to the note, not under a key.
        let json = serde_json::to_value(&content).unwrap();
        assert_eq!(json["status"], "pending");
        assert_eq!(json["moderationNote"], "Add a city");
        assert!(json.get("person").is_none());
        assert!(json.get("email").is_none());
    }

    #[test]
    fn test_privacy_toggles_win_even_when_confirmed() {
        let mut person = test_person("confirmed");
        person.show_email = false;
        person.show_whatsapp = false;
        person.show_city = false;

        let content = PersonContent::from_gated(person, 0, vec![]);
        assert!(content.email.is_none());
        assert!(content.whatsapp.is_none());
        assert!(content.city.is_none());
    }

    /// The moderation note and recommender id are internal.
    #[test]
    fn test_internal_fields_not_serialized() {
        let mut person = test_person("confirmed");
        person.moderation_note = Some("looks suspicious".to_string());

        let json = serde_json::to_string(&PersonContent::from_gated(person, 0, vec![])).unwrap();
        assert!(!json.contains("looks suspicious"));
        assert!(!json.contains("recommendedBy"));
        assert!(!json.contains("sendVia"));
    }
}
