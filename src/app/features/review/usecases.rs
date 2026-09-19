use super::{
    entities::{CreateReview, ReviewTarget, validate_rating},
    presenters::ReviewPresenter,
    repositories::ReviewRepository,
};
use crate::app::features::organisation::entities::OrganisationStatus;
use crate::error::AppError;
use actix_web::HttpResponse;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ReviewUsecase {
    review_repo: Arc<dyn ReviewRepository>,
    review_presenter: Arc<dyn ReviewPresenter>,
}

impl ReviewUsecase {
    pub fn new(
        review_repo: Arc<dyn ReviewRepository>,
        review_presenter: Arc<dyn ReviewPresenter>,
    ) -> Self {
        Self {
            review_repo,
            review_presenter,
        }
    }

    /// Creates a review for exactly one target. Orgs must be live; people
    /// must be public AND have reviews allowed (`allow_reviews` toggle).
    pub fn create_review(
        &self,
        params: CreateReviewUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        validate_rating(params.rating)?;

        match (params.organisation_id, params.person_id) {
            (Some(org_id), None) => {
                let org = self.review_repo.fetch_target_organisation(org_id)?;
                if org.status != OrganisationStatus::Live.as_str() {
                    return Err(AppError::UnprocessableEntity(
                        json!({ "error": "Only live organisations can be reviewed" }),
                    ));
                }
            }
            (None, Some(person_id)) => {
                let person = self.review_repo.fetch_target_person(person_id)?;
                if !person.status_enum().is_public() {
                    return Err(AppError::UnprocessableEntity(
                        json!({ "error": "Only confirmed people can be reviewed" }),
                    ));
                }
                if !person.allow_reviews {
                    return Err(AppError::Forbidden(
                        json!({ "error": "This person has disabled reviews" }),
                    ));
                }
            }
            _ => {
                return Err(AppError::UnprocessableEntity(json!({
                    "error": "Provide exactly one of organisation_id or person_id"
                })));
            }
        }

        let review = self.review_repo.create_review(CreateReview {
            author_id: params.author_id,
            organisation_id: params.organisation_id,
            person_id: params.person_id,
            rating: params.rating,
            topic: params.topic,
            text: params.text,
        })?;

        Ok(self.review_presenter.to_single_json(review))
    }

    pub fn list_reviews(
        &self,
        organisation_id: Option<Uuid>,
        person_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<HttpResponse, AppError> {
        let target = match (organisation_id, person_id) {
            (Some(org_id), None) => ReviewTarget::Organisation(org_id),
            (None, Some(person_id)) => ReviewTarget::Person(person_id),
            _ => {
                return Err(AppError::UnprocessableEntity(json!({
                    "error": "Provide exactly one of organisation_id or person_id"
                })));
            }
        };

        let reviews = self.review_repo.list_reviews(target, limit, offset)?;

        Ok(self.review_presenter.to_multi_json(reviews))
    }

    /// Delete: author or moderator/admin.
    pub fn delete_review(
        &self,
        review_id: Uuid,
        caller_user_id: Uuid,
    ) -> Result<HttpResponse, AppError> {
        let review = self.review_repo.fetch_review(review_id)?;

        if review.author_id != caller_user_id {
            let role = self.review_repo.fetch_user_role(caller_user_id)?;
            if !role.is_moderator() {
                return Err(AppError::Forbidden(
                    json!({ "error": "Only the author or a moderator can delete this review" }),
                ));
            }
        }

        self.review_repo.delete_review(&review)?;

        Ok(self.review_presenter.to_http_res())
    }
}

pub struct CreateReviewUsecaseInput {
    pub author_id: Uuid,
    pub organisation_id: Option<Uuid>,
    pub person_id: Option<Uuid>,
    pub rating: i32,
    pub topic: Option<String>,
    pub text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::features::organisation::entities::Organisation;
    use crate::app::features::person::entities::{Person, PersonStatus};
    use crate::app::features::review::entities::Review;
    use crate::app::features::review::presenters::ReviewPresenterImpl;
    use crate::app::features::user::entities::UserRole;
    use std::sync::Mutex;

    /// Serves one org, one person and one review; records the writes so the
    /// tests can assert a rejected call never reached the repository.
    struct StubRepo {
        organisation: Organisation,
        person: Person,
        review: Review,
        caller_role: UserRole,
        created: Mutex<Vec<CreateReview>>,
        deleted: Mutex<Vec<Uuid>>,
    }

