use crate::data::schema::reports;
use crate::error::*;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Reportable target (TEXT + CHECK). `conversation` is scaffolded for the
/// phase-3 messaging feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportTargetKind {
    Org,
    Person,
    Conversation,
}

impl ReportTargetKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReportTargetKind::Org => "org",
            ReportTargetKind::Person => "person",
            ReportTargetKind::Conversation => "conversation",
        }
    }
}

impl TryFrom<&str> for ReportTargetKind {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "org" => Ok(ReportTargetKind::Org),
            "person" => Ok(ReportTargetKind::Person),
            "conversation" => Ok(ReportTargetKind::Conversation),
            other => Err(AppError::UnprocessableEntity(
                json!({ "error": format!("Unknown report target kind: {}", other) }),
            )),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable, Selectable, Clone)]
#[diesel(table_name = reports)]
pub struct Report {
    pub id: Uuid,
    pub reporter_id: Option<Uuid>,
    pub target_kind: String,
    pub target_id: Uuid,
    pub reason: String,
    pub status: String,
    pub created_at: NaiveDateTime,
}

impl Report {
    pub fn create(
        conn: &mut PgConnection,
        reporter_id: Uuid,
        target_kind: ReportTargetKind,
        target_id: Uuid,
        reason: &str,
    ) -> Result<Self, AppError> {
        let report = diesel::insert_into(reports::table)
            .values((
                reports::reporter_id.eq(reporter_id),
                reports::target_kind.eq(target_kind.as_str()),
                reports::target_id.eq(target_id),
                reports::reason.eq(reason),
            ))
            .get_result::<Report>(conn)?;

        Ok(report)
    }
}
