use crate::app::features::organisation::entities::{Organisation, OrganisationStatus};
use crate::app::features::person::entities::{Person, PersonStatus};
use crate::app::features::user::entities::{User, UserRole};
use crate::data::schema::{moderation_events, organisations, people, reports};
use crate::{
    error::AppError,
    utils::{
        cache::{CacheKeys, CacheService, TypedCache},
        db::DbPool,
    },
};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Moderation target (TEXT + CHECK in moderation_events/reports).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Org,
    Person,
}

impl TargetKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TargetKind::Org => "org",
            TargetKind::Person => "person",
        }
    }
}

impl TryFrom<&str> for TargetKind {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "org" => Ok(TargetKind::Org),
            "person" => Ok(TargetKind::Person),
            other => Err(AppError::UnprocessableEntity(
                json!({ "error": format!("Unknown moderation kind: {}", other) }),
            )),
        }
    }
}

/// Moderator decision (TEXT + CHECK in moderation_events).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModerationAction {
    Approve,
    RequestChanges,
    Reject,
}

impl ModerationAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModerationAction::Approve => "approve",
            ModerationAction::RequestChanges => "request_changes",
            ModerationAction::Reject => "reject",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Queryable, Selectable, Clone)]
#[diesel(table_name = moderation_events)]
pub struct ModerationEvent {
    pub id: Uuid,
    pub target_kind: String,
    pub target_id: Uuid,
    pub moderator_id: Option<Uuid>,
    pub action: String,
    pub note: Option<String>,
    pub flags: Option<serde_json::Value>,
    pub created_at: NaiveDateTime,
}

/// One row of the moderation queue: a pending org or person with its latest
/// event (incl. submit-time auto-check flags) and open reports count.
#[derive(Debug, Clone)]
pub struct QueueItem {
    pub kind: TargetKind,
    pub target_id: Uuid,
    pub name: String,
    pub status: String,
    pub city: Option<String>,
    pub submitted_by: Option<Uuid>,
    pub created_at: NaiveDateTime,
    pub last_event: Option<ModerationEvent>,
    pub open_reports: i64,
}

pub struct SubmittedEventInput {
    pub target_kind: TargetKind,
    pub target_id: Uuid,
    pub flags: serde_json::Value,
}

pub trait ModerationRepository: Send + Sync + 'static {
    /// Records the auto-check `submitted` event at submission time.
    fn record_submitted(&self, input: SubmittedEventInput) -> Result<(), AppError>;

    fn fetch_queue(&self, kind: Option<TargetKind>) -> Result<Vec<QueueItem>, AppError>;

    /// Applies a moderator decision: flips target status, stores the note on
    /// the target, and appends an audit event — in one transaction.
    fn moderate(
        &self,
        kind: TargetKind,
        target_id: Uuid,
        action: ModerationAction,
        note: Option<String>,
        moderator_id: Uuid,
    ) -> Result<(), AppError>;

    fn fetch_user_role(&self, user_id: Uuid) -> Result<UserRole, AppError>;
}

#[derive(Clone)]
pub struct ModerationRepositoryImpl {
    pool: DbPool,
    cache_service: TypedCache<Arc<dyn CacheService>>,
}

impl ModerationRepositoryImpl {
    pub fn new(pool: DbPool, cache_service: TypedCache<Arc<dyn CacheService>>) -> Self {
        Self {
            pool,
            cache_service,
        }
    }

    fn open_reports_count(
        conn: &mut PgConnection,
        kind: TargetKind,
        target_id: Uuid,
    ) -> Result<i64, AppError> {
        let count = reports::table
            .filter(reports::target_kind.eq(kind.as_str()))
            .filter(reports::target_id.eq(target_id))
            .filter(reports::status.eq("open"))
            .count()
            .get_result::<i64>(conn)?;

        Ok(count)
    }

    fn last_event(
        conn: &mut PgConnection,
        kind: TargetKind,
        target_id: Uuid,
    ) -> Result<Option<ModerationEvent>, AppError> {
        let event = moderation_events::table
            .filter(moderation_events::target_kind.eq(kind.as_str()))
            .filter(moderation_events::target_id.eq(target_id))
            .order(moderation_events::created_at.desc())
            .first::<ModerationEvent>(conn)
            .optional()?;

        Ok(event)
    }

    fn insert_event(
        conn: &mut PgConnection,
        kind: TargetKind,
        target_id: Uuid,
        action: &str,
        note: Option<&String>,
        flags: Option<&serde_json::Value>,
        moderator_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        diesel::insert_into(moderation_events::table)
            .values((
                moderation_events::target_kind.eq(kind.as_str()),
                moderation_events::target_id.eq(target_id),
                moderation_events::moderator_id.eq(moderator_id),
                moderation_events::action.eq(action),
                moderation_events::note.eq(note),
                moderation_events::flags.eq(flags),
            ))
            .execute(conn)?;

        Ok(())
    }
}

