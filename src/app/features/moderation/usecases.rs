use super::{
    entities::{ModerationAction, TargetKind},
    presenters::ModerationPresenter,
    repositories::ModerationRepository,
};
use crate::error::AppError;
use actix_web::HttpResponse;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ModerationUsecase {
    moderation_repo: Arc<dyn ModerationRepository>,
    moderation_presenter: Arc<dyn ModerationPresenter>,
}

impl ModerationUsecase {
    pub fn new(
        moderation_repo: Arc<dyn ModerationRepository>,
        moderation_presenter: Arc<dyn ModerationPresenter>,
    ) -> Self {
        Self {
            moderation_repo,
            moderation_presenter,
        }
    }

    /// Every moderation endpoint requires the moderator (or admin) role.
    fn ensure_moderator(&self, caller_user_id: Uuid) -> Result<(), AppError> {
        let role = self.moderation_repo.fetch_user_role(caller_user_id)?;

        if role.is_moderator() {
            return Ok(());
        }

        Err(AppError::Forbidden(
            json!({ "error": "Moderator role required" }),
        ))
    }

    pub fn fetch_queue(
        &self,
        caller_user_id: Uuid,
        kind: Option<String>,
    ) -> Result<HttpResponse, AppError> {
        self.ensure_moderator(caller_user_id)?;

        let kind = kind.as_deref().map(TargetKind::try_from).transpose()?;
        let items = self.moderation_repo.fetch_queue(kind)?;

        Ok(self.moderation_presenter.to_queue_json(items))
    }

    pub fn approve(
        &self,
        caller_user_id: Uuid,
        kind: String,
        target_id: Uuid,
        note: Option<String>,
    ) -> Result<HttpResponse, AppError> {
        self.ensure_moderator(caller_user_id)?;
        let kind = TargetKind::try_from(kind.as_str())?;

        self.moderation_repo.moderate(
            kind,
            target_id,
            ModerationAction::Approve,
            note,
            caller_user_id,
        )?;

        Ok(self.moderation_presenter.to_http_res())
    }

    /// Request-changes keeps the item pending; the note to the author is
    /// mandatory — without it the author has nothing to act on.
    pub fn request_changes(
        &self,
        caller_user_id: Uuid,
        kind: String,
        target_id: Uuid,
        note: Option<String>,
    ) -> Result<HttpResponse, AppError> {
        self.ensure_moderator(caller_user_id)?;
        let kind = TargetKind::try_from(kind.as_str())?;

        let note = note.filter(|n| !n.trim().is_empty()).ok_or_else(|| {
            AppError::UnprocessableEntity(json!({
                "error": "A note for the author is required when requesting changes"
            }))
        })?;

        self.moderation_repo.moderate(
            kind,
            target_id,
            ModerationAction::RequestChanges,
            Some(note),
            caller_user_id,
        )?;

        Ok(self.moderation_presenter.to_http_res())
    }

