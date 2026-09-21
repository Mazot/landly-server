use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreatePersonRequest {
    pub name: String,
    pub bio: Option<String>,
    pub city: Option<String>,
    pub location_country_id: Option<Uuid>,
    /// e.g. ["Relocation", "Paperwork & legal", "Housing", "Language"]
    pub skills: Option<Vec<String>>,
    pub language_ids: Option<Vec<Uuid>>,
    /// Hidden contact — used only for the claim link, gated afterwards
    pub email: Option<String>,
    pub whatsapp: Option<String>,
    /// "email" | "whatsapp" — how the recommender sends the claim link
    pub send_via: Option<String>,
    /// Must be true: the person agreed to be recommended
    pub consent_given: bool,
    pub show_whatsapp: Option<bool>,
    pub show_email: Option<bool>,
    pub show_city: Option<bool>,
    pub allow_reviews: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, ToSchema, IntoParams)]
pub struct ListPeopleQueryRequest {
    /// Comma-separated skill names, e.g. "Relocation,Housing"
    pub skills: Option<String>,
    pub city: Option<String>,
    /// Comma-separated language UUIDs
    pub language_ids: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct VouchPersonRequest {
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ClaimConfirmRequest {
    pub show_whatsapp: Option<bool>,
    pub show_email: Option<bool>,
    pub show_city: Option<bool>,
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