    impl ReviewRepository for StubRepo {
        fn create_review(&self, record: CreateReview) -> Result<Review, AppError> {
            let review = Review {
                id: Uuid::new_v4(),
                author_id: record.author_id,
                organisation_id: record.organisation_id,
                person_id: record.person_id,
                rating: record.rating,
                topic: record.topic.clone(),
                text: record.text.clone(),
                created_at: chrono::Utc::now().naive_utc(),
            };
            self.created.lock().unwrap().push(record);

            Ok(review)
        }

        fn list_reviews(
            &self,
            _target: ReviewTarget,
            _limit: i64,
            _offset: i64,
        ) -> Result<Vec<Review>, AppError> {
            Ok(vec![self.review.clone()])
        }

        fn fetch_review(&self, _id: Uuid) -> Result<Review, AppError> {
            Ok(self.review.clone())
        }

        fn delete_review(&self, review: &Review) -> Result<(), AppError> {
            self.deleted.lock().unwrap().push(review.id);

            Ok(())
        }

        fn fetch_target_organisation(&self, _id: Uuid) -> Result<Organisation, AppError> {
            Ok(self.organisation.clone())
        }

        fn fetch_target_person(&self, _id: Uuid) -> Result<Person, AppError> {
            Ok(self.person.clone())
        }

        fn fetch_user_role(&self, _user_id: Uuid) -> Result<UserRole, AppError> {
            Ok(self.caller_role)
        }
    }

    fn test_organisation(status: &str) -> Organisation {
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
            created_by: None,
            verified: false,
            status: status.to_string(),
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
            cost: None,
            google_place_id: None,
            google_rating: None,
            visits_count: 0,
            rating_avg: None,
            reviews_count: 0,
        }
    }

    fn test_person(status: PersonStatus, allow_reviews: bool) -> Person {
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
            status: status.as_str().to_string(),
            show_whatsapp: false,
            show_email: false,
            show_city: false,
            allow_reviews,
            recommended_by: None,
            claimed_by: None,
            moderation_note: None,
            rating_avg: None,
            reviews_count: 0,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }

