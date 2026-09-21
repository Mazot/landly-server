use super::entities::{CreateReview, Review, ReviewTarget};
use crate::app::features::organisation::entities::Organisation;
use crate::app::features::person::entities::Person;
use crate::app::features::user::entities::{User, UserRole};
use crate::{
    error::AppError,
    utils::{
        cache::{CacheKeys, CacheService, TypedCache},
        db::DbPool,
    },
};
use std::sync::Arc;
use uuid::Uuid;

pub trait ReviewRepository: Send + Sync + 'static {
    fn create_review(&self, record: CreateReview) -> Result<Review, AppError>;

    fn list_reviews(
        &self,
        target: ReviewTarget,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Review>, AppError>;

    fn fetch_review(&self, id: Uuid) -> Result<Review, AppError>;

    fn delete_review(&self, review: &Review) -> Result<(), AppError>;

    /// Target existence/visibility checks for create.
    fn fetch_target_organisation(&self, id: Uuid) -> Result<Organisation, AppError>;
    fn fetch_target_person(&self, id: Uuid) -> Result<Person, AppError>;

    fn fetch_user_role(&self, user_id: Uuid) -> Result<UserRole, AppError>;
}

#[derive(Clone)]
pub struct ReviewRepositoryImpl {
    pool: DbPool,
    cache_service: TypedCache<Arc<dyn CacheService>>,
}

impl ReviewRepositoryImpl {
    pub fn new(pool: DbPool, cache_service: TypedCache<Arc<dyn CacheService>>) -> Self {
        Self {
            pool,
            cache_service,
        }
    }

    /// Aggregates live on the org/person rows — both caches go stale on write.
    fn invalidate_targets(&self) {
        let _ = self
            .cache_service
            .invalidate_pattern(&CacheKeys::organisation_pattern());
        let _ = self
            .cache_service
            .invalidate_pattern(&CacheKeys::person_pattern());
    }
}

impl ReviewRepository for ReviewRepositoryImpl {
    fn create_review(&self, record: CreateReview) -> Result<Review, AppError> {
        let conn = &mut self.pool.get()?;
        let review = Review::create(conn, &record)?;

        self.invalidate_targets();

        Ok(review)
    }

    fn list_reviews(
        &self,
        target: ReviewTarget,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Review>, AppError> {
        let conn = &mut self.pool.get()?;

        Review::list_for_target(conn, target, limit, offset)
    }

    fn fetch_review(&self, id: Uuid) -> Result<Review, AppError> {
        let conn = &mut self.pool.get()?;

        Review::fetch_by_id(conn, id)
    }

    fn delete_review(&self, review: &Review) -> Result<(), AppError> {
        let conn = &mut self.pool.get()?;
        Review::delete(conn, review)?;

        self.invalidate_targets();

        Ok(())
    }

    fn fetch_target_organisation(&self, id: Uuid) -> Result<Organisation, AppError> {
        let conn = &mut self.pool.get()?;

        Organisation::fetch_by_id(conn, id)
    }

    fn fetch_target_person(&self, id: Uuid) -> Result<Person, AppError> {
        let conn = &mut self.pool.get()?;

        Person::fetch_by_id(conn, id)
    }

    fn fetch_user_role(&self, user_id: Uuid) -> Result<UserRole, AppError> {
        let conn = &mut self.pool.get()?;

        User::fetch_role(conn, user_id)
    }
}
