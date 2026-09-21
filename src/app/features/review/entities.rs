use crate::data::schema::{organisations, people, reviews};
use crate::error::*;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Polymorphic review target: exactly one of org/person (DB CHECK enforced).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewTarget {
    Organisation(Uuid),
    Person(Uuid),
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = reviews)]
pub struct Review {
    pub id: Uuid,
    pub author_id: Uuid,
    pub organisation_id: Option<Uuid>,
    pub person_id: Option<Uuid>,
    pub rating: i32,
    pub topic: Option<String>,
    pub text: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable, Clone)]
#[diesel(table_name = reviews)]
pub struct CreateReview {
    pub author_id: Uuid,
    pub organisation_id: Option<Uuid>,
    pub person_id: Option<Uuid>,
    pub rating: i32,
    pub topic: Option<String>,
    pub text: Option<String>,
}

impl Review {
    /// Inserts the review and refreshes the target's rating_avg /
    /// reviews_count in the same transaction.
    pub fn create(conn: &mut PgConnection, record: &CreateReview) -> Result<Self, AppError> {
        conn.transaction::<Review, AppError, _>(|conn| {
            let review = diesel::insert_into(reviews::table)
                .values(record)
                .get_result::<Review>(conn)?;

            Self::refresh_target_aggregates(conn, &review)?;

            Ok(review)
        })
    }

    /// Deletes the review and refreshes the target's aggregates atomically.
    pub fn delete(conn: &mut PgConnection, review: &Review) -> Result<(), AppError> {
        conn.transaction::<(), AppError, _>(|conn| {
            diesel::delete(reviews::table.find(review.id)).execute(conn)?;
            Self::refresh_target_aggregates(conn, review)?;

            Ok(())
        })
    }

    pub fn fetch_by_id(conn: &mut PgConnection, id: Uuid) -> Result<Self, AppError> {
        let review = reviews::table.find(id).first::<Review>(conn)?;

        Ok(review)
    }

    pub fn list_for_target(
        conn: &mut PgConnection,
        target: ReviewTarget,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, AppError> {
        let mut query = reviews::table.into_boxed();

        query = match target {
            ReviewTarget::Organisation(id) => query.filter(reviews::organisation_id.eq(id)),
            ReviewTarget::Person(id) => query.filter(reviews::person_id.eq(id)),
        };

        let result = query
            .order(reviews::created_at.desc())
            .limit(limit)
            .offset(offset)
            .load::<Review>(conn)?;

        Ok(result)
    }

    /// Recomputes AVG/COUNT from the reviews table — the single source of
    /// truth — instead of incrementally patching counters.
    fn refresh_target_aggregates(conn: &mut PgConnection, review: &Review) -> Result<(), AppError> {
        if let Some(org_id) = review.organisation_id {
            let (avg, count) = reviews::table
                .filter(reviews::organisation_id.eq(org_id))
                .select((
                    diesel::dsl::avg(reviews::rating),
                    diesel::dsl::count(reviews::id),
                ))
                .first::<(Option<bigdecimal::BigDecimal>, i64)>(conn)?;

            diesel::update(organisations::table.find(org_id))
                .set((
                    organisations::rating_avg
                        .eq(avg.and_then(|a| bigdecimal::ToPrimitive::to_f64(&a))),
                    organisations::reviews_count.eq(count),
                ))
                .execute(conn)?;
        }

        if let Some(person_id) = review.person_id {
            let (avg, count) = reviews::table
                .filter(reviews::person_id.eq(person_id))
                .select((
                    diesel::dsl::avg(reviews::rating),
                    diesel::dsl::count(reviews::id),
                ))
                .first::<(Option<bigdecimal::BigDecimal>, i64)>(conn)?;

            diesel::update(people::table.find(person_id))
                .set((
                    people::rating_avg.eq(avg.and_then(|a| bigdecimal::ToPrimitive::to_f64(&a))),
                    people::reviews_count.eq(count),
                ))
                .execute(conn)?;
        }

        Ok(())
    }
}

pub fn validate_rating(rating: i32) -> Result<(), AppError> {
    if !(1..=5).contains(&rating) {
        return Err(AppError::UnprocessableEntity(
            json!({ "error": "rating must be between 1 and 5" }),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_rating() {
        for ok in 1..=5 {
            assert!(validate_rating(ok).is_ok());
        }
        assert!(validate_rating(0).is_err());
        assert!(validate_rating(6).is_err());
        assert!(validate_rating(-1).is_err());
    }
}
