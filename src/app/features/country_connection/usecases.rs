use super::{
    presenters::CountryConnectionPresenter,
    repositories::{
        CountryConnectionRepository, CreateCountryConnectionRepositoryInput,
        FetchCountryConnectionsRepositoryInput, UpdateCountryConnectionRepositoryInput,
    },
};
use crate::error::AppError;
use actix_web::HttpResponse;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct CountryConnectionUsecase {
    country_connection_repo: Arc<dyn CountryConnectionRepository>,
    country_connection_presenter: Arc<dyn CountryConnectionPresenter>,
}

impl CountryConnectionUsecase {
    pub fn new(
        country_connection_repo: Arc<dyn CountryConnectionRepository>,
        country_connection_presenter: Arc<dyn CountryConnectionPresenter>,
    ) -> Self {
        Self {
            country_connection_repo,
            country_connection_presenter,
        }
    }

    /// Country connections are a system reference table — admin only.
    fn ensure_admin(&self, caller_user_id: Uuid) -> Result<(), AppError> {
        let role = self
            .country_connection_repo
            .fetch_user_role(caller_user_id)?;

        if role.is_admin() {
            return Ok(());
        }

        Err(AppError::Forbidden(
            json!({ "error": "Only an admin can manage country connections" }),
        ))
    }

    pub fn fetch_country_connection(&self, id: Uuid) -> Result<HttpResponse, AppError> {
        let country_connection = self.country_connection_repo.fetch_country_connection(id)?;
        let response = self
            .country_connection_presenter
            .to_single_json(country_connection);

        Ok(response)
    }

    pub fn fetch_country_connections(
        &self,
        params: FetchCountryConnectionsUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        let country_connections = self.country_connection_repo.fetch_country_connections(
            FetchCountryConnectionsRepositoryInput {
                embassy_org_id: params.embassy_org_id,
                consulate_org_id: params.consulate_org_id,
                location_country_id: params.location_country_id,
                limit: params.limit,
                offset: params.offset,
            },
        )?;
        let response = self
            .country_connection_presenter
            .to_multi_json(country_connections);

        Ok(response)
    }

    pub fn create_country_connection(
        &self,
        caller_user_id: Uuid,
        params: CreateCountryConnectionUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        self.ensure_admin(caller_user_id)?;

        let new_country_connection = self.country_connection_repo.create_country_connection(
            CreateCountryConnectionRepositoryInput {
                embassy_org_id: params.embassy_org_id,
                consulate_org_id: params.consulate_org_id,
                common_info: params.common_info,
                location_country_id: params.location_country_id,
            },
        )?;
        let response = self
            .country_connection_presenter
            .to_single_json(new_country_connection);

        Ok(response)
    }

    pub fn update_country_connection(
        &self,
        id: Uuid,
        caller_user_id: Uuid,
        params: UpdateCountryConnectionUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        self.ensure_admin(caller_user_id)?;

        let updated_country_connection = self.country_connection_repo.update_country_connection(
            id,
            UpdateCountryConnectionRepositoryInput {
                embassy_org_id: params.embassy_org_id,
                consulate_org_id: params.consulate_org_id,
                common_info: params.common_info,
                location_country_id: params.location_country_id,
            },
        )?;
        let response = self
            .country_connection_presenter
            .to_single_json(updated_country_connection);

        Ok(response)
    }

    pub fn delete_country_connection(
        &self,
        id: Uuid,
        caller_user_id: Uuid,
    ) -> Result<HttpResponse, AppError> {
        self.ensure_admin(caller_user_id)?;

        self.country_connection_repo.delete_country_connection(id)?;
        let response = self.country_connection_presenter.to_http_res();

        Ok(response)
    }
}

pub struct FetchCountryConnectionsUsecaseInput {
    pub embassy_org_id: Option<Uuid>,
    pub consulate_org_id: Option<Uuid>,
    pub location_country_id: Option<Uuid>,
    pub limit: i64,
    pub offset: i64,
}

pub struct CreateCountryConnectionUsecaseInput {
    pub embassy_org_id: Option<Uuid>,
    pub consulate_org_id: Option<Uuid>,
    pub common_info: Option<String>,
    pub location_country_id: Option<Uuid>,
}

pub struct UpdateCountryConnectionUsecaseInput {
    pub embassy_org_id: Option<Uuid>,
    pub consulate_org_id: Option<Uuid>,
    pub common_info: Option<String>,
    pub location_country_id: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::features::country_connection::entities::CountryConnection;
    use crate::app::features::country_connection::presenters::CountryConnectionPresenterImpl;
    use crate::app::features::user::entities::UserRole;
    use std::sync::Mutex;

