use super::{
    presenters::UserPresenter,
    repositories::UserRepository,
};
use crate::{app::features::user::entities::User, error::AppError};
use actix_web::HttpResponse;
use uuid::Uuid;
use std::sync::Arc;

#[derive(Clone)]
pub struct UserUsecase {
    user_repository: Arc<dyn UserRepository>,
    user_presenter: Arc<dyn UserPresenter>,
}

impl UserUsecase {
    pub fn new(
        user_repository: Arc<dyn UserRepository>,
        user_presenter: Arc<dyn UserPresenter>,
    ) -> Self {
        UserUsecase {
            user_repository,
            user_presenter,
        }
    }

    pub fn signin(
        &self,
        email: String,
        password: String,
    ) -> Result<HttpResponse, AppError> {
        let (user, token) = self.user_repository.signin(email, password)?;
        let response = self.user_presenter.to_single_json(user, token);

        Ok(response)
    }

    pub fn signup(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<HttpResponse, AppError> {
        let (user, token) = self.user_repository.signup(username, email, password)?;
        let response = self.user_presenter.to_single_json(user, token);

        Ok(response)
    }

    pub fn add_languages(
        &self,
        user_id: Uuid,
        languages_ids: Vec<Uuid>,
    ) -> Result<HttpResponse, AppError> {
        let languages = self.user_repository.add_languages(user_id, languages_ids)?;
        let response = self.user_presenter.to_lang_vec_json(languages);

        Ok(response)
    }

    pub fn delete_language(
        &self,
        user_id: Uuid,
        language_id: Uuid,
    ) -> Result<HttpResponse, AppError> {
        self.user_repository.delete_language(user_id, language_id)?;
        let response = self.user_presenter.to_http_res();

        Ok(response)
    }

    pub fn fetch_languages(&self, user_id: Uuid) -> Result<HttpResponse, AppError> {
        let languages = self.user_repository.fetch_languages(user_id)?;
        let response = self.user_presenter.to_lang_vec_json(languages);

        Ok(response)
    }

    pub fn oauth_google_upsert(
        &self,
        email: String,
        provider_user_id: String
    ) -> Result<HttpResponse, AppError> {
        let (user, token) = self.user_repository.upsert_oauth_user(
            "google".to_string(),
            provider_user_id,
            email
        )?;
        let response = self.user_presenter.to_single_json(user, token);

        Ok(response)
    }

    pub fn find_auth_user(&self, user_id: Uuid) -> Result<User, &str> {
        // let maybe_user = self.user_repository.find(user_id);
        // self.user_presenter.to_auth_middleware(maybe_user)
        todo!("Implement find_auth_user logic")
    }
}