    pub fn reject(
        &self,
        caller_user_id: Uuid,
        kind: String,
        target_id: Uuid,
        note: Option<String>,
    ) -> Result<HttpResponse, AppError> {
        self.ensure_moderator(caller_user_id)?;
        let kind = TargetKind::try_from(kind.as_str())?;

        self.moderation_repo.moderate(
            kind,
            target_id,
            ModerationAction::Reject,
            note,
            caller_user_id,
        )?;

        Ok(self.moderation_presenter.to_http_res())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::features::moderation::entities::QueueItem;
    use crate::app::features::moderation::presenters::ModerationPresenterImpl;
    use crate::app::features::moderation::repositories::SubmittedEventInput;
    use crate::app::features::user::entities::UserRole;
    use std::sync::Mutex;

    /// Records what reached the repository so the tests can assert that a
    /// rejected call never touched it.
    #[derive(Default)]
    struct StubRepo {
        caller_role: Option<UserRole>,
        moderated: Mutex<Vec<(TargetKind, ModerationAction, Option<String>)>>,
    }

    impl ModerationRepository for StubRepo {
        fn record_submitted(&self, _input: SubmittedEventInput) -> Result<(), AppError> {
            unimplemented!()
        }

        fn fetch_queue(&self, _kind: Option<TargetKind>) -> Result<Vec<QueueItem>, AppError> {
            Ok(vec![])
        }

        fn moderate(
            &self,
            kind: TargetKind,
            _target_id: Uuid,
            action: ModerationAction,
            note: Option<String>,
            _moderator_id: Uuid,
        ) -> Result<(), AppError> {
            self.moderated.lock().unwrap().push((kind, action, note));

            Ok(())
        }

        fn fetch_user_role(&self, _user_id: Uuid) -> Result<UserRole, AppError> {
            self.caller_role
                .ok_or_else(|| AppError::NotFound(json!({ "error": "no user" })))
        }
    }

    fn usecase_with(caller_role: UserRole) -> (ModerationUsecase, Arc<StubRepo>) {
        let repo = Arc::new(StubRepo {
            caller_role: Some(caller_role),
            ..Default::default()
        });

        let usecase =
            ModerationUsecase::new(repo.clone(), Arc::new(ModerationPresenterImpl::new()));

        (usecase, repo)
    }

    #[test]
    fn test_moderator_and_admin_pass_the_role_gate() {
        for role in [UserRole::Moderator, UserRole::Admin] {
            let (usecase, _) = usecase_with(role);

            assert!(
                usecase.ensure_moderator(Uuid::new_v4()).is_ok(),
                "{:?} must pass the moderation gate",
                role
            );
        }
    }

    #[test]
    fn test_plain_user_is_forbidden() {
        let (usecase, repo) = usecase_with(UserRole::User);

        match usecase.approve(Uuid::new_v4(), "org".to_string(), Uuid::new_v4(), None) {
            Err(AppError::Forbidden(_)) => (),
            other => panic!("expected Forbidden, got {:?}", other.err()),
        }
        assert!(
            repo.moderated.lock().unwrap().is_empty(),
            "a forbidden call must not reach the repository"
        );
    }

    #[test]
    fn test_every_endpoint_is_role_gated() {
        let (usecase, repo) = usecase_with(UserRole::User);
        let caller = Uuid::new_v4();
        let target = Uuid::new_v4();
        let note = Some("fix the address".to_string());

        assert!(usecase.fetch_queue(caller, None).is_err());
        assert!(
            usecase
                .approve(caller, "org".to_string(), target, None)
                .is_err()
        );
        assert!(
            usecase
                .request_changes(caller, "org".to_string(), target, note)
                .is_err()
        );
        assert!(
            usecase
                .reject(caller, "person".to_string(), target, None)
                .is_err()
        );
        assert!(repo.moderated.lock().unwrap().is_empty());
    }

    /// Without a note the author has nothing to act on, so request-changes
    /// must fail before anything is written.
    #[test]
    fn test_request_changes_requires_a_non_empty_note() {
        for note in [None, Some(String::new()), Some("   ".to_string())] {
            let (usecase, repo) = usecase_with(UserRole::Moderator);

            match usecase.request_changes(
                Uuid::new_v4(),
                "org".to_string(),
                Uuid::new_v4(),
                note.clone(),
            ) {
                Err(AppError::UnprocessableEntity(_)) => (),
                other => panic!(
                    "expected UnprocessableEntity for {:?}, got {:?}",
                    note,
                    other.err()
                ),
            }
            assert!(repo.moderated.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn test_request_changes_forwards_the_note() {
        let (usecase, repo) = usecase_with(UserRole::Moderator);

        usecase
            .request_changes(
                Uuid::new_v4(),
                "person".to_string(),
                Uuid::new_v4(),
                Some("add a phone number".to_string()),
            )
            .unwrap();

        let moderated = repo.moderated.lock().unwrap();
        assert_eq!(moderated.len(), 1);
        assert_eq!(moderated[0].0, TargetKind::Person);
        assert_eq!(moderated[0].1, ModerationAction::RequestChanges);
        assert_eq!(moderated[0].2.as_deref(), Some("add a phone number"));
    }

    #[test]
    fn test_approve_and_reject_forward_the_action() {
        let (usecase, repo) = usecase_with(UserRole::Moderator);
        let caller = Uuid::new_v4();

        usecase
            .approve(caller, "org".to_string(), Uuid::new_v4(), None)
            .unwrap();
        usecase
            .reject(caller, "person".to_string(), Uuid::new_v4(), None)
            .unwrap();

        let moderated = repo.moderated.lock().unwrap();
        assert_eq!(
            moderated[0],
            (TargetKind::Org, ModerationAction::Approve, None)
        );
        assert_eq!(
            moderated[1],
            (TargetKind::Person, ModerationAction::Reject, None)
        );
    }

    /// The kind arrives as a raw string from the request body / query.
    #[test]
    fn test_unknown_kind_is_rejected_after_the_role_check() {
        let (usecase, repo) = usecase_with(UserRole::Admin);

        match usecase.approve(
            Uuid::new_v4(),
            "conversation".to_string(),
            Uuid::new_v4(),
            None,
        ) {
            Err(AppError::UnprocessableEntity(_)) => (),
            other => panic!("expected UnprocessableEntity, got {:?}", other.err()),
        }
        assert!(
            usecase
                .fetch_queue(Uuid::new_v4(), Some("corridor".to_string()))
                .is_err()
        );
        assert!(repo.moderated.lock().unwrap().is_empty());
    }

    #[test]
    fn test_queue_accepts_a_known_kind_and_no_kind() {
        let (usecase, _) = usecase_with(UserRole::Moderator);

        assert!(usecase.fetch_queue(Uuid::new_v4(), None).is_ok());
        assert!(
            usecase
                .fetch_queue(Uuid::new_v4(), Some("person".to_string()))
                .is_ok()
        );
    }
}
