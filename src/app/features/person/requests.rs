use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreatePersonRequest {
    /// Display name, e.g. "Daria K.".
    pub name: String,
    /// Short free-text bio.
    pub bio: Option<String>,
    /// City; shown publicly only when `show_city` is on.
    pub city: Option<String>,
    /// Country the person lives in.
    pub location_country_id: Option<Uuid>,
    /// e.g. ["Relocation", "Paperwork & legal", "Housing", "Language"]
    pub skills: Option<Vec<String>>,
    /// Languages the person speaks.
    pub language_ids: Option<Vec<Uuid>>,
    /// Hidden contact — used only for the claim link, gated afterwards
    pub email: Option<String>,
    /// Hidden contact — used only for the claim link, gated afterwards. Email or WhatsApp is required.
    pub whatsapp: Option<String>,
    /// "email" | "whatsapp" — how the recommender sends the claim link
    pub send_via: Option<String>,
    /// Must be true: the person agreed to be recommended
    pub consent_given: bool,
    /// Publish WhatsApp once confirmed. Default false.
    pub show_whatsapp: Option<bool>,
    /// Publish email once confirmed. Default false.
    pub show_email: Option<bool>,
    /// Publish the city. Default true.
    pub show_city: Option<bool>,
    /// Accept reviews. Default true.
    pub allow_reviews: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, ToSchema, IntoParams)]
pub struct ListPeopleQueryRequest {
    /// Comma-separated skill names, e.g. "Relocation,Housing"
    pub skills: Option<String>,
    /// Case-insensitive substring filter on the city.
    pub city: Option<String>,
    /// Comma-separated language UUIDs
    pub language_ids: Option<String>,
    /// Page size (default 20, max 100).
    pub limit: Option<i64>,
    /// Rows to skip (default 0, max 500).
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct VouchPersonRequest {
    /// Optional comment; stored with the vouch, not returned by the API yet.
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ClaimConfirmRequest {
    /// Privacy toggle: publish WhatsApp once confirmed; omit to keep the current value.
    pub show_whatsapp: Option<bool>,
    /// Privacy toggle: publish email once confirmed; omit to keep the current value.
    pub show_email: Option<bool>,
    /// Privacy toggle: publish the city; omit to keep the current value.
    pub show_city: Option<bool>,
    /// Whether reviews are accepted; omit to keep the current value.
    pub allow_reviews: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_person_request_minimal() {
        let json = r#"{"name": "Daria K.", "consent_given": true}"#;
        let request: CreatePersonRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.name, "Daria K.");
        assert!(request.consent_given);
        assert!(request.skills.is_none());
    }

    #[test]
    fn test_claim_confirm_request_empty_body() {
        let request: ClaimConfirmRequest = serde_json::from_str("{}").unwrap();
        assert!(request.show_whatsapp.is_none());
    }
}
