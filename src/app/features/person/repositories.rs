use super::entities::{
    ClaimOutcome, CreatePerson, ListPeopleFilters, Person, PersonStatus, PersonVouch,
};
use crate::{
    error::AppError,
    utils::{
        cache::{CacheKeys, CacheService, TypedCache},
        db::DbPool,
    },
};
use std::sync::Arc;
use uuid::Uuid;

pub trait PersonRepository: Send + Sync + 'static {
    /// Creates person + languages + claim token; returns the raw token.
    fn create_person(
        &self,
        record: CreatePerson,
        language_ids: Vec<Uuid>,
    ) -> Result<(Person, String), AppError>;

    fn fetch_person(&self, id: Uuid) -> Result<(Person, i64, Vec<Uuid>), AppError>;

    fn list_people(&self, filters: ListPeopleFilters) -> Result<Vec<(Person, i64)>, AppError>;

    fn vouch_person(
        &self,
        person_id: Uuid,
        user_id: Uuid,
        note: Option<String>,
    ) -> Result<PersonVouch, AppError>;

    fn fetch_by_claim_token(&self, token: &str) -> Result<Person, AppError>;

    fn resolve_claim(&self, token: &str, outcome: ClaimOutcome) -> Result<Person, AppError>;

    /// Submit-time auto-check input: how many approved (public) people the
    /// recommender already has.
    fn count_public_people_recommended_by(&self, user_id: Uuid) -> Result<i64, AppError>;
}

#[derive(Clone)]
pub struct PersonRepositoryImpl {
    pool: DbPool,
    cache_service: TypedCache<Arc<dyn CacheService>>,
}

impl PersonRepositoryImpl {
    pub fn new(pool: DbPool, cache_service: TypedCache<Arc<dyn CacheService>>) -> Self {
        Self {
            pool,
            cache_service,
        }
    }

    fn invalidate(&self) {
        let _ = self
            .cache_service
            .invalidate_pattern(&CacheKeys::person_pattern());
    }
}

impl PersonRepository for PersonRepositoryImpl {
    fn create_person(
        &self,
        record: CreatePerson,
        language_ids: Vec<Uuid>,
    ) -> Result<(Person, String), AppError> {
        let conn = &mut self.pool.get()?;
        let result = Person::create_with_claim_token(conn, &record, &language_ids)?;

        self.invalidate();

        Ok(result)
    }

    fn fetch_person(&self, id: Uuid) -> Result<(Person, i64, Vec<Uuid>), AppError> {
        let conn = &mut self.pool.get()?;
        let person = Person::fetch_by_id(conn, id)?;
        let vouches = Person::vouch_count(conn, id)?;
        let language_ids = Person::languages_of(conn, id)?;

        Ok((person, vouches, language_ids))
    }

    fn list_people(&self, filters: ListPeopleFilters) -> Result<Vec<(Person, i64)>, AppError> {
        let conn = &mut self.pool.get()?;
        let people = Person::list_public(conn, &filters)?;

        people
            .into_iter()
            .map(|p| {
                let vouches = Person::vouch_count(conn, p.id)?;
                Ok((p, vouches))
            })
            .collect()
    }

    fn vouch_person(
        &self,
        person_id: Uuid,
        user_id: Uuid,
        note: Option<String>,
    ) -> Result<PersonVouch, AppError> {
        let conn = &mut self.pool.get()?;
        let vouch = Person::vouch(conn, person_id, user_id, note)?;

        self.invalidate();

        Ok(vouch)
    }

    fn fetch_by_claim_token(&self, token: &str) -> Result<Person, AppError> {
        let conn = &mut self.pool.get()?;
        let (person, _claim) = Person::find_by_claim_token(conn, token)?;

        Ok(person)
    }

    fn resolve_claim(&self, token: &str, outcome: ClaimOutcome) -> Result<Person, AppError> {
        let conn = &mut self.pool.get()?;
        let person = Person::resolve_claim(conn, token, outcome)?;

        self.invalidate();

        Ok(person)
    }

    fn count_public_people_recommended_by(&self, user_id: Uuid) -> Result<i64, AppError> {
        use crate::data::schema::people;
        use diesel::prelude::*;

        let conn = &mut self.pool.get()?;
        let count = people::table
            .filter(people::recommended_by.eq(user_id))
            .filter(people::status.eq_any(vec![
                PersonStatus::Awaiting.as_str(),
                PersonStatus::Confirmed.as_str(),
                PersonStatus::Claimed.as_str(),
            ]))
            .count()
            .get_result::<i64>(conn)?;

        Ok(count)
    }
}
