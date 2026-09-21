use super::entities::{CreateSavedItem, SavedItem, SavedKind};
use crate::{error::AppError, utils::db::DbPool};
use uuid::Uuid;

pub trait SavedRepository: Send + Sync + 'static {
    fn create_saved(&self, record: CreateSavedItem) -> Result<SavedItem, AppError>;
    fn delete_saved(&self, item_id: Uuid, user_id: Uuid) -> Result<(), AppError>;
    fn list_saved(
        &self,
        user_id: Uuid,
        kind: Option<SavedKind>,
    ) -> Result<Vec<SavedItem>, AppError>;
    fn counts_saved(&self, user_id: Uuid) -> Result<Vec<(String, i64)>, AppError>;
}

#[derive(Clone)]
pub struct SavedRepositoryImpl {
    pool: DbPool,
}

impl SavedRepositoryImpl {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

impl SavedRepository for SavedRepositoryImpl {
    fn create_saved(&self, record: CreateSavedItem) -> Result<SavedItem, AppError> {
        let conn = &mut self.pool.get()?;

        SavedItem::create(conn, &record)
    }

    fn delete_saved(&self, item_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        let conn = &mut self.pool.get()?;

        SavedItem::delete_owned(conn, item_id, user_id)
    }

    fn list_saved(
        &self,
        user_id: Uuid,
        kind: Option<SavedKind>,
    ) -> Result<Vec<SavedItem>, AppError> {
        let conn = &mut self.pool.get()?;

        SavedItem::list_by_user(conn, user_id, kind)
    }

    fn counts_saved(&self, user_id: Uuid) -> Result<Vec<(String, i64)>, AppError> {
        let conn = &mut self.pool.get()?;

        SavedItem::counts_by_kind(conn, user_id)
    }
}
