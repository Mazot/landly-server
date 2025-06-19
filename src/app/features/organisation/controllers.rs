use super::{
    requests::CreateOrganisationRequest,
    usecases::CreateOrganisationUsecaseInput,
};
use crate::app::drivers::middlewares::state::AppState;
use crate::error::AppError;
use actix_web::{web::{Data, Json, Path, Query}, HttpRequest, HttpResponse};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct OrganisationsListQueryParams {
    location_country_id: Option<Uuid>,
    organisation_type_id: Option<Uuid>,
    address: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

pub async fn list(
    state: Data<AppState>,
    query: Query<OrganisationsListQueryParams>
) -> Result<HttpResponse, AppError> {
    todo!()
}

pub async fn fetch(
    state: Data<AppState>,
    _req: HttpRequest,
    id: Path<Uuid>
) -> Result<HttpResponse, AppError> {
    todo!()
}

pub async fn create(
    state: Data<AppState>,
    _req: HttpRequest,
    form: Json<CreateOrganisationRequest>
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .organisation_usecase
        .create_organisation(
            CreateOrganisationUsecaseInput{
                name: form.name.clone(),
                tel: form.tel.clone(),
                email: form.email.clone(),
                address: form.address.clone(),
                description: form.description.clone(),
                location_country_id: form.location_country_id,
                organisation_type_id: form.organisation_type_id,
            }
        )
}