    fn test_review(author_id: Uuid) -> Review {
        Review {
            id: Uuid::new_v4(),
            author_id,
            organisation_id: Some(Uuid::new_v4()),
            person_id: None,
            rating: 5,
            topic: None,
            text: None,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }

    fn usecase_with(repo: Arc<StubRepo>) -> ReviewUsecase {
        ReviewUsecase::new(repo, Arc::new(ReviewPresenterImpl::new()))
    }

    fn stub_repo(
        organisation: Organisation,
        person: Person,
        review: Review,
        caller_role: UserRole,
    ) -> Arc<StubRepo> {
        Arc::new(StubRepo {
            organisation,
            person,
            review,
            caller_role,
            created: Mutex::new(vec![]),
            deleted: Mutex::new(vec![]),
        })
    }

    fn create_input(
        organisation_id: Option<Uuid>,
        person_id: Option<Uuid>,
    ) -> CreateReviewUsecaseInput {
        CreateReviewUsecaseInput {
            author_id: Uuid::new_v4(),
            organisation_id,
            person_id,
            rating: 5,
            topic: None,
            text: None,
        }
    }

    #[test]
    fn test_create_review_for_live_organisation() {
        let repo = stub_repo(
            test_organisation("live"),
            test_person(PersonStatus::Confirmed, true),
            test_review(Uuid::new_v4()),
            UserRole::User,
        );

        assert!(
            usecase_with(repo.clone())
                .create_review(create_input(Some(Uuid::new_v4()), None))
                .is_ok()
        );
        assert_eq!(repo.created.lock().unwrap().len(), 1);
    }

    /// Pending/rejected orgs are invisible in list/search — reviewing them
    /// would leak their existence and skew the aggregates.
    #[test]
    fn test_create_review_rejects_non_live_organisation() {
        for status in ["pending", "rejected"] {
            let repo = stub_repo(
                test_organisation(status),
                test_person(PersonStatus::Confirmed, true),
                test_review(Uuid::new_v4()),
                UserRole::User,
            );

            match usecase_with(repo.clone()).create_review(create_input(Some(Uuid::new_v4()), None))
            {
                Err(AppError::UnprocessableEntity(_)) => (),
                other => panic!(
                    "expected UnprocessableEntity for {}, got {:?}",
                    status,
                    other.err()
                ),
            }
            assert!(repo.created.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn test_create_review_rejects_person_not_yet_public() {
        for status in [
            PersonStatus::Pending,
            PersonStatus::Awaiting,
            PersonStatus::Declined,
        ] {
            let repo = stub_repo(
                test_organisation("live"),
                test_person(status, true),
                test_review(Uuid::new_v4()),
                UserRole::User,
            );

            match usecase_with(repo.clone()).create_review(create_input(None, Some(Uuid::new_v4())))
            {
                Err(AppError::UnprocessableEntity(_)) => (),
                other => panic!(
                    "expected UnprocessableEntity for {:?}, got {:?}",
                    status,
                    other.err()
                ),
            }
            assert!(repo.created.lock().unwrap().is_empty());
        }
    }

    /// The `allow_reviews` privacy toggle is the person's own decision, so it
    /// is a 403 rather than a validation error.
    #[test]
    fn test_create_review_respects_allow_reviews_toggle() {
        let repo = stub_repo(
            test_organisation("live"),
            test_person(PersonStatus::Claimed, false),
            test_review(Uuid::new_v4()),
            UserRole::User,
        );

        match usecase_with(repo.clone()).create_review(create_input(None, Some(Uuid::new_v4()))) {
            Err(AppError::Forbidden(_)) => (),
            other => panic!("expected Forbidden, got {:?}", other.err()),
        }
        assert!(repo.created.lock().unwrap().is_empty());
    }

    /// The DB CHECK allows exactly one target; the usecase must catch both
    /// violations before the insert.
    #[test]
    fn test_create_review_requires_exactly_one_target() {
        for (org_id, person_id) in [(None, None), (Some(Uuid::new_v4()), Some(Uuid::new_v4()))] {
            let repo = stub_repo(
                test_organisation("live"),
                test_person(PersonStatus::Confirmed, true),
                test_review(Uuid::new_v4()),
                UserRole::User,
            );

            match usecase_with(repo.clone()).create_review(create_input(org_id, person_id)) {
                Err(AppError::UnprocessableEntity(_)) => (),
                other => panic!("expected UnprocessableEntity, got {:?}", other.err()),
            }
            assert!(repo.created.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn test_create_review_validates_the_rating_first() {
        let repo = stub_repo(
            test_organisation("live"),
            test_person(PersonStatus::Confirmed, true),
            test_review(Uuid::new_v4()),
            UserRole::User,
        );

        let mut params = create_input(Some(Uuid::new_v4()), None);
        params.rating = 6;

        assert!(usecase_with(repo.clone()).create_review(params).is_err());
        assert!(repo.created.lock().unwrap().is_empty());
    }

    #[test]
    fn test_list_reviews_requires_exactly_one_target() {
        let repo = stub_repo(
            test_organisation("live"),
            test_person(PersonStatus::Confirmed, true),
            test_review(Uuid::new_v4()),
            UserRole::User,
        );
        let usecase = usecase_with(repo);

        assert!(usecase.list_reviews(None, None, 20, 0).is_err());
        assert!(
            usecase
                .list_reviews(Some(Uuid::new_v4()), Some(Uuid::new_v4()), 20, 0)
                .is_err()
        );
        assert!(
            usecase
                .list_reviews(Some(Uuid::new_v4()), None, 20, 0)
                .is_ok()
        );
        assert!(
            usecase
                .list_reviews(None, Some(Uuid::new_v4()), 20, 0)
                .is_ok()
        );
    }

    #[test]
    fn test_author_can_delete_own_review() {
        let author = Uuid::new_v4();
        let review = test_review(author);
        let review_id = review.id;
        let repo = stub_repo(
            test_organisation("live"),
            test_person(PersonStatus::Confirmed, true),
            review,
            UserRole::User,
        );

        assert!(
            usecase_with(repo.clone())
                .delete_review(review_id, author)
                .is_ok()
        );
        assert_eq!(repo.deleted.lock().unwrap().as_slice(), &[review_id]);
    }

    #[test]
    fn test_moderator_and_admin_can_delete_any_review() {
        for role in [UserRole::Moderator, UserRole::Admin] {
            let review = test_review(Uuid::new_v4());
            let review_id = review.id;
            let repo = stub_repo(
                test_organisation("live"),
                test_person(PersonStatus::Confirmed, true),
                review,
                role,
            );

            assert!(
                usecase_with(repo.clone())
                    .delete_review(review_id, Uuid::new_v4())
                    .is_ok(),
                "{:?} must be able to delete any review",
                role
            );
            assert_eq!(repo.deleted.lock().unwrap().len(), 1);
        }
    }

    #[test]
    fn test_stranger_cannot_delete_someone_elses_review() {
        let review = test_review(Uuid::new_v4());
        let review_id = review.id;
        let repo = stub_repo(
            test_organisation("live"),
            test_person(PersonStatus::Confirmed, true),
            review,
            UserRole::User,
        );

        match usecase_with(repo.clone()).delete_review(review_id, Uuid::new_v4()) {
            Err(AppError::Forbidden(_)) => (),
            other => panic!("expected Forbidden, got {:?}", other.err()),
        }
        assert!(repo.deleted.lock().unwrap().is_empty());
    }
}
