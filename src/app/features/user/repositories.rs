use super::entities::{SignUpV2Input, UpdateUserProfile, User, UserToLanguage};
use crate::{error::AppError, utils::db::DbPool};
use serde_json::json;
use uuid::Uuid;

/// Computed profile stats for the account screen (design: account.jsx).
/// `people_recommended` / `people_helped` / `rating` come alive in phase 2
/// together with the person and review features.
#[derive(Debug, Clone, Default)]
pub struct ProfileStats {
    /// Organisations this user submitted (any moderation status).
    pub places_added: i64,
    /// Not computed yet — always 0.
    pub people_recommended: i64,
    /// Not computed yet — always 0.
    pub people_helped: i64,
    /// Not computed yet — always null.
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

        // Never link a provider identity to an existing account by email:
        // signup does not verify emails, so whoever registered the address
        // first (possibly an attacker pre-registering the victim's email)
        // would share the account. Linking needs an explicit, authenticated
        // flow instead.
        if User::find_by_email(conn, &email)?.is_some() {
            return Err(AppError::Forbidden(json!({
                "error": "An account with this email already exists. Sign in with your password."
            })));
        }

        let base_username = email.split('@').next().unwrap_or("user").to_string();
        let uniq_username = format!("{}-{}", base_username, &Uuid::new_v4().to_string()[..8]);
        let password_stub = Uuid::new_v4().to_string();

        let (user, token) = User::signup(conn, uniq_username, email.clone(), password_stub)?;

        User::create_user_provider(conn, user.id, &provider, &provider_user_id, &email)?;

        Ok((user, token))
    }
}

/// DB-backed tests of the Google OAuth account rules (no Google involved:
/// they start after the verified email + `sub` have been obtained).
#[cfg(test)]
mod db_tests {
    use super::{UserRepository, UserRepositoryImpl};
    use crate::data::schema::user_providers;
    use crate::error::AppError;
    use crate::test_support::{self as ts, require_db};
    use diesel::prelude::*;
    use serial_test::serial;

    #[test]
    #[serial(jwt_env)]
    fn test_oauth_creates_the_account_once_and_links_the_provider() {
        let pool = require_db!();
        ts::jwt_env();
        let repo = UserRepositoryImpl::new(pool.clone());
        let (email, sub) = (format!("g{}@example.test", ts::unique()), ts::unique());

        let (first, token) = repo
            .upsert_oauth_user("google".into(), sub.clone(), email.clone())
            .unwrap();
        assert_eq!(first.email, email);
        assert!(!token.is_empty());
        let (again, _) = repo
            .upsert_oauth_user("google".into(), sub.clone(), email)
            .unwrap();
        assert_eq!(
            again.id, first.id,
            "the same Google account signs in to the same user"
        );

        let links: i64 = user_providers::table
            .filter(user_providers::provider_user_id.eq(&sub))
            .count()
            .get_result(&mut pool.get().unwrap())
            .unwrap();
        assert_eq!(links, 1);
    }

    /// Audit #2: an attacker controlling a Google account with a victim's
    /// email must not take over the victim's password account.
    #[test]
    #[serial(jwt_env)]
    fn test_oauth_never_links_to_an_existing_account_by_email() {
        let pool = require_db!();
        ts::jwt_env();
        let existing = ts::user(&pool, "user");
        let repo = UserRepositoryImpl::new(pool.clone());

        match repo.upsert_oauth_user("google".into(), ts::unique(), existing.email.clone()) {
            Err(AppError::Forbidden(_)) => (),
            other => panic!("expected Forbidden, got {:?}", other.map(|(u, _)| u.id)),
        }
    }
}
