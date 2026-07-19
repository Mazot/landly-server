use crate::data::schema::saved_items;
use crate::error::*;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Bookmark kind (TEXT + CHECK): polymorphic target without FK — cleanup is
/// the owning delete-usecase's job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavedKind {
    Org,
    Person,
    Country,
    Corridor,
}

impl SavedKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SavedKind::Org => "org",
            SavedKind::Person => "person",
            SavedKind::Country => "country",
            SavedKind::Corridor => "corridor",
        }
    }

    pub const ALL: [SavedKind; 4] = [
        SavedKind::Org,
        SavedKind::Person,
        SavedKind::Country,
        SavedKind::Corridor,
    ];
}

impl TryFrom<&str> for SavedKind {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "org" => Ok(SavedKind::Org),
            "person" => Ok(SavedKind::Person),
            "country" => Ok(SavedKind::Country),
            "corridor" => Ok(SavedKind::Corridor),
            other => Err(AppError::UnprocessableEntity(
                json!({ "error": format!("Unknown saved kind: {}", other) }),
            )),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = saved_items)]
pub struct SavedItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub kind: String,
    pub target_id: Uuid,
    pub note: Option<String>,
    pub list_name: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable, Clone)]
#[diesel(table_name = saved_items)]
pub struct CreateSavedItem {
    pub user_id: Uuid,
    pub kind: String,
    pub target_id: Uuid,
    pub note: Option<String>,
    pub list_name: Option<String>,
}

impl SavedItem {
    pub fn create(conn: &mut PgConnection, record: &CreateSavedItem) -> Result<Self, AppError> {
        let item = diesel::insert_into(saved_items::table)
            .values(record)
            .get_result::<SavedItem>(conn)?;

        Ok(item)
    }

    /// Delete is scoped to the owner: deleting someone else's bookmark is 404.
    pub fn delete_owned(
        conn: &mut PgConnection,
        item_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let deleted = diesel::delete(
            saved_items::table
                .find(item_id)
                .filter(saved_items::user_id.eq(user_id)),
        )
        .execute(conn)?;

        if deleted == 0 {
            return Err(AppError::NotFound(
                json!({ "error": "Saved item not found" }),
            ));
        }

        Ok(())
    }

    pub fn list_by_user(
        conn: &mut PgConnection,
        user_id: Uuid,
        kind: Option<SavedKind>,
    ) -> Result<Vec<Self>, AppError> {
        let mut query = saved_items::table
            .filter(saved_items::user_id.eq(user_id))
            .into_boxed();

        if let Some(kind) = kind {
            query = query.filter(saved_items::kind.eq(kind.as_str()));
        }

        let items = query
            .order(saved_items::created_at.desc())
            .load::<SavedItem>(conn)?;

        Ok(items)
    }

    /// Per-kind counters for the Saved tab badges.
    pub fn counts_by_kind(
        conn: &mut PgConnection,
        user_id: Uuid,
    ) -> Result<Vec<(String, i64)>, AppError> {
        let rows: Vec<(String, i64)> = saved_items::table
            .filter(saved_items::user_id.eq(user_id))
            .group_by(saved_items::kind)
            .select((saved_items::kind, diesel::dsl::count_star()))
            .load(conn)?;

        Ok(rows)
    }
}
