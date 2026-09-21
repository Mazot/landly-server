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

        // Corridor is all-or-nothing: a one-sided pair would silently create
        // the user without the corridor, losing the intent.
        match (
            params.corridor_from_country_id,
            params.corridor_to_country_id,
        ) {
            (None, None) => {}
            (Some(from), Some(to)) => {
                if from == to {
                    return Err(AppError::UnprocessableEntity(serde_json::json!({
                        "error": "Corridor countries must be different"
                    })));
                }
            }
            _ => {
                return Err(AppError::UnprocessableEntity(serde_json::json!({
                    "error": "Corridor requires both corridor_from_country_id and corridor_to_country_id"
                })));
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::features::user::entities::UserToLanguage;
    use crate::app::features::user::presenters::UserPresenterImpl;
    use crate::app::features::user::repositories::{ProfileStats, UserRepository};

    /// Validation-only stub: signup must reject bad input BEFORE any repo
    /// call, so every method here is unreachable.
    struct UnreachableRepo;

    impl UserRepository for UnreachableRepo {
        fn signin(&self, _e: String, _p: String) -> Result<(User, String), AppError> {
            unreachable!()
        }
        fn signup(&self, _i: SignUpV2Input) -> Result<(User, String), AppError> {
            unreachable!("validation must fail before the repository is called")
        }
        fn fetch_profile(&self, _u: Uuid) -> Result<(User, ProfileStats), AppError> {
            unreachable!()
        }
        fn update_profile(
            &self,
            _u: Uuid,
            _c: UpdateUserProfile,
        ) -> Result<(User, ProfileStats), AppError> {
            unreachable!()
        }
        fn add_languages(&self, _u: Uuid, _l: Vec<Uuid>) -> Result<Vec<UserToLanguage>, AppError> {
            unreachable!()
        }
        fn delete_language(&self, _u: Uuid, _l: Uuid) -> Result<(), AppError> {
            unreachable!()
        }
        fn fetch_languages(&self, _u: Uuid) -> Result<Vec<UserToLanguage>, AppError> {
            unreachable!()
        }
        fn upsert_oauth_user(
            &self,
            _p: String,
            _pid: String,
            _e: String,
        ) -> Result<(User, String), AppError> {
            unreachable!()
        }
    }

    fn usecase() -> UserUsecase {
        UserUsecase::new(
            Arc::new(UnreachableRepo),
            Arc::new(UserPresenterImpl::new()),
        )
    }

    fn signup_input() -> SignUpUsecaseInput {
        SignUpUsecaseInput {
            username: "u".to_string(),
            email: "u@example.com".to_string(),
            password: "secret123".to_string(),
            name: None,
            locale: None,
            here_as: None,
            home_country_id: None,
            avatar_color: None,
            corridor_from_country_id: None,
            corridor_to_country_id: None,
        }
    }

    fn expect_422(result: Result<actix_web::HttpResponse, AppError>) {
        match result {
            Err(AppError::UnprocessableEntity(_)) => (),
            other => panic!("expected UnprocessableEntity, got {:?}", other.err()),
        }
    }

    #[test]
    fn test_signup_rejects_one_sided_corridor() {
        let mut params = signup_input();
        params.corridor_from_country_id = Some(Uuid::new_v4());

        expect_422(usecase().signup(params));

        let mut params = signup_input();
        params.corridor_to_country_id = Some(Uuid::new_v4());

        expect_422(usecase().signup(params));
    }

    #[test]
    fn test_signup_rejects_same_country_corridor() {
        let country = Uuid::new_v4();
        let mut params = signup_input();
        params.corridor_from_country_id = Some(country);
        params.corridor_to_country_id = Some(country);

        expect_422(usecase().signup(params));
    }

    #[test]
    fn test_signup_rejects_bad_locale_and_here_as() {
        let mut params = signup_input();
        params.locale = Some("de".to_string());
        expect_422(usecase().signup(params));

        let mut params = signup_input();
        params.here_as = Some("tourist".to_string());
        expect_422(usecase().signup(params));
    }
}
