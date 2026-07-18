use crate::data::schema::{user_providers, users, users_to_languages};
use crate::error::*;
use crate::utils::{hash, token};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// RBAC role stored in `users.role` (TEXT + CHECK constraint).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRole {
    User,
    Moderator,
    Admin,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::User => "user",
            UserRole::Moderator => "moderator",
            UserRole::Admin => "admin",
        }
    }

    pub fn is_moderator(&self) -> bool {
        matches!(self, UserRole::Moderator | UserRole::Admin)
    }
}

impl TryFrom<&str> for UserRole {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "user" => Ok(UserRole::User),
            "moderator" => Ok(UserRole::Moderator),
            "admin" => Ok(UserRole::Admin),
            other => Err(AppError::UnprocessableEntity(
                json!({ "error": format!("Unknown role: {}", other) }),
            )),
        }
    }
}

/// Why the user is on Landly (`users.here_as`, TEXT + CHECK constraint).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HereAs {
    Newcomer,
    Helping,
    Exploring,
}

impl HereAs {
    pub fn as_str(&self) -> &'static str {
        match self {
            HereAs::Newcomer => "newcomer",
            HereAs::Helping => "helping",
            HereAs::Exploring => "exploring",
        }
    }
}

impl TryFrom<&str> for HereAs {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "newcomer" => Ok(HereAs::Newcomer),
            "helping" => Ok(HereAs::Helping),
            "exploring" => Ok(HereAs::Exploring),
            other => Err(AppError::UnprocessableEntity(
                json!({ "error": format!("Unknown here_as value: {}", other) }),
            )),
        }
    }
}

/// UI locale (`users.locale`, TEXT + CHECK constraint).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    Ru,
    Uk,
}

impl Locale {
    pub fn as_str(&self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::Ru => "ru",
            Locale::Uk => "uk",
        }
    }
}

impl TryFrom<&str> for Locale {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "en" => Ok(Locale::En),
            "ru" => Ok(Locale::Ru),
            "uk" => Ok(Locale::Uk),
            other => Err(AppError::UnprocessableEntity(
                json!({ "error": format!("Unsupported locale: {}", other) }),
            )),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = users)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub name: Option<String>,
    pub bio: Option<String>,
    pub city: Option<String>,
    pub home_country_id: Option<Uuid>,
    pub avatar_color: Option<String>,
    pub locale: String,
    pub here_as: Option<String>,
    pub role: String,
    pub notification_settings: Option<serde_json::Value>,
}

impl User {
    pub fn role_enum(&self) -> UserRole {
        UserRole::try_from(self.role.as_str()).unwrap_or(UserRole::User)
    }
}

impl User {
    pub fn signup(
        conn: &mut PgConnection,
        username: String,
        email: String,
        password: String,
    ) -> Result<(Self, String), AppError> {
        let password_hash = hash::hash_password(&password)?;

        let new_user = CreateUser {
            username,
            email,
            password_hash,
            name: None,
            locale: None,
            here_as: None,
            home_country_id: None,
            avatar_color: None,
        };

        let user = Self::create(conn, &new_user)?;
        let token = user.generate_token()?;

        Ok((user, token))
    }

    /// Signup v2 (design: signup-fast.jsx + signup-corridor.jsx): creates the
    /// user together with an optional default corridor in one transaction.
    pub fn signup_v2(
        conn: &mut PgConnection,
        input: SignUpV2Input,
    ) -> Result<(Self, String), AppError> {
        let password_hash = hash::hash_password(&input.password)?;

        let user = conn.transaction::<User, AppError, _>(|conn| {
            let user = Self::create(
                conn,
                &CreateUser {
                    username: input.username,
                    email: input.email,
                    password_hash,
                    name: input.name,
                    locale: input.locale,
                    here_as: input.here_as,
                    home_country_id: input.home_country_id,
                    avatar_color: input.avatar_color,
                },
            )?;

            if let (Some(from), Some(to)) =
                (input.corridor_from_country_id, input.corridor_to_country_id)
            {
                use crate::data::schema::corridors;

                diesel::insert_into(corridors::table)
                    .values((
                        corridors::user_id.eq(user.id),
                        corridors::from_country_id.eq(from),
                        corridors::to_country_id.eq(to),
                        corridors::is_default.eq(true),
                    ))
                    .execute(conn)?;
            }

            Ok(user)
        })?;

        let token = user.generate_token()?;

        Ok((user, token))
    }

    pub fn find_by_id(conn: &mut PgConnection, id: Uuid) -> Result<Self, AppError> {
        let user = users::table.find(id).first::<User>(conn)?;

        Ok(user)
    }

    pub fn update_profile(
        conn: &mut PgConnection,
        id: Uuid,
        record: &UpdateUserProfile,
    ) -> Result<Self, AppError> {
        let target = users::table.find(id);
        let user = diesel::update(target)
            .set(record)
            .get_result::<User>(conn)?;

        Ok(user)
    }

    /// Profile stat for the account screen: how many places this user added.
    pub fn count_created_organisations(
        conn: &mut PgConnection,
        user_id: Uuid,
    ) -> Result<i64, AppError> {
        use crate::data::schema::organisations;

        let count = organisations::table
            .filter(organisations::created_by.eq(user_id))
            .count()
            .get_result::<i64>(conn)?;

        Ok(count)
    }

