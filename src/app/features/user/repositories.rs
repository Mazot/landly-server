use super::entities::{User, UserToLanguage};
use crate::{error::AppError, utils::db::DbPool};
use uuid::Uuid;

pub trait UserRepository: Send + Sync + 'static {
    fn signin(
        &self,
        email: String,
        password: String,
    ) -> Result<(User, String), AppError>;

    fn signup(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<(User, String), AppError>;

    fn add_languages(
        &self,
        user_id: Uuid,
        languages_ids: Vec<Uuid>,
    ) -> Result<Vec<UserToLanguage>, AppError>;

    fn delete_language(
        &self,
        user_id: Uuid,
        language_id: Uuid,
    ) -> Result<(), AppError>;

    fn fetch_languages(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserToLanguage>, AppError>;

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
    fn signin(
        &self,
        email: String,
        password: String,
    ) -> Result<(User, String), AppError> {
        let conn = &mut self.pool.get()?;

        User::signin(conn, email, password)
    }

    fn signup(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<(User, String), AppError> {
        let conn = &mut self.pool.get()?;

        User::signup(conn, username, email, password)
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

    fn delete_language(
        &self,
        user_id: Uuid,
        language_id: Uuid,
    ) -> Result<(), AppError> {
        let conn = &mut self.pool.get()?;
        User::delete_language(conn, user_id, language_id)?;

        Ok(())
    }

    fn fetch_languages(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserToLanguage>, AppError> {
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
