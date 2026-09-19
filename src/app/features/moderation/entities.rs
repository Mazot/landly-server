use crate::data::schema::moderation_events;
use crate::error::AppError;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_kind_round_trip() {
        for kind in [TargetKind::Org, TargetKind::Person] {
            assert_eq!(TargetKind::try_from(kind.as_str()).unwrap(), kind);
        }
    }

    /// The queue filter comes straight from a query param, so an unknown
    /// value must be a 422 rather than silently listing everything.
    #[test]
    fn test_target_kind_rejects_unknown() {
        match TargetKind::try_from("conversation") {
            Err(AppError::UnprocessableEntity(_)) => (),
            other => panic!("expected UnprocessableEntity, got {:?}", other),
        }
        assert!(TargetKind::try_from("").is_err());
        assert!(TargetKind::try_from("Org").is_err());
    }

    /// `as_str` values are persisted in moderation_events.action and are
    /// pinned by a CHECK constraint — they must not drift.
    #[test]
    fn test_moderation_action_as_str() {
        assert_eq!(ModerationAction::Approve.as_str(), "approve");
        assert_eq!(ModerationAction::RequestChanges.as_str(), "request_changes");
        assert_eq!(ModerationAction::Reject.as_str(), "reject");
    }
}
