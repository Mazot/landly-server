use super::{
    entities::{ClaimOutcome, CreatePerson, ListPeopleFilters, PersonStatus, SendVia},
    presenters::PersonPresenter,
    repositories::PersonRepository,
};
use crate::app::features::moderation::repositories::{
    ModerationRepository, SubmittedEventInput, TargetKind,
};
use crate::error::AppError;
use actix_web::HttpResponse;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct PersonUsecase {
    person_repo: Arc<dyn PersonRepository>,
    person_presenter: Arc<dyn PersonPresenter>,
    moderation_repo: Arc<dyn ModerationRepository>,
}

impl PersonUsecase {
    pub fn new(
        person_repo: Arc<dyn PersonRepository>,
        person_presenter: Arc<dyn PersonPresenter>,
        moderation_repo: Arc<dyn ModerationRepository>,
    ) -> Self {
        Self {
            person_repo,
            person_presenter,
            moderation_repo,
        }
    }

    /// Recommends a person. Requires explicit consent confirmation from the
    /// recommender; the created person starts in `pending` (moderation) and
    /// the claim link is returned for manual sending (`send_via`).
    pub fn create_person(
        &self,
        params: CreatePersonUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        if !params.consent_given {
            return Err(AppError::UnprocessableEntity(json!({
                "error": "consent_given is required: the person must have agreed to be recommended"
            })));
        }
        if let Some(send_via) = params.send_via.as_deref() {
            SendVia::try_from(send_via)?;
        }
        if params.email.is_none() && params.whatsapp.is_none() {
            return Err(AppError::UnprocessableEntity(json!({
                "error": "At least one contact (email or whatsapp) is required to send the claim link"
            })));
        }

        let (person, token) = self.person_repo.create_person(
            CreatePerson {
                name: params.name,
                bio: params.bio,
                city: params.city,
                location_country_id: params.location_country_id,
                skills: params.skills.into_iter().map(Some).collect(),
                email: params.email,
                whatsapp: params.whatsapp,
                send_via: params.send_via,
                consent_given: params.consent_given,
                status: PersonStatus::Pending.as_str().to_string(),
                show_whatsapp: params.show_whatsapp,
                show_email: params.show_email,
                show_city: params.show_city,
                allow_reviews: params.allow_reviews,
                recommended_by: Some(params.recommended_by),
            },
            params.language_ids,
        )?;

        // Submit-time auto-checks for the moderation queue.
        let trusted_recommender = self
            .person_repo
            .count_public_people_recommended_by(params.recommended_by)?;
        let _ = self.moderation_repo.record_submitted(SubmittedEventInput {
            target_kind: TargetKind::Person,
            target_id: person.id,
            flags: json!({
                "recommenderApprovedPeople": trusted_recommender,
                "trustedRecommender": trusted_recommender >= 3,
            }),
        });

        let claim_url = format!("/claim/{}", token);

        Ok(self.person_presenter.to_created_json(person, claim_url))
    }

    pub fn fetch_person(&self, id: Uuid) -> Result<HttpResponse, AppError> {
        let (person, vouches, language_ids) = self.person_repo.fetch_person(id)?;

        Ok(self
            .person_presenter
            .to_single_json(person, vouches, language_ids))
    }

    pub fn list_people(&self, filters: ListPeopleFilters) -> Result<HttpResponse, AppError> {
        let items = self.person_repo.list_people(filters)?;

        Ok(self.person_presenter.to_multi_json(items))
    }

    /// Vouch is only meaningful for people already visible publicly.
    pub fn vouch_person(
        &self,
        person_id: Uuid,
        user_id: Uuid,
        note: Option<String>,
    ) -> Result<HttpResponse, AppError> {
        let (person, _, _) = self.person_repo.fetch_person(person_id)?;
        if !person.status_enum().is_public() {
            return Err(AppError::UnprocessableEntity(
                json!({ "error": "Only confirmed people can be vouched for" }),
            ));
        }

        self.person_repo.vouch_person(person_id, user_id, note)?;

        self.fetch_person(person_id)
    }

    /// Public claim preview — the token is the credential.
    pub fn claim_preview(&self, token: &str) -> Result<HttpResponse, AppError> {
        let person = self.person_repo.fetch_by_claim_token(token)?;

        Ok(self.person_presenter.to_claim_preview_json(person))
    }

    pub fn claim_confirm(
        &self,
        token: &str,
        params: ClaimConfirmUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        let person = self.person_repo.resolve_claim(
            token,
            ClaimOutcome::Confirm {
                claimed_by: params.claimed_by,
                show_whatsapp: params.show_whatsapp,
                show_email: params.show_email,
                show_city: params.show_city,
                allow_reviews: params.allow_reviews,
            },
        )?;

        Ok(self.person_presenter.to_claim_preview_json(person))
    }

    pub fn claim_decline(&self, token: &str) -> Result<HttpResponse, AppError> {
        self.person_repo
            .resolve_claim(token, ClaimOutcome::Decline)?;

        Ok(self.person_presenter.to_http_res())
    }
}

pub struct CreatePersonUsecaseInput {
    pub name: String,
    pub bio: Option<String>,
    pub city: Option<String>,
    pub location_country_id: Option<Uuid>,
    pub skills: Vec<String>,
    pub language_ids: Vec<Uuid>,
    pub email: Option<String>,
    pub whatsapp: Option<String>,
    pub send_via: Option<String>,
    pub consent_given: bool,
    pub show_whatsapp: bool,
    pub show_email: bool,
    pub show_city: bool,
    pub allow_reviews: bool,
    pub recommended_by: Uuid,
}

pub struct ClaimConfirmUsecaseInput {
    pub claimed_by: Option<Uuid>,
    pub show_whatsapp: Option<bool>,
    pub show_email: Option<bool>,
    pub show_city: Option<bool>,
    pub allow_reviews: Option<bool>,
}
