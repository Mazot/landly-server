use super::entities::{SignUpV2Input, UpdateUserProfile, User, UserToLanguage};
use crate::{error::AppError, utils::db::DbPool};
use uuid::Uuid;

/// Computed profile stats for the account screen (design: account.jsx).
/// `people_recommended` / `people_helped` / `rating` come alive in phase 2
/// together with the person and review features.
#[derive(Debug, Clone, Default)]
pub struct ProfileStats {
    pub places_added: i64,
    pub people_recommended: i64,
    pub people_helped: i64,
    pub rating: Option<f64>,
}

pub trait UserRepository: Send + Sync + 'static {
    fn signin(&self, email: String, password: String) -> Result<(User, String), AppError>;

    fn signup(&self, input: SignUpV2Input) -> Result<(User, String), AppError>;

    fn fetch_profile(&self, user_id: Uuid) -> Result<(User, ProfileStats), AppError>;

    fn update_profile(
        &self,
        user_id: Uuid,
        changes: UpdateUserProfile,
    ) -> Result<(User, ProfileStats), AppError>;

    fn add_languages(
        &self,
        user_id: Uuid,
        languages_ids: Vec<Uuid>,
    ) -> Result<Vec<UserToLanguage>, AppError>;

    fn delete_language(&self, user_id: Uuid, language_id: Uuid) -> Result<(), AppError>;

    fn fetch_languages(&self, user_id: Uuid) -> Result<Vec<UserToLanguage>, AppError>;

    fn upsert_oauth_user(
        &self,
        provider: String,
        provider_user_id: String,
        email: String,
    ) -> Result<(User, String), AppError>;
}

#[derive(Clone)]
pub struct UserRepositoryImpl {
    pool: DbPool,
}
impl UserRepositoryImpl {
    pub fn new(pool: DbPool) -> Self {
        UserRepositoryImpl { pool }
    }
}

impl UserRepository for UserRepositoryImpl {
    fn signin(&self, email: String, password: String) -> Result<(User, String), AppError> {
        let conn = &mut self.pool.get()?;

        User::signin(conn, email, password)
    }

    fn signup(&self, input: SignUpV2Input) -> Result<(User, String), AppError> {
        let conn = &mut self.pool.get()?;

        User::signup_v2(conn, input)
    }

    fn fetch_profile(&self, user_id: Uuid) -> Result<(User, ProfileStats), AppError> {
        let conn = &mut self.pool.get()?;
        let user = User::find_by_id(conn, user_id)?;
        let stats = ProfileStats {
            places_added: User::count_created_organisations(conn, user_id)?,
            ..ProfileStats::default()
        };

        Ok((user, stats))
    }

    fn update_profile(
        &self,
        user_id: Uuid,
        changes: UpdateUserProfile,
    ) -> Result<(User, ProfileStats), AppError> {
        let conn = &mut self.pool.get()?;
        let user = User::update_profile(conn, user_id, &changes)?;
        let stats = ProfileStats {
            places_added: User::count_created_organisations(conn, user_id)?,
            ..ProfileStats::default()
        };

        Ok((user, stats))
    }

    fn add_languages(
        &self,
        user_id: Uuid,
        languages_ids: Vec<Uuid>,
    ) -> Result<Vec<UserToLanguage>, AppError> {
        let conn = &mut self.pool.get()?;
        let res = User::add_languages(conn, user_id, languages_ids)?;

        Ok(res)
    }

    fn delete_language(&self, user_id: Uuid, language_id: Uuid) -> Result<(), AppError> {
        let conn = &mut self.pool.get()?;
        User::delete_language(conn, user_id, language_id)?;

        Ok(())
    }

    fn fetch_languages(&self, user_id: Uuid) -> Result<Vec<UserToLanguage>, AppError> {
        let conn = &mut self.pool.get()?;
        let res = User::fetch_languages(conn, user_id)?;

        Ok(res)
    }

    fn upsert_oauth_user(
        &self,
        provider: String,
        provider_user_id: String,
        email: String,
    ) -> Result<(User, String), AppError> {
        let conn = &mut self.pool.get()?;

        let user_opt = User::find_user_by_provider(conn, &provider, &provider_user_id)?;

        if let Some(user) = user_opt {
            let token = user.generate_token()?;
            return Ok((user, token));
        }

        let maybe_user = User::find_by_email(conn, &email)?;

        let (user, token) = if let Some(user) = maybe_user {
            let token = user.generate_token()?;
            (user, token)
        } else {
            // Создаем нового пользователя через signup
            let base_username = email.split('@').next().unwrap_or("user").to_string();
            let uniq_username = format!("{}-{}", base_username, &Uuid::new_v4().to_string()[..8]);
            let password_stub = Uuid::new_v4().to_string();

            User::signup(conn, uniq_username, email.clone(), password_stub)?
        };

        User::create_user_provider(conn, user.id, &provider, &provider_user_id, &email)?;

        Ok((user, token))
    }
}
