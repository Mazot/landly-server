use crate::data::schema::{users, users_to_languages};
use crate::utils::hash;
use crate::error::*;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = users)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
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
        };

        let user = Self::create(conn, &new_user)?;
        let token = user.generate_token()?;

        Ok((user, token))
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
                languages_ids.into_iter().map(|lang_id| {
                    CreateUserToLanguage {
                        id: user_id,
                        language_id: lang_id,
                    }
                }).collect::<Vec<CreateUserToLanguage>>()
            )
            .get_results::<UserToLanguage>(conn)?;

        Ok(res)
    }

    pub fn fetch_languages(
        conn: &mut PgConnection,
        user_id: Uuid,
    ) -> Result<Vec<UserToLanguage>, AppError> {
        let res = users_to_languages::table
            .filter(users_to_languages::id.eq(user_id))
            .load::<UserToLanguage>(conn)?;

        Ok(res)
    }

    pub fn delete_language(
        conn: &mut PgConnection,
        user_id: Uuid,
        language_id: Uuid,
    ) -> Result<(), AppError> {
        diesel::delete(users_to_languages::table)
            .filter(users_to_languages::id.eq(user_id))
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

    fn create(
        conn: &mut PgConnection,
        record: &CreateUser,
    ) -> Result<Self, AppError> {
        let result = diesel::insert_into(users::table)
            .values(record)
            .get_result::<User>(conn)?;

        Ok(result)
    }

    fn generate_token(&self) -> Result<String, AppError> {
        todo!("Implement token generation logic")
    }

    fn update(
        conn: &mut PgConnection,
        record: &UpdateUser,
    ) -> Result<Self, AppError> {
        let user = users::table.find(record.id);
        let result = diesel::update(user)
            .set(record)
            .get_result::<User>(conn)?;

        Ok(result)
    }
}

#[derive(Insertable, Clone)]
#[diesel(table_name = users)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
    pub password_hash: String,
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

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = users_to_languages)]
pub struct UserToLanguage {
    pub id: Uuid,
    pub language_id: Uuid,
}

#[derive(Insertable, Clone)]
#[diesel(table_name = users_to_languages)]
pub struct CreateUserToLanguage {
    pub id: Uuid,
    pub language_id: Uuid,
}