    struct StubRepo {
        caller_role: UserRole,
        writes: Mutex<Vec<&'static str>>,
    }

    impl StubRepo {
        fn connection() -> CountryConnection {
            CountryConnection {
                id: Uuid::new_v4(),
                embassy_org_id: None,
                consulate_org_id: None,
                location_country_id: None,
                common_info: None,
            }
        }
    }

    impl CountryConnectionRepository for StubRepo {
        fn fetch_country_connections(
            &self,
            _params: FetchCountryConnectionsRepositoryInput,
        ) -> Result<Vec<CountryConnection>, AppError> {
            Ok(vec![])
        }

        fn fetch_country_connection(&self, _id: Uuid) -> Result<CountryConnection, AppError> {
            Ok(Self::connection())
        }

        fn create_country_connection(
            &self,
            _params: CreateCountryConnectionRepositoryInput,
        ) -> Result<CountryConnection, AppError> {
            self.writes.lock().unwrap().push("create");

            Ok(Self::connection())
        }

        fn update_country_connection(
            &self,
            _id: Uuid,
            _params: UpdateCountryConnectionRepositoryInput,
        ) -> Result<CountryConnection, AppError> {
            self.writes.lock().unwrap().push("update");

            Ok(Self::connection())
        }

        fn delete_country_connection(&self, _id: Uuid) -> Result<(), AppError> {
            self.writes.lock().unwrap().push("delete");

            Ok(())
        }

        fn fetch_user_role(&self, _user_id: Uuid) -> Result<UserRole, AppError> {
            Ok(self.caller_role)
        }
    }

    fn usecase_with(caller_role: UserRole) -> (CountryConnectionUsecase, Arc<StubRepo>) {
        let repo = Arc::new(StubRepo {
            caller_role,
            writes: Mutex::new(vec![]),
        });
        let usecase = CountryConnectionUsecase::new(
            repo.clone(),
            Arc::new(CountryConnectionPresenterImpl::new()),
        );

        (usecase, repo)
    }

    fn create_input() -> CreateCountryConnectionUsecaseInput {
        CreateCountryConnectionUsecaseInput {
            embassy_org_id: None,
            consulate_org_id: None,
            common_info: None,
            location_country_id: None,
        }
    }

    fn update_input() -> UpdateCountryConnectionUsecaseInput {
        UpdateCountryConnectionUsecaseInput {
            embassy_org_id: None,
            consulate_org_id: None,
            common_info: None,
            location_country_id: None,
        }
    }

    #[test]
    fn test_admin_may_mutate_country_connections() {
        let (usecase, repo) = usecase_with(UserRole::Admin);
        let caller = Uuid::new_v4();

        assert!(
            usecase
                .create_country_connection(caller, create_input())
                .is_ok()
        );
        assert!(
            usecase
                .update_country_connection(Uuid::new_v4(), caller, update_input())
                .is_ok()
        );
        assert!(
            usecase
                .delete_country_connection(Uuid::new_v4(), caller)
                .is_ok()
        );

        assert_eq!(
            repo.writes.lock().unwrap().as_slice(),
            &["create", "update", "delete"]
        );
    }

    /// Country connections are a system reference table — a moderator is not
    /// enough, and no write may reach the repository.
    #[test]
    fn test_non_admin_cannot_mutate_country_connections() {
        for role in [UserRole::User, UserRole::Moderator] {
            let (usecase, repo) = usecase_with(role);
            let caller = Uuid::new_v4();

            match usecase.create_country_connection(caller, create_input()) {
                Err(AppError::Forbidden(_)) => (),
                other => panic!("expected Forbidden for {:?}, got {:?}", role, other.err()),
            }
            assert!(
                usecase
                    .update_country_connection(Uuid::new_v4(), caller, update_input())
                    .is_err()
            );
            assert!(
                usecase
                    .delete_country_connection(Uuid::new_v4(), caller)
                    .is_err()
            );
            assert!(repo.writes.lock().unwrap().is_empty());
        }
    }

    /// Reads stay public for everyone, including anonymous-equivalent callers.
    #[test]
    fn test_reads_are_not_role_gated() {
        let (usecase, _) = usecase_with(UserRole::User);

        assert!(usecase.fetch_country_connection(Uuid::new_v4()).is_ok());
        assert!(
            usecase
                .fetch_country_connections(FetchCountryConnectionsUsecaseInput {
                    embassy_org_id: None,
                    consulate_org_id: None,
                    location_country_id: None,
                    limit: 20,
                    offset: 0,
                })
                .is_ok()
        );
    }
}
