use super::{
    entities::{ClaimOutcome, CreatePerson, ListPeopleFilters, PersonStatus, SendVia},
    presenters::PersonPresenter,
    repositories::PersonRepository,
};
use crate::app::features::moderation::entities::TargetKind;
use crate::app::features::moderation::repositories::{ModerationRepository, SubmittedEventInput};
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::features::moderation::entities::{ModerationAction, QueueItem};
    use crate::app::features::person::entities::{Person, PersonVouch};
    use crate::app::features::person::presenters::PersonPresenterImpl;
    use crate::app::features::user::entities::UserRole;
    use std::sync::Mutex;

    #[derive(Default)]
    struct StubRepo {
        /// Status of the person served by `fetch_person`.
        person_status: Option<PersonStatus>,
        created: Mutex<Vec<CreatePerson>>,
        vouched: Mutex<Vec<Uuid>>,
    }

    impl StubRepo {
        fn person(&self) -> Person {
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
                status: self
                    .person_status
                    .unwrap_or(PersonStatus::Pending)
                    .as_str()
                    .to_string(),
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
    }

    impl PersonRepository for StubRepo {
        fn create_person(
            &self,
            record: CreatePerson,
            _language_ids: Vec<Uuid>,
        ) -> Result<(Person, String), AppError> {
            let mut person = self.person();
            person.status = record.status.clone();
            self.created.lock().unwrap().push(record);

            Ok((person, "claim-token".to_string()))
        }

        fn fetch_person(&self, _id: Uuid) -> Result<(Person, i64, Vec<Uuid>), AppError> {
            Ok((self.person(), 0, vec![]))
        }

        fn list_people(&self, _filters: ListPeopleFilters) -> Result<Vec<(Person, i64)>, AppError> {
            Ok(vec![])
        }

        fn vouch_person(
            &self,
            person_id: Uuid,
            user_id: Uuid,
            note: Option<String>,
        ) -> Result<PersonVouch, AppError> {
            self.vouched.lock().unwrap().push(person_id);

            Ok(PersonVouch {
                id: Uuid::new_v4(),
                person_id,
                user_id,
                note,
                created_at: chrono::Utc::now().naive_utc(),
            })
        }

        fn fetch_by_claim_token(&self, _token: &str) -> Result<Person, AppError> {
            Ok(self.person())
        }

        fn resolve_claim(&self, _token: &str, _outcome: ClaimOutcome) -> Result<Person, AppError> {
            Ok(self.person())
        }

        fn count_public_people_recommended_by(&self, _user_id: Uuid) -> Result<i64, AppError> {
            Ok(0)
        }
    }

    /// The submit-time auto-check write is best-effort; it must never be the
    /// reason a recommendation fails.
    struct FailingModerationRepo;

    impl ModerationRepository for FailingModerationRepo {
        fn record_submitted(&self, _input: SubmittedEventInput) -> Result<(), AppError> {
            Err(AppError::InternalServerError)
        }

        fn fetch_queue(&self, _kind: Option<TargetKind>) -> Result<Vec<QueueItem>, AppError> {
            unreachable!()
        }

        fn moderate(
            &self,
            _kind: TargetKind,
            _target_id: Uuid,
            _action: ModerationAction,
            _note: Option<String>,
            _moderator_id: Uuid,
        ) -> Result<(), AppError> {
            unreachable!()
        }

        fn fetch_user_role(&self, _user_id: Uuid) -> Result<UserRole, AppError> {
            unreachable!()
        }
    }

    fn usecase_with(repo: Arc<StubRepo>) -> PersonUsecase {
        PersonUsecase::new(
            repo,
            Arc::new(PersonPresenterImpl::new()),
            Arc::new(FailingModerationRepo),
        )
    }

    fn create_input() -> CreatePersonUsecaseInput {
        CreatePersonUsecaseInput {
            name: "Daria K.".to_string(),
            bio: None,
            city: None,
            location_country_id: None,
            skills: vec![],
            language_ids: vec![],
            email: Some("daria@example.com".to_string()),
            whatsapp: None,
            send_via: Some("email".to_string()),
            consent_given: true,
            show_whatsapp: false,
            show_email: true,
            show_city: true,
            allow_reviews: true,
            recommended_by: Uuid::new_v4(),
        }
    }

    /// A person is a real human who never asked to be listed — without the
    /// recommender's consent confirmation nothing may be written.
    #[test]
    fn test_create_person_requires_consent() {
        let repo = Arc::new(StubRepo::default());
        let mut params = create_input();
        params.consent_given = false;

        match usecase_with(repo.clone()).create_person(params) {
            Err(AppError::UnprocessableEntity(_)) => (),
            other => panic!("expected UnprocessableEntity, got {:?}", other.err()),
        }
        assert!(repo.created.lock().unwrap().is_empty());
    }

    /// Without a contact the claim link can never be delivered, so the person
    /// would be stuck in `pending` forever.
    #[test]
    fn test_create_person_requires_a_contact() {
        let repo = Arc::new(StubRepo::default());
        let mut params = create_input();
        params.email = None;
        params.whatsapp = None;

        match usecase_with(repo.clone()).create_person(params) {
            Err(AppError::UnprocessableEntity(_)) => (),
            other => panic!("expected UnprocessableEntity, got {:?}", other.err()),
        }
        assert!(repo.created.lock().unwrap().is_empty());
    }

    #[test]
    fn test_create_person_accepts_whatsapp_only() {
        let repo = Arc::new(StubRepo::default());
        let mut params = create_input();
        params.email = None;
        params.whatsapp = Some("+49111222333".to_string());
        params.send_via = Some("whatsapp".to_string());

        assert!(usecase_with(repo.clone()).create_person(params).is_ok());
        assert_eq!(repo.created.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_create_person_rejects_unknown_send_via() {
        let repo = Arc::new(StubRepo::default());
        let mut params = create_input();
        params.send_via = Some("telegram".to_string());

        match usecase_with(repo.clone()).create_person(params) {
            Err(AppError::UnprocessableEntity(_)) => (),
            other => panic!("expected UnprocessableEntity, got {:?}", other.err()),
        }
        assert!(repo.created.lock().unwrap().is_empty());
    }

    /// New recommendations always enter the moderation queue.
    #[test]
    fn test_create_person_starts_pending() {
        let repo = Arc::new(StubRepo::default());

        usecase_with(repo.clone())
            .create_person(create_input())
            .unwrap();

        let created = repo.created.lock().unwrap();
        assert_eq!(created[0].status, PersonStatus::Pending.as_str());
        assert!(created[0].recommended_by.is_some());
    }

    #[test]
    fn test_create_person_survives_a_failing_moderation_write() {
        let repo = Arc::new(StubRepo::default());

        // FailingModerationRepo::record_submitted always errors.
        assert!(
            usecase_with(repo.clone())
                .create_person(create_input())
                .is_ok()
        );
        assert_eq!(repo.created.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_vouch_requires_a_public_person() {
        for status in [
            PersonStatus::Pending,
            PersonStatus::Awaiting,
            PersonStatus::Declined,
        ] {
            let repo = Arc::new(StubRepo {
                person_status: Some(status),
                ..Default::default()
            });

            match usecase_with(repo.clone()).vouch_person(Uuid::new_v4(), Uuid::new_v4(), None) {
                Err(AppError::UnprocessableEntity(_)) => (),
                other => panic!(
                    "expected UnprocessableEntity for {:?}, got {:?}",
                    status,
                    other.err()
                ),
            }
            assert!(repo.vouched.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn test_vouch_allowed_for_confirmed_and_claimed() {
        for status in [PersonStatus::Confirmed, PersonStatus::Claimed] {
            let repo = Arc::new(StubRepo {
                person_status: Some(status),
                ..Default::default()
            });

            assert!(
                usecase_with(repo.clone())
                    .vouch_person(Uuid::new_v4(), Uuid::new_v4(), None)
                    .is_ok(),
                "vouching must be allowed for {:?}",
                status
            );
            assert_eq!(repo.vouched.lock().unwrap().len(), 1);
        }
    }
}
