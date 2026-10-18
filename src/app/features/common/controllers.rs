use super::usecases::{FetchAllCountriesUsecaseInput, FetchAllLanguagesUsecaseInput};
use crate::app::{
    drivers::middlewares::state::AppState,
    features::common::usecases::CreateOrganisationTypeUsecaseInput,
};
use crate::error::AppError;
use actix_web::{
    HttpMessage, HttpRequest, HttpResponse,
    web::{Data, Json, Path, Query},
};
use serde::Deserialize;
use serde_json::json;
use utoipa::{IntoParams, ToSchema};
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
    path = "/common/countries",
    context_path = "/api",
    responses(
        (status = 200, description = "Countries list response", body = Vec<super::presenters::CountryContent>),
    ),
    params(CountriesListQueryParams),
    tag = "Common"
)]
pub async fn fetch_all_countries(
    state: Data<AppState>,
    params: Query<CountriesListQueryParams>,
) -> Result<HttpResponse, AppError> {
    // Clamp to sane bounds: negative values reach Postgres as
    // `OFFSET -n` / `LIMIT -n` and blow up with a 500.
    let offset = params.offset.unwrap_or(0).clamp(0, 150);
    let limit = params.limit.unwrap_or(20).clamp(0, 250);
    let name = params.name.clone();

    state
        .di_container
        .common_usecase
        .fetch_all_countries(FetchAllCountriesUsecaseInput {
            limit,
            offset,
            name,
        })
}

// [authorship] AI-generated (Claude).
#[utoipa::path(
    get,
    path = "/common/countries/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Country ID")
    ),
    responses(
        (status = 200, description = "Country with live-organisation breakdown by type", body = super::presenters::CountryDetailContent),
        (status = 404, description = "Not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Common"
)]
pub async fn fetch_country_detail(
    state: Data<AppState>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .common_usecase
        .fetch_country_detail(id.into_inner())
}

// [authorship] AI-generated (Claude).
#[utoipa::path(
    get,
    path = "/common/languages",
    context_path = "/api",
    params(LanguagesListQueryParams),
    responses(
        (status = 200, description = "Languages ordered by name; `id` is what `language_ids` / `languages_ids` reference", body = Vec<super::presenters::LanguageContent>),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Common"
)]
pub async fn fetch_all_languages(
    state: Data<AppState>,
    params: Query<LanguagesListQueryParams>,
) -> Result<HttpResponse, AppError> {
    let offset = params.offset.unwrap_or(0).max(0);
    let limit = params.limit.unwrap_or(100).clamp(0, 500);
    let name = params.name.clone();

    state
        .di_container
        .common_usecase
        .fetch_all_languages(FetchAllLanguagesUsecaseInput {
            limit,
            offset,
            name,
        })
}