    pub fn signin(
        conn: &mut PgConnection,
        email: String,
        password: String,
    ) -> Result<(Self, String), AppError> {
        let user_by_email = users::table
            .filter(users::email.eq(email))
            .first::<User>(conn)?;

        hash::verify(password, &user_by_email.password_hash)?;
        let token = user_by_email.generate_token()?;

        Ok((user_by_email, token))
    }

    pub fn add_languages(
        conn: &mut PgConnection,
        user_id: Uuid,
        languages_ids: Vec<Uuid>,
    ) -> Result<Vec<UserToLanguage>, AppError> {
        let res = diesel::insert_into(users_to_languages::table)
            .values(
                languages_ids
                    .into_iter()
                    .map(|lang_id| CreateUserToLanguage {
                        user_id,
                        language_id: lang_id,
                    })
                    .collect::<Vec<CreateUserToLanguage>>(),
            )
            .get_results::<UserToLanguage>(conn)?;

        Ok(res)
    }

    pub fn fetch_languages(
        conn: &mut PgConnection,
        user_id: Uuid,
    ) -> Result<Vec<UserToLanguage>, AppError> {
        let res = users_to_languages::table
            .filter(users_to_languages::user_id.eq(user_id))
            .load::<UserToLanguage>(conn)?;

        Ok(res)
    }

    pub fn delete_language(
        conn: &mut PgConnection,
        user_id: Uuid,
        language_id: Uuid,
    ) -> Result<(), AppError> {
        diesel::delete(users_to_languages::table)
            .filter(users_to_languages::user_id.eq(user_id))
            .filter(users_to_languages::language_id.eq(language_id))
            .execute(conn)?;

        Ok(())
    }

    pub fn change_user(
        conn: &mut PgConnection,
        id: Uuid,
        username: Option<String>,
        email: Option<String>,
        password: Option<String>,
    ) -> Result<Self, AppError> {
        let password_hash = if let Some(pass) = password {
            Some(hash::hash_password(&pass)?)
        } else {
            None
        };

        let update_user_record = UpdateUser {
            id,
            username,
            email,
            password_hash,
            updated_at: Some(chrono::Utc::now().naive_utc()),
        };
        let updated_user = Self::update(conn, &update_user_record)?;

        Ok(updated_user)
    }

    pub fn find_user_by_provider(
        conn: &mut PgConnection,
        provider: &str,
        provider_user_id: &str,
    ) -> Result<Option<User>, AppError> {
        let user_provider = user_providers::table
            .filter(user_providers::provider.eq(provider))
            .filter(user_providers::provider_user_id.eq(provider_user_id))
            .first::<UserProvider>(conn)
            .optional()?;

        if let Some(up) = user_provider {
            let user = users::table.find(up.user_id).first::<User>(conn)?;

            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    pub fn find_by_email(conn: &mut PgConnection, email: &str) -> Result<Option<User>, AppError> {
        let user = users::table
            .filter(users::email.eq(email))
            .first::<User>(conn)
            .optional()?;

        Ok(user)
    }

    pub fn create_user_provider(
        conn: &mut PgConnection,
        user_id: Uuid,
        provider: &str,
        provider_user_id: &str,
        email: &str,
    ) -> Result<(), AppError> {
        diesel::insert_into(user_providers::table)
            .values((
                user_providers::user_id.eq(user_id),
                user_providers::provider.eq(&provider),
                user_providers::provider_user_id.eq(&provider_user_id),
                user_providers::email.eq(&email),
            ))
            .on_conflict_do_nothing()
            .execute(conn)?;

        Ok(())
    }

    pub fn generate_token(&self) -> Result<String, AppError> {
        token::generate_token(self.id)
    }

    fn create(conn: &mut PgConnection, record: &CreateUser) -> Result<Self, AppError> {
        let result = diesel::insert_into(users::table)
            .values(record)
            .get_result::<User>(conn)?;

        Ok(result)
    }

    fn update(conn: &mut PgConnection, record: &UpdateUser) -> Result<Self, AppError> {
        let user = users::table.find(record.id);
        let result = diesel::update(user).set(record).get_result::<User>(conn)?;

        Ok(result)
    }
}

#[derive(Insertable, Clone)]
#[diesel(table_name = users)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub name: Option<String>,
    pub locale: Option<String>,
    pub here_as: Option<String>,
    pub home_country_id: Option<Uuid>,
    pub avatar_color: Option<String>,
}

/// Everything the account/corridor screens need to create a user in one go.
pub struct SignUpV2Input {
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

#[derive(AsChangeset, Clone)]
#[diesel(table_name = users)]
pub struct UpdateUser {
    pub id: Uuid,
    pub username: Option<String>,
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(AsChangeset, Clone, Default)]
#[diesel(table_name = users)]
pub struct UpdateUserProfile {
    pub name: Option<String>,
    pub bio: Option<String>,
    pub city: Option<String>,
    pub home_country_id: Option<Uuid>,
    pub avatar_color: Option<String>,
    pub locale: Option<String>,
    pub here_as: Option<String>,
    pub notification_settings: Option<serde_json::Value>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = users_to_languages)]
pub struct UserToLanguage {
    pub user_id: Uuid,
    pub language_id: Uuid,
}

#[derive(Insertable, Clone)]
#[diesel(table_name = users_to_languages)]
pub struct CreateUserToLanguage {
    pub user_id: Uuid,
    pub language_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = user_providers)]
pub struct UserProvider {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub email: Option<String>,
    pub created_at: Option<NaiveDateTime>,
}
