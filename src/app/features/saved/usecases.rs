use super::{
    entities::{CreateSavedItem, SavedKind},
    presenters::SavedPresenter,
    repositories::SavedRepository,
};
use crate::error::AppError;
use actix_web::HttpResponse;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct SavedUsecase {
    saved_repo: Arc<dyn SavedRepository>,
    saved_presenter: Arc<dyn SavedPresenter>,
}

impl SavedUsecase {
    pub fn new(
        saved_repo: Arc<dyn SavedRepository>,
        saved_presenter: Arc<dyn SavedPresenter>,
    ) -> Self {
        Self {
            saved_repo,
            saved_presenter,
        }
    }

    /// Saves a bookmark; the (user, kind, target) uniqueness lives in the DB.
    pub fn create_saved(&self, params: CreateSavedUsecaseInput) -> Result<HttpResponse, AppError> {
        let kind = SavedKind::try_from(params.kind.as_str())?;

        let item = self.saved_repo.create_saved(CreateSavedItem {
            user_id: params.user_id,
            kind: kind.as_str().to_string(),
            target_id: params.target_id,
            note: params.note,
            list_name: params.list_name,
        })?;

        Ok(self.saved_presenter.to_single_json(item))
    }

    pub fn delete_saved(&self, item_id: Uuid, user_id: Uuid) -> Result<HttpResponse, AppError> {
        self.saved_repo.delete_saved(item_id, user_id)?;

        Ok(self.saved_presenter.to_http_res())
    }

    pub fn list_saved(
        &self,
        user_id: Uuid,
        kind: Option<String>,
    ) -> Result<HttpResponse, AppError> {
        let kind = kind.as_deref().map(SavedKind::try_from).transpose()?;
        let items = self.saved_repo.list_saved(user_id, kind)?;

        Ok(self.saved_presenter.to_multi_json(items))
    }

    pub fn counts_saved(&self, user_id: Uuid) -> Result<HttpResponse, AppError> {
        let counts = self.saved_repo.counts_saved(user_id)?;

        Ok(self.saved_presenter.to_counts_json(counts))
    }
}

pub struct CreateSavedUsecaseInput {
    pub user_id: Uuid,
    pub kind: String,
    pub target_id: Uuid,
    pub note: Option<String>,
    pub list_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::features::saved::entities::SavedItem;
    use crate::app::features::saved::presenters::SavedPresenterImpl;
    use std::sync::Mutex;

    #[derive(Default)]
    struct StubRepo {
        created: Mutex<Vec<String>>,
        listed: Mutex<Vec<Option<SavedKind>>>,
    }

    impl SavedRepository for StubRepo {
        fn create_saved(&self, record: CreateSavedItem) -> Result<SavedItem, AppError> {
            self.created.lock().unwrap().push(record.kind.clone());

            Ok(SavedItem {
                id: Uuid::new_v4(),
                user_id: record.user_id,
                kind: record.kind,
                target_id: record.target_id,
                note: record.note,
                list_name: record.list_name,
                created_at: chrono::Utc::now().naive_utc(),
            })
        }

        fn delete_saved(&self, _item_id: Uuid, _user_id: Uuid) -> Result<(), AppError> {
            Ok(())
        }

        fn list_saved(
            &self,
            _user_id: Uuid,
            kind: Option<SavedKind>,
        ) -> Result<Vec<SavedItem>, AppError> {
            self.listed.lock().unwrap().push(kind);

            Ok(vec![])
        }

        fn counts_saved(&self, _user_id: Uuid) -> Result<Vec<(String, i64)>, AppError> {
            Ok(vec![("org".to_string(), 2)])
        }
    }

    fn usecase() -> (SavedUsecase, Arc<StubRepo>) {
        let repo = Arc::new(StubRepo::default());
        let usecase = SavedUsecase::new(repo.clone(), Arc::new(SavedPresenterImpl::new()));

        (usecase, repo)
    }

    fn create_input(kind: &str) -> CreateSavedUsecaseInput {
        CreateSavedUsecaseInput {
            user_id: Uuid::new_v4(),
            kind: kind.to_string(),
            target_id: Uuid::new_v4(),
            note: None,
            list_name: None,
        }
    }

    #[test]
    fn test_create_saved_accepts_every_known_kind() {
        for kind in SavedKind::ALL {
            let (usecase, repo) = usecase();

            assert!(usecase.create_saved(create_input(kind.as_str())).is_ok());
            assert_eq!(repo.created.lock().unwrap()[0], kind.as_str());
        }
    }

    #[test]
    fn test_create_saved_rejects_unknown_kind() {
        let (usecase, repo) = usecase();

        match usecase.create_saved(create_input("conversation")) {
            Err(AppError::UnprocessableEntity(_)) => (),
            other => panic!("expected UnprocessableEntity, got {:?}", other.err()),
        }
        assert!(repo.created.lock().unwrap().is_empty());
    }

    /// The kind reaching the repository is the canonical `as_str()` value, not
    /// whatever the client sent — the CHECK constraint depends on it.
    #[test]
    fn test_create_saved_normalises_the_kind() {
        let (usecase, repo) = usecase();

        usecase.create_saved(create_input("person")).unwrap();

        assert_eq!(repo.created.lock().unwrap()[0], SavedKind::Person.as_str());
    }

    #[test]
    fn test_list_saved_filter_is_optional_but_validated() {
        let (usecase, repo) = usecase();
        let user = Uuid::new_v4();

        assert!(usecase.list_saved(user, None).is_ok());
        assert!(
            usecase
                .list_saved(user, Some("country".to_string()))
                .is_ok()
        );
        assert!(
            usecase
                .list_saved(user, Some("nonsense".to_string()))
                .is_err()
        );

        assert_eq!(
            repo.listed.lock().unwrap().as_slice(),
            &[None, Some(SavedKind::Country)]
        );
    }

    #[test]
    fn test_counts_saved_is_not_filtered() {
        let (usecase, _) = usecase();

        assert!(usecase.counts_saved(Uuid::new_v4()).is_ok());
    }
}