impl ModerationRepository for ModerationRepositoryImpl {
    fn record_submitted(&self, input: SubmittedEventInput) -> Result<(), AppError> {
        let conn = &mut self.pool.get()?;

        Self::insert_event(
            conn,
            input.target_kind,
            input.target_id,
            "submitted",
            None,
            Some(&input.flags),
            None,
        )
    }

    fn fetch_queue(&self, kind: Option<TargetKind>) -> Result<Vec<QueueItem>, AppError> {
        let conn = &mut self.pool.get()?;
        let mut items: Vec<QueueItem> = Vec::new();

        if kind.is_none() || kind == Some(TargetKind::Org) {
            let orgs = organisations::table
                .filter(organisations::status.eq(OrganisationStatus::Pending.as_str()))
                .order(organisations::created_at.asc())
                .load::<Organisation>(conn)?;

            for org in orgs {
                items.push(QueueItem {
                    kind: TargetKind::Org,
                    target_id: org.id,
                    name: org.name,
                    status: org.status,
                    city: org.city,
                    submitted_by: org.created_by,
                    created_at: org.created_at,
                    last_event: Self::last_event(conn, TargetKind::Org, org.id)?,
                    open_reports: Self::open_reports_count(conn, TargetKind::Org, org.id)?,
                });
            }
        }

        if kind.is_none() || kind == Some(TargetKind::Person) {
            let persons = people::table
                .filter(people::status.eq(PersonStatus::Pending.as_str()))
                .order(people::created_at.asc())
                .load::<Person>(conn)?;

            for person in persons {
                items.push(QueueItem {
                    kind: TargetKind::Person,
                    target_id: person.id,
                    name: person.name,
                    status: person.status,
                    city: person.city,
                    submitted_by: person.recommended_by,
                    created_at: person.created_at,
                    last_event: Self::last_event(conn, TargetKind::Person, person.id)?,
                    open_reports: Self::open_reports_count(conn, TargetKind::Person, person.id)?,
                });
            }
        }

        items.sort_by_key(|item| item.created_at);

        Ok(items)
    }

    fn moderate(
        &self,
        kind: TargetKind,
        target_id: Uuid,
        action: ModerationAction,
        note: Option<String>,
        moderator_id: Uuid,
    ) -> Result<(), AppError> {
        let conn = &mut self.pool.get()?;

        conn.transaction::<(), AppError, _>(|conn| {
            match kind {
                TargetKind::Org => {
                    // Ensure the target exists before mutating.
                    let _ = organisations::table
                        .find(target_id)
                        .first::<Organisation>(conn)?;

                    let new_status = match action {
                        ModerationAction::Approve => Some(OrganisationStatus::Live),
                        ModerationAction::Reject => Some(OrganisationStatus::Rejected),
                        ModerationAction::RequestChanges => None, // stays pending
                    };

                    if let Some(status) = new_status {
                        diesel::update(organisations::table.find(target_id))
                            .set((
                                organisations::status.eq(status.as_str()),
                                organisations::moderation_note.eq(note.clone()),
                                organisations::updated_at.eq(chrono::Utc::now().naive_utc()),
                            ))
                            .execute(conn)?;
                    } else {
                        diesel::update(organisations::table.find(target_id))
                            .set((
                                organisations::moderation_note.eq(note.clone()),
                                organisations::updated_at.eq(chrono::Utc::now().naive_utc()),
                            ))
                            .execute(conn)?;
                    }
                }
                TargetKind::Person => {
                    let person = people::table.find(target_id).first::<Person>(conn)?;
                    let _ = person; // existence check

                    let new_status = match action {
                        ModerationAction::Approve => Some(PersonStatus::Awaiting),
                        ModerationAction::Reject => Some(PersonStatus::Declined),
                        ModerationAction::RequestChanges => None,
                    };

                    if let Some(status) = new_status {
                        diesel::update(people::table.find(target_id))
                            .set((
                                people::status.eq(status.as_str()),
                                people::moderation_note.eq(note.clone()),
                                people::updated_at.eq(chrono::Utc::now().naive_utc()),
                            ))
                            .execute(conn)?;
                    } else {
                        diesel::update(people::table.find(target_id))
                            .set((
                                people::moderation_note.eq(note.clone()),
                                people::updated_at.eq(chrono::Utc::now().naive_utc()),
                            ))
                            .execute(conn)?;
                    }
                }
            }

            Self::insert_event(
                conn,
                kind,
                target_id,
                action.as_str(),
                note.as_ref(),
                None,
                Some(moderator_id),
            )?;

            Ok(())
        })?;

        // Approved orgs become visible in list/search immediately.
        let _ = self
            .cache_service
            .invalidate_pattern(&CacheKeys::organisation_pattern());
        let _ = self
            .cache_service
            .invalidate_pattern(&CacheKeys::person_pattern());

        Ok(())
    }

    fn fetch_user_role(&self, user_id: Uuid) -> Result<UserRole, AppError> {
        let conn = &mut self.pool.get()?;

        User::fetch_role(conn, user_id)
    }
}
