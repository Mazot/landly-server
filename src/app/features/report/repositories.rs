use super::entities::{Report, ReportTargetKind};
use crate::{error::AppError, utils::db::DbPool};
use uuid::Uuid;

pub trait ReportRepository: Send + Sync + 'static {
    fn create_report(
        &self,
        reporter_id: Uuid,
        target_kind: ReportTargetKind,
        target_id: Uuid,
        reason: &str,
    ) -> Result<Report, AppError>;
}

#[derive(Clone)]
pub struct ReportRepositoryImpl {
    pool: DbPool,
}

impl ReportRepositoryImpl {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

impl ReportRepository for ReportRepositoryImpl {
    fn create_report(
        &self,
        reporter_id: Uuid,
        target_kind: ReportTargetKind,
        target_id: Uuid,
        reason: &str,
    ) -> Result<Report, AppError> {
        let conn = &mut self.pool.get()?;

        Report::create(conn, reporter_id, target_kind, target_id, reason)
    }
}
