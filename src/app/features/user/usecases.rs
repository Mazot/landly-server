use super::{
    presenters::{AuthUserContent, UserPresenter},
    repositories::UserRepository,
};
use crate::{
    app::features::user::entities::{HereAs, Locale, SignUpV2Input, UpdateUserProfile, User},
    error::AppError,
};
use actix_web::HttpResponse;
use std::sync::Arc;
use uuid::Uuid;

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

    /// Email + password login; returns the user with a fresh JWT.
    pub fn signin(&self, email: String, password: String) -> Result<HttpResponse, AppError> {
        let (user, token) = self.user_repository.signin(email, password)?;
        let response = self.user_presenter.to_single_json(user, token);

        Ok(response)
    }

    /// Signup v2: validates enum-like fields, then creates the user and an
    /// optional default corridor in one transaction.
    pub fn signup(&self, params: SignUpUsecaseInput) -> Result<HttpResponse, AppError> {
        // Validate enum-like fields before touching the database.
        if let Some(locale) = params.locale.as_deref() {
            Locale::try_from(locale)?;
        }
        if let Some(here_as) = params.here_as.as_deref() {
            HereAs::try_from(here_as)?;
        }

        let (user, token) = self.user_repository.signup(SignUpV2Input {
            username: params.username,
            email: params.email,
            password: params.password,
            name: params.name,
            locale: params.locale,
            here_as: params.here_as,
            home_country_id: params.home_country_id,
            avatar_color: params.avatar_color,
            corridor_from_country_id: params.corridor_from_country_id,
            corridor_to_country_id: params.corridor_to_country_id,
        })?;
        let response = self.user_presenter.to_single_json(user, token);

        Ok(response)
    }

    /// Account-screen payload: profile fields + computed stats.
    pub fn fetch_profile(&self, user_id: Uuid) -> Result<HttpResponse, AppError> {
        let (user, stats) = self.user_repository.fetch_profile(user_id)?;
        let response = self.user_presenter.to_profile_json(user, stats);

        Ok(response)
    }

    /// Partial profile update; locale/here_as are validated against their
    /// enums before hitting the DB CHECK constraints.
    pub fn update_profile(
        &self,
        user_id: Uuid,
        params: UpdateProfileUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        if let Some(locale) = params.locale.as_deref() {
            Locale::try_from(locale)?;
        }
        if let Some(here_as) = params.here_as.as_deref() {
            HereAs::try_from(here_as)?;
        }

        let (user, stats) = self.user_repository.update_profile(
            user_id,
            UpdateUserProfile {
                name: params.name,
                bio: params.bio,
                city: params.city,
                home_country_id: params.home_country_id,
                avatar_color: params.avatar_color,
                locale: params.locale,
                here_as: params.here_as,
                notification_settings: None,
                updated_at: Some(chrono::Utc::now().naive_utc()),
            },
        )?;
        let response = self.user_presenter.to_profile_json(user, stats);

        Ok(response)
    }

    /// Replaces the free-form notification settings JSON wholesale.
    pub fn update_notification_settings(
        &self,
        user_id: Uuid,
        notification_settings: serde_json::Value,
    ) -> Result<HttpResponse, AppError> {
        let (user, stats) = self.user_repository.update_profile(
            user_id,
            UpdateUserProfile {
                notification_settings: Some(notification_settings),
                updated_at: Some(chrono::Utc::now().naive_utc()),
                ..UpdateUserProfile::default()
            },
        )?;
        let response = self.user_presenter.to_profile_json(user, stats);

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
        provider_user_id: String,
    ) -> Result<HttpResponse, AppError> {
        let (user, token) = self.user_repository.upsert_oauth_user(
            "google".to_string(),
            provider_user_id,
            email,
        )?;
        let response = self.user_presenter.to_single_json(user, token);

        Ok(response)
    }

    pub fn oauth_google_upsert_content(
        &self,
        email: String,
        provider_user_id: String,
    ) -> Result<AuthUserContent, AppError> {
        let (user, token) = self.user_repository.upsert_oauth_user(
            "google".to_string(),
            provider_user_id,
            email,
        )?;
        Ok(AuthUserContent::from((user, token)))
    }

    pub fn find_auth_user(&self, _user_id: Uuid) -> Result<User, &str> {
        // let maybe_user = self.user_repository.find(user_id);
        // self.user_presenter.to_auth_middleware(maybe_user)
        todo!("Implement find_auth_user logic")
    }
}

pub struct SignUpUsecaseInput {
    pub username: String,
    pub email: String,
    pub password: String,
    pub name: Option<String>,
    pub locale: Option<String>,
    pub here_as: Option<String>,
    pub home_country_id: Option<Uuid>,
    pub avatar_color: Option<String>,
    pub corridor_from_country_id: Option<Uuid>,
    pub corridor_to_country_id: Option<Uuid>,
}

pub struct UpdateProfileUsecaseInput {
    pub name: Option<String>,
    pub bio: Option<String>,
    pub city: Option<String>,
    pub home_country_id: Option<Uuid>,
    pub avatar_color: Option<String>,
    pub locale: Option<String>,
    pub here_as: Option<String>,
}
