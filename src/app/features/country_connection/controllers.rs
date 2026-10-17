use super::{
    requests::{
        CountryConnectionsListQueryParams, CreateCountryConnectionRequest,
        UpdateCountryConnectionRequest,
    },
    usecases::{
        CreateCountryConnectionUsecaseInput, FetchCountryConnectionsUsecaseInput,
        UpdateCountryConnectionUsecaseInput,
    },
};
use crate::app::drivers::middlewares::state::AppState;
use crate::error::AppError;
use actix_web::{
    HttpMessage, HttpRequest, HttpResponse,
    web::{Data, Json, Path, Query},
};
use serde_json::json;
use uuid::Uuid;

/// Extracts the authenticated user id inserted by the auth middleware.
fn caller_user_id(req: &HttpRequest) -> Result<Uuid, AppError> {
    req.extensions()
        .get::<Uuid>()
        .copied()
        .ok_or_else(|| AppError::Unauthorized(json!({ "error": "Missing authenticated user" })))
}

// [authorship] Human-written (original codebase).
#[utoipa::path(
    get,
    path = "/country-connection/list",
    context_path = "/api",
    params(CountryConnectionsListQueryParams),
    responses(
        (status = 200, description = "Country connections list response", body = super::presenters::MultipleCountryConnectionsResponse),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "CountryConnection"
)]
pub async fn list(
    state: Data<AppState>,
    query: Query<CountryConnectionsListQueryParams>,
) -> Result<HttpResponse, AppError> {
    // Clamp to sane bounds: negative values reach Postgres as
    // `OFFSET -n` / `LIMIT -n` and blow up with a 500.
    let offset = query.offset.unwrap_or(0).clamp(0, 150);
    let limit = query.limit.unwrap_or(20).clamp(0, 100);

    state
        .di_container
        .country_connection_usecase
        .fetch_country_connections(FetchCountryConnectionsUsecaseInput {
            embassy_org_id: query.embassy_org_id,
            consulate_org_id: query.consulate_org_id,
            location_country_id: query.location_country_id,
            limit,
            offset,
        })
}

// [authorship] Human-written (original codebase).
#[utoipa::path(
    get,
    path = "/country-connection/fetch/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Country Connection ID to fetch")
    ),
    responses(
        (status = 200, description = "Country connections list response", body = super::presenters::CountryConnectionContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "CountryConnection"
)]
pub async fn fetch(
    state: Data<AppState>,
    _req: HttpRequest,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .country_connection_usecase
        .fetch_country_connection(id.into_inner())
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// admin-only RBAC gate.
#[utoipa::path(
    post,
    path = "/country-connection/create",
    context_path = "/api",
    request_body = CreateCountryConnectionRequest,
    responses(
        (status = 200, description = "Country connections list response", body = super::presenters::CountryConnectionContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "CountryConnection"
)]
pub async fn create(
    state: Data<AppState>,
    req: HttpRequest,
    form: Json<CreateCountryConnectionRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .country_connection_usecase
        .create_country_connection(
            caller,
            CreateCountryConnectionUsecaseInput {
                embassy_org_id: form.embassy_org_id,
                consulate_org_id: form.consulate_org_id,
                common_info: form.common_info.clone(),
                location_country_id: form.location_country_id,
            },
        )
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// admin-only RBAC gate.
#[utoipa::path(
    put,
    path = "/country-connection/update/{id}",
    context_path = "/api",
    request_body = UpdateCountryConnectionRequest,
    params(
        ("id" = Uuid, Path, description = "Country Connection ID to update")
    ),
    responses(
        (status = 200, description = "Country connections list response", body = super::presenters::CountryConnectionContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "CountryConnection"
)]
pub async fn update(
    state: Data<AppState>,
    req: HttpRequest,
    id: Path<Uuid>,
    form: Json<UpdateCountryConnectionRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .country_connection_usecase
        .update_country_connection(
            id.into_inner(),
            caller,
            UpdateCountryConnectionUsecaseInput {
                embassy_org_id: form.embassy_org_id,
                consulate_org_id: form.consulate_org_id,
                location_country_id: form.location_country_id,
                common_info: form.common_info.clone(),
            },
        )
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// admin-only RBAC gate.
#[utoipa::path(
    delete,
    path = "/country-connection/delete/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Country Connection ID to delete")
    ),
    responses(
        (status = 200, description = "Country connection deleted successfully"),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "CountryConnection"
)]
pub async fn delete(
    state: Data<AppState>,
    req: HttpRequest,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .country_connection_usecase
        .delete_country_connection(id.into_inner(), caller)
}

/// DB-backed HTTP tests (skipped without TEST_DATABASE_URL, see test_support).
#[cfg(test)]
mod http_tests {
    use crate::test_support::{self as ts, OrgSpec, call, require_db, test_app};
    use actix_web::http::{Method, StatusCode};
    use serde_json::json;
    use serial_test::serial;
    use uuid::Uuid;

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_country_connection_crud_is_admin_only() {
        let pool = require_db!();
        let app = test_app!(pool);
        let (admin, user) = (ts::user(&pool, "admin"), ts::user(&pool, "user"));
        let country = ts::country(&pool);
        let embassy = ts::org(&pool, OrgSpec::default());
        let consulate = ts::org(&pool, OrgSpec::default());
        let body = json!({ "embassy_org_id": embassy.id, "location_country_id": country.id, "common_info": "Appointments only" });

        let (status, _) = call(
            &app,
            Method::POST,
            "/api/country-connection/create",
            Some(&user.token),
            Some(body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, created) = call(
            &app,
            Method::POST,
            "/api/country-connection/create",
            Some(&admin.token),
            Some(body),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{created}");
        let id = created["id"].as_str().unwrap().to_string();

        let (status, list) = call(
            &app,
            Method::GET,
            &format!(
                "/api/country-connection/list?location_country_id={}",
                country.id
            ),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(list["total"], 1);
        let (_, by_embassy) = call(
            &app,
            Method::GET,
            &format!("/api/country-connection/list?embassy_org_id={}", embassy.id),
            None,
            None,
        )
        .await;
        assert_eq!(by_embassy["items"][0]["id"], json!(id));

        let update = format!("/api/country-connection/update/{id}");
        let (status, _) = call(
            &app,
            Method::PUT,
            &update,
            Some(&user.token),
            Some(json!({ "common_info": "x" })),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, updated) = call(
            &app,
            Method::PUT,
            &update,
            Some(&admin.token),
            Some(json!({ "consulate_org_id": consulate.id })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{updated}");
        assert_eq!(updated["consulateOrgId"], json!(consulate.id));
        assert_eq!(
            updated["commonInfo"], "Appointments only",
            "omitted fields are kept"
        );

        let (status, fetched) = call(
            &app,
            Method::GET,
            &format!("/api/country-connection/fetch/{id}"),
            None,
            None,
        )
        .await;
        assert_eq!(
            (status, fetched["embassyOrgId"].clone()),
            (StatusCode::OK, json!(embassy.id))
        );

        let delete = format!("/api/country-connection/delete/{id}");
        let (status, _) = call(&app, Method::DELETE, &delete, Some(&user.token), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, _) = call(&app, Method::DELETE, &delete, Some(&admin.token), None).await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = call(
            &app,
            Method::GET,
            &format!("/api/country-connection/fetch/{id}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = call(
            &app,
            Method::PUT,
            &format!("/api/country-connection/update/{}", Uuid::new_v4()),
            Some(&admin.token),
            Some(json!({ "common_info": "x" })),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