// [authorship] Human-written (original codebase).
#[utoipa::path(
    get,
    path = "/common/org_types",
    context_path = "/api",
    responses(
        (status = 200, description = "Organisation types list response", body = Vec<super::presenters::OrganisationTypeContent>),
    ),
    tag = "Common"
)]
pub async fn fetch_all_organisation_types(state: Data<AppState>) -> Result<HttpResponse, AppError> {
    state.di_container.common_usecase.fetch_organisation_types()
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// admin-only RBAC gate + optional stable slug.
#[utoipa::path(
    post,
    path = "/common/org_types",
    context_path = "/api",
    request_body = CreateOrganisationTypeRequest,
    responses(
        (status = 200, description = "Organisation type created successfully", body = super::presenters::OrganisationTypeContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 403, description = "Forbidden (admin only)", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Common"
)]
pub async fn create_organisation_type(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<CreateOrganisationTypeRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state.di_container.common_usecase.create_organisation_type(
        caller,
        CreateOrganisationTypeUsecaseInput {
            org_type: form.org_type.to_owned(),
            color: form.color.to_owned(),
            title: form.title.to_owned(),
            slug: form.slug.to_owned(),
        },
    )
}

#[derive(Deserialize, ToSchema)]
pub struct CreateOrganisationTypeRequest {
    /// Type key, e.g. "embassy".
    pub org_type: String,
    /// Hex colour for map pins and badges, e.g. "#4A6D8C".
    pub color: String,
    /// Human-readable name, e.g. "Embassy".
    pub title: String,
    /// Optional stable key for filters/stats; must be unique.
    pub slug: Option<String>,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct CountriesListQueryParams {
    /// Page size (default 20, max 250).
    pub limit: Option<i64>,
    /// Rows to skip (default 0, max 150).
    pub offset: Option<i64>,
    /// Case-insensitive substring filter on the country name.
    pub name: Option<String>,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct LanguagesListQueryParams {
    /// Page size (default 100, max 500).
    pub limit: Option<i64>,
    /// Rows to skip (default 0).
    pub offset: Option<i64>,
    /// Case-insensitive substring filter on the language name.
    pub name: Option<String>,
}

/// DB-backed HTTP tests (skipped without TEST_DATABASE_URL, see test_support).
#[cfg(test)]
mod http_tests {
    use crate::test_support::{self as ts, OrgSpec, call, require_db, test_app};
    use actix_web::http::{Method, StatusCode};
    use serde_json::{Value, json};
    use serial_test::serial;

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_countries_list_and_detail() {
        let pool = require_db!();
        let app = test_app!(pool);
        let country = ts::country(&pool);
        let kind = ts::org_type(&pool);
        ts::org(
            &pool,
            OrgSpec {
                country: Some(country.id),
                org_type: Some(kind.id),
                ..Default::default()
            },
        );
        ts::org(
            &pool,
            OrgSpec {
                country: Some(country.id),
                ..Default::default()
            },
        ); // no type → "other"
        ts::org(
            &pool,
            OrgSpec {
                country: Some(country.id),
                status: Some("pending"),
                ..Default::default()
            },
        );

        let suffix = country.name.rsplit(' ').next().unwrap().to_uppercase();
        let (status, list) = call(
            &app,
            Method::GET,
            &format!("/api/common/countries?name={suffix}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let names: Vec<&str> = list
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec![country.name.as_str()],
            "case-insensitive substring filter"
        );

        let (_, page) = call(
            &app,
            Method::GET,
            "/api/common/countries?limit=1&offset=0",
            None,
            None,
        )
        .await;
        assert_eq!(page.as_array().unwrap().len(), 1);

        let (status, detail) = call(
            &app,
            Method::GET,
            &format!("/api/common/countries/{}", country.id),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{detail}");
        assert_eq!(detail["name"], country.name);
        assert_eq!(detail["totalPlaces"], 2, "only live organisations");
        let mut by_type: Vec<Value> = detail["placesByType"].as_array().unwrap().clone();
        by_type.sort_by_key(|t| t["slug"].as_str().unwrap().to_string());
        assert_eq!(
            by_type,
            vec![
                json!({ "slug": "other", "count": 1 }),
                json!({ "slug": kind.slug, "count": 1 })
            ]
        );

        let (status, _) = call(
            &app,
            Method::GET,
            &format!("/api/common/countries/{}", uuid::Uuid::new_v4()),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_languages_list() {
        use crate::data::schema::languages;
        use diesel::prelude::*;

        let pool = require_db!();
        let app = test_app!(pool);
        let marker = ts::unique();
        let conn = &mut pool.get().unwrap();
        let ids: Vec<uuid::Uuid> = ["B", "A"]
            .iter()
            .map(|prefix| {
                diesel::insert_into(languages::table)
                    .values((
                        languages::name.eq(format!("{prefix} Lang {marker}")),
                        languages::symbol.eq("xx"),
                    ))
                    .returning(languages::id)
                    .get_result(conn)
                    .unwrap()
            })
            .collect();

        let (status, list) = call(
            &app,
            Method::GET,
            &format!("/api/common/languages?name={}", marker.to_uppercase()),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{list}");
        assert_eq!(
            list,
            json!([
                { "id": ids[1], "name": format!("A Lang {marker}"), "symbol": "xx" },
                { "id": ids[0], "name": format!("B Lang {marker}"), "symbol": "xx" },
            ]),
            "public, case-insensitive name filter, ordered by name"
        );

        let (status, page) = call(
            &app,
            Method::GET,
            &format!("/api/common/languages?name={marker}&limit=1&offset=1"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(page.as_array().unwrap().len(), 1);
        assert_eq!(page[0]["id"], json!(ids[0]));

        let (status, _) = call(
            &app,
            Method::GET,
            "/api/common/languages?limit=-5&offset=-5",
            None,
            None,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "negative paging is clamped, not a 500"
        );
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_organisation_types_are_admin_managed() {
        let pool = require_db!();
        let app = test_app!(pool);
        let (admin, moderator) = (ts::user(&pool, "admin"), ts::user(&pool, "moderator"));
        let slug = format!("legal-{}", ts::unique());
        let body =
            json!({ "org_type": "legal", "color": "#000000", "title": "Legal aid", "slug": slug });

        let (status, _) = call(
            &app,
            Method::POST,
            "/api/common/org_types",
            Some(&moderator.token),
            Some(body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "admin only");
        let (status, created) = call(
            &app,
            Method::POST,
            "/api/common/org_types",
            Some(&admin.token),
            Some(body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{created}");
        assert_eq!(created["type"], "legal");
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/common/org_types",
            Some(&admin.token),
            Some(body),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "slug is unique");

        let (status, types) = call(&app, Method::GET, "/api/common/org_types", None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            types
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["slug"] == json!(slug))
        );
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/common/org_types",
            None,
            Some(json!({})),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
