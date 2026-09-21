use super::entities::Person;
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
}

/// Public person payload. Contacts (email/whatsapp) are serialized ONLY when
/// the person is confirmed/claimed AND the matching privacy toggle is on;
/// the city respects show_city the same way.
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PersonContent {
    pub id: Uuid,
    pub name: String,
    pub bio: Option<String>,
    pub city: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub skills: Vec<String>,
    pub language_ids: Vec<Uuid>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub whatsapp: Option<String>,
    pub allow_reviews: bool,
    pub rating_avg: Option<f64>,
    pub reviews_count: i64,
    pub vouches: i64,
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
    pub items: Vec<PersonContent>,
    pub total: i64,
}

/// Create response: person + the claim link the recommender sends manually
/// (`send_via` says how) until a mailer exists.
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PersonCreatedContent {
    pub person: PersonContent,
    pub claim_url: String,
    pub send_via: Option<String>,
}

/// What the claimed person sees on GET /claim/{token}: enough to decide,
/// including the contacts the recommender entered about them.
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClaimPreviewContent {
    pub name: String,
    pub bio: Option<String>,
    pub city: Option<String>,
    pub skills: Vec<String>,
    pub status: String,
    pub email: Option<String>,
    pub whatsapp: Option<String>,
    pub show_whatsapp: bool,
    pub show_email: bool,
    pub show_city: bool,
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
