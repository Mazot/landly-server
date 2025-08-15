use core::str;

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
}
