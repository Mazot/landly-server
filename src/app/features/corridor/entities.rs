use crate::data::schema::corridors;
use crate::error::*;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = corridors)]
pub struct Corridor {
    pub id: Uuid,
    pub user_id: Uuid,
    pub from_country_id: Uuid,
    pub to_country_id: Uuid,
    pub is_default: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable, Clone)]
#[diesel(table_name = corridors)]
pub struct CreateCorridor {
    pub user_id: Uuid,
    pub from_country_id: Uuid,
    pub to_country_id: Uuid,
    pub is_default: bool,
}

/// Live-organisation counters for the destination country of a corridor
/// (design: signup-corridor.jsx "🇷🇺 → 🇩🇪 has", map-corridor.jsx).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CorridorStats {
    pub corridor: Corridor,
    pub total_places: i64,
    pub new_this_week: i64,
    pub by_type: Vec<(String, i64)>,
}

impl Corridor {
    pub fn create(conn: &mut PgConnection, record: &CreateCorridor) -> Result<Self, AppError> {
        conn.transaction::<Corridor, AppError, _>(|conn| {
            if record.is_default {
                Self::clear_default(conn, record.user_id)?;
            }

            let corridor = diesel::insert_into(corridors::table)
                .values(record)
                .get_result::<Corridor>(conn)?;

            Ok(corridor)
        })
    }

    pub fn list_by_user(conn: &mut PgConnection, user_id: Uuid) -> Result<Vec<Self>, AppError> {
        let result = corridors::table
            .filter(corridors::user_id.eq(user_id))
            .order((corridors::is_default.desc(), corridors::created_at.asc()))
            .load::<Corridor>(conn)?;

        Ok(result)
    }

    pub fn fetch_owned(
        conn: &mut PgConnection,
        corridor_id: Uuid,
        user_id: Uuid,
    ) -> Result<Self, AppError> {
        let corridor = corridors::table.find(corridor_id).first::<Corridor>(conn)?;

        if corridor.user_id != user_id {
            return Err(AppError::Forbidden(
                json!({ "error": "Corridor belongs to another user" }),
            ));
        }

        Ok(corridor)
    }

    pub fn set_default(
        conn: &mut PgConnection,
        corridor_id: Uuid,
        user_id: Uuid,
    ) -> Result<Self, AppError> {
        conn.transaction::<Corridor, AppError, _>(|conn| {
            let corridor = Self::fetch_owned(conn, corridor_id, user_id)?;
            Self::clear_default(conn, user_id)?;

            let updated = diesel::update(corridors::table.find(corridor.id))
                .set(corridors::is_default.eq(true))
                .get_result::<Corridor>(conn)?;

            Ok(updated)
        })
    }

    pub fn delete(
        conn: &mut PgConnection,
        corridor_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), AppError> {
        let corridor = Self::fetch_owned(conn, corridor_id, user_id)?;
        diesel::delete(corridors::table.find(corridor.id)).execute(conn)?;

        Ok(())
    }

    fn clear_default(conn: &mut PgConnection, user_id: Uuid) -> Result<(), AppError> {
        diesel::update(corridors::table.filter(corridors::user_id.eq(user_id)))
            .set(corridors::is_default.eq(false))
            .execute(conn)?;

        Ok(())
    }

    /// Counts live organisations in the destination country, grouped by org
    /// type slug, plus how many appeared during the last 7 days.
    pub fn stats(conn: &mut PgConnection, corridor: Corridor) -> Result<CorridorStats, AppError> {
        use crate::data::schema::{organisation_types, organisations};

        let rows: Vec<(Option<String>, i64)> = organisations::table
            .filter(organisations::location_country_id.eq(corridor.to_country_id))
            .filter(organisations::status.eq("live"))
            .left_join(organisation_types::table)
            .group_by(organisation_types::slug)
            .select((
                organisation_types::slug.nullable(),
                diesel::dsl::count_star(),
            ))
            .load(conn)?;

        let week_ago = chrono::Utc::now().naive_utc() - chrono::Duration::days(7);
        let new_this_week = organisations::table
            .filter(organisations::location_country_id.eq(corridor.to_country_id))
            .filter(organisations::status.eq("live"))
            .filter(organisations::created_at.gt(week_ago))
            .count()
            .get_result::<i64>(conn)?;

        let mut total_places = 0;
        let by_type: Vec<(String, i64)> = rows
            .into_iter()
            .map(|(slug, count)| {
                total_places += count;
                (slug.unwrap_or_else(|| "other".to_string()), count)
            })
            .collect();

        Ok(CorridorStats {
            corridor,
            total_places,
            new_this_week,
            by_type,
        })
    }
}
