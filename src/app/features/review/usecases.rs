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
