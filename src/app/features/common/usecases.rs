use super::{
    presenters::CommonPresenter,
    repositories::{CommonRepository, GetAllCountriesRepositoryInput},
};
use crate::{
    app::features::common::repositories::CreateOrganisationTypeRepositoryInput, error::AppError,
};
use actix_web::HttpResponse;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct CommonUsecase {
    common_repo: Arc<dyn CommonRepository>,
    common_presenter: Arc<dyn CommonPresenter>,
}

impl CommonUsecase {
    pub fn new(
        common_repo: Arc<dyn CommonRepository>,
        common_presenter: Arc<dyn CommonPresenter>,
    ) -> Self {
        Self {
            common_repo,
            common_presenter,
        }
    }

    pub fn fetch_all_countries(
        &self,
        params: FetchAllCountriesUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        let countries = self
            .common_repo
            .get_all_countries(GetAllCountriesRepositoryInput {
                limit: params.limit,
                offset: params.offset,
                name: params.name,
            })?;
        let response = self.common_presenter.to_multi_country_json(countries);

        Ok(response)
    }

    pub fn fetch_country_detail(&self, id: uuid::Uuid) -> Result<HttpResponse, AppError> {
        let (country, by_type) = self.common_repo.get_country_detail(id)?;
        let response = self
            .common_presenter
            .to_country_detail_json(country, by_type);

        Ok(response)
    }

    pub fn fetch_organisation_types(&self) -> Result<HttpResponse, AppError> {
        let org_types = self.common_repo.get_all_organisation_types()?;
        let response = self
            .common_presenter
            .to_multi_organization_type_json(org_types);

        Ok(response)
    }

    pub fn create_organisation_type(
        &self,
        caller_user_id: Uuid,
        params: CreateOrganisationTypeUsecaseInput,
    ) -> Result<HttpResponse, AppError> {
        // Organisation types are a system reference table — admin only.
        let role = self.common_repo.fetch_user_role(caller_user_id)?;
        if !role.is_admin() {
            return Err(AppError::Forbidden(
                json!({ "error": "Only an admin can manage organisation types" }),
            ));
        }

        let org_type =
            self.common_repo
                .create_organisation_type(CreateOrganisationTypeRepositoryInput {
                    org_type: params.org_type,
                    color: params.color,
                    title: params.title,
                    slug: params.slug,
                })?;
        let response = self
            .common_presenter
            .to_single_organization_type_json(org_type);

        Ok(response)
    }
}

pub struct FetchAllCountriesUsecaseInput {
    pub limit: i64,
    pub offset: i64,
    pub name: Option<String>,
}

pub struct CreateOrganisationTypeUsecaseInput {
    pub org_type: String,
    pub color: String,
    pub title: String,
    pub slug: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::features::common::presenters::CommonPresenterImpl;
    use crate::app::features::common::repositories::GetCountryRepositoryInput;
    use crate::app::features::user::entities::UserRole;
    use crate::data::models::{Country, OrganisationType};
    use std::sync::Mutex;

    struct StubRepo {
        caller_role: UserRole,
        created: Mutex<Vec<String>>,
    }

    impl CommonRepository for StubRepo {
        fn get_all_countries(
            &self,
            _params: GetAllCountriesRepositoryInput,
        ) -> Result<Vec<Country>, AppError> {
            unimplemented!()
        }

        fn get_country(&self, _params: GetCountryRepositoryInput) -> Result<Country, AppError> {
            unimplemented!()
        }

        fn get_country_detail(&self, _id: Uuid) -> Result<(Country, Vec<(String, i64)>), AppError> {
            unimplemented!()
        }

        fn get_organisation_type(&self, _id: &Uuid) -> Result<OrganisationType, AppError> {
            unimplemented!()
        }

        fn get_all_organisation_types(&self) -> Result<Vec<OrganisationType>, AppError> {
            unimplemented!()
        }

        fn create_organisation_type(
            &self,
            params: CreateOrganisationTypeRepositoryInput,
        ) -> Result<OrganisationType, AppError> {
            self.created.lock().unwrap().push(params.org_type.clone());

            Ok(OrganisationType {
                id: Uuid::new_v4(),
                org_type: params.org_type,
                color: Some(params.color),
                title: Some(params.title),
                slug: params.slug,
            })
        }

        fn fetch_user_role(&self, _user_id: Uuid) -> Result<UserRole, AppError> {
            Ok(self.caller_role)
        }
    }

    fn usecase_with(caller_role: UserRole) -> (CommonUsecase, Arc<StubRepo>) {
        let repo = Arc::new(StubRepo {
            caller_role,
            created: Mutex::new(vec![]),
        });
        let usecase = CommonUsecase::new(repo.clone(), Arc::new(CommonPresenterImpl::new()));

        (usecase, repo)
    }

    fn create_input() -> CreateOrganisationTypeUsecaseInput {
        CreateOrganisationTypeUsecaseInput {
            org_type: "embassy".to_string(),
            color: "#123456".to_string(),
            title: "Embassy".to_string(),
            slug: Some("embassy".to_string()),
        }
    }

    /// Organisation types are a system reference table: only an admin may
    /// extend it.
    #[test]
    fn test_only_admin_can_create_an_organisation_type() {
        let (usecase, repo) = usecase_with(UserRole::Admin);

        assert!(
            usecase
                .create_organisation_type(Uuid::new_v4(), create_input())
                .is_ok()
        );
        assert_eq!(repo.created.lock().unwrap().len(), 1);
    }

    /// A moderator outranks a user but is still not an admin here.
    #[test]
    fn test_user_and_moderator_cannot_create_an_organisation_type() {
        for role in [UserRole::User, UserRole::Moderator] {
            let (usecase, repo) = usecase_with(role);

            match usecase.create_organisation_type(Uuid::new_v4(), create_input()) {
                Err(AppError::Forbidden(_)) => (),
                other => panic!("expected Forbidden for {:?}, got {:?}", role, other.err()),
            }
            assert!(
                repo.created.lock().unwrap().is_empty(),
                "{:?} must not reach the repository",
                role
            );
        }
    }
}
