use super::{requests::CreateCorridorRequest, usecases::CreateCorridorUsecaseInput};
use crate::app::drivers::middlewares::state::AppState;
use crate::error::AppError;
use actix_web::{
    HttpMessage, HttpRequest, HttpResponse,
    web::{Data, Json, Path},
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

// [authorship] AI-generated (Claude) — the whole corridor feature is new.
#[utoipa::path(
    post,
    path = "/corridor/create",
    context_path = "/api",
    request_body = CreateCorridorRequest,
    responses(
        (status = 200, description = "Corridor created successfully", body = super::presenters::CorridorContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 422, description = "Unprocessable entity", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Corridor"
)]
pub async fn create_corridor(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<CreateCorridorRequest>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    if form.from_country_id == form.to_country_id {
        return Err(AppError::UnprocessableEntity(
            json!({ "error": "Corridor countries must be different" }),
        ));
    }

    state
        .di_container
        .corridor_usecase
        .create_corridor(CreateCorridorUsecaseInput {
            user_id,
            from_country_id: form.from_country_id,
            to_country_id: form.to_country_id,
            is_default: form.is_default.unwrap_or(false),
        })
}

// [authorship] AI-generated (Claude) — the whole corridor feature is new.
#[utoipa::path(
    get,
    path = "/corridor/list",
    context_path = "/api",
    responses(
        (status = 200, description = "Corridors of the authenticated user", body = super::presenters::MultipleCorridorsResponse),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Corridor"
)]
pub async fn list_corridors(
    req: HttpRequest,
    state: Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state.di_container.corridor_usecase.list_corridors(user_id)
}

// [authorship] AI-generated (Claude) — the whole corridor feature is new.
#[utoipa::path(
    put,
    path = "/corridor/set-default/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Corridor ID to make default")
    ),
    responses(
        (status = 200, description = "Corridor set as default", body = super::presenters::CorridorContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 403, description = "Forbidden", body = AppError),
        (status = 404, description = "Not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Corridor"
)]
pub async fn set_default_corridor(
    req: HttpRequest,
    state: Data<AppState>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state
        .di_container
        .corridor_usecase
        .set_default_corridor(id.into_inner(), user_id)
}

// [authorship] AI-generated (Claude) — the whole corridor feature is new.
#[utoipa::path(
    delete,
    path = "/corridor/delete/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Corridor ID to delete")
    ),
    responses(
        (status = 200, description = "Corridor deleted successfully"),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 403, description = "Forbidden", body = AppError),
        (status = 404, description = "Not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Corridor"
)]
pub async fn delete_corridor(
    req: HttpRequest,
    state: Data<AppState>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state
        .di_container
        .corridor_usecase
        .delete_corridor(id.into_inner(), user_id)
}

// [authorship] AI-generated (Claude) — the whole corridor feature is new.
#[utoipa::path(
    get,
    path = "/corridor/stats/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Corridor ID to fetch stats for")
    ),
    responses(
        (status = 200, description = "Live-organisation counters for the corridor destination", body = super::presenters::CorridorStatsContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 403, description = "Forbidden", body = AppError),
        (status = 404, description = "Not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Corridor"
)]
pub async fn fetch_corridor_stats(
    req: HttpRequest,
    state: Data<AppState>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state
        .di_container
        .corridor_usecase
        .fetch_corridor_stats(id.into_inner(), user_id)
}

/// DB-backed HTTP tests (skipped without TEST_DATABASE_URL, see test_support).
#[cfg(test)]
mod tests {
    use crate::test_support::{self as ts, OrgSpec, call, require_db, test_app};
    use actix_web::http::{Method, StatusCode};
    use serde_json::json;
    use serial_test::serial;

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_corridor_lifecycle() {
        let pool = require_db!();
        let app = test_app!(pool);
        let me = ts::user(&pool, "user");
        let (home, a, b) = (ts::country(&pool), ts::country(&pool), ts::country(&pool));

        let (status, first) = call(
            &app,
            Method::POST,
            "/api/corridor/create",
            Some(&me.token),
            Some(json!({ "from_country_id": home.id, "to_country_id": a.id, "is_default": true })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{first}");
        assert_eq!(first["isDefault"], true);

        let (status, second) = call(
            &app,
            Method::POST,
            "/api/corridor/create",
            Some(&me.token),
            Some(json!({ "from_country_id": home.id, "to_country_id": b.id })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(second["isDefault"], false);

        let (_, list) = call(
            &app,
            Method::GET,
            "/api/corridor/list",
            Some(&me.token),
            None,
        )
        .await;
        assert_eq!(list["total"], 2);
        assert_eq!(
            list["items"][0]["id"], first["id"],
            "default corridor comes first"
        );

        // Switching the default clears the old one.
        let second_id = second["id"].as_str().unwrap();
        let (status, switched) = call(
            &app,
            Method::PUT,
            &format!("/api/corridor/set-default/{second_id}"),
            Some(&me.token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(switched["isDefault"], true);
        let (_, list) = call(
            &app,
            Method::GET,
            "/api/corridor/list",
            Some(&me.token),
            None,
        )
        .await;
        assert_eq!(list["items"][0]["id"], second["id"]);
        assert_eq!(list["items"][1]["isDefault"], false);

        let (status, _) = call(
            &app,
            Method::DELETE,
            &format!("/api/corridor/delete/{second_id}"),
            Some(&me.token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (_, list) = call(
            &app,
            Method::GET,
            "/api/corridor/list",
            Some(&me.token),
            None,
        )
        .await;
        assert_eq!(list["total"], 1);
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_corridor_stats_count_only_live_orgs_in_the_destination() {
        let pool = require_db!();
        let app = test_app!(pool);
        let me = ts::user(&pool, "user");
        let (home, dest) = (ts::country(&pool), ts::country(&pool));
        let kind = ts::org_type(&pool);
        for status in ["live", "live", "pending"] {
            ts::org(
                &pool,
                OrgSpec {
                    status: Some(status),
                    country: Some(dest.id),
                    org_type: Some(kind.id),
                    ..Default::default()
                },
            );
        }
        ts::org(
            &pool,
            OrgSpec {
                country: Some(home.id),
                ..Default::default()
            },
        ); // other country

        let (_, corridor) = call(
            &app,
            Method::POST,
            "/api/corridor/create",
            Some(&me.token),
            Some(json!({ "from_country_id": home.id, "to_country_id": dest.id })),
        )
        .await;
        let id = corridor["id"].as_str().unwrap();

        let (status, stats) = call(
            &app,
            Method::GET,
            &format!("/api/corridor/stats/{id}"),
            Some(&me.token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{stats}");
        assert_eq!(stats["totalPlaces"], 2);
        assert_eq!(stats["newThisWeek"], 2);
        assert_eq!(stats["byType"], json!([{ "slug": kind.slug, "count": 2 }]));
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_corridor_of_another_user_is_forbidden() {
        let pool = require_db!();
        let app = test_app!(pool);
        let (owner, other) = (ts::user(&pool, "user"), ts::user(&pool, "user"));
        let (home, dest) = (ts::country(&pool), ts::country(&pool));
        let (_, corridor) = call(
            &app,
            Method::POST,
            "/api/corridor/create",
            Some(&owner.token),
            Some(json!({ "from_country_id": home.id, "to_country_id": dest.id })),
        )
        .await;
        let id = corridor["id"].as_str().unwrap();

        for (method, path) in [
            (Method::PUT, format!("/api/corridor/set-default/{id}")),
            (Method::DELETE, format!("/api/corridor/delete/{id}")),
            (Method::GET, format!("/api/corridor/stats/{id}")),
        ] {
            let (status, body) = call(&app, method.clone(), &path, Some(&other.token), None).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}: {body}");
        }
        let (_, list) = call(
            &app,
            Method::GET,
            "/api/corridor/list",
            Some(&other.token),
            None,
        )
        .await;
        assert_eq!(list["total"], 0, "corridors are private to their owner");
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_corridor_validation_and_constraints() {
        let pool = require_db!();
        let app = test_app!(pool);
        let me = ts::user(&pool, "user");
        let (home, dest) = (ts::country(&pool), ts::country(&pool));

        let same = json!({ "from_country_id": home.id, "to_country_id": home.id });
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/corridor/create",
            Some(&me.token),
            Some(same),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        let pair = json!({ "from_country_id": home.id, "to_country_id": dest.id });
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/corridor/create",
            Some(&me.token),
            Some(pair.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, body) = call(
            &app,
            Method::POST,
            "/api/corridor/create",
            Some(&me.token),
            Some(pair),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "duplicate corridor: {body}"
        );
        assert_eq!(body["constraint"], "corridors_unique_per_user");

        let unknown = json!({ "from_country_id": home.id, "to_country_id": uuid::Uuid::new_v4() });
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/corridor/create",
            Some(&me.token),
            Some(unknown),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "unknown country is an FK violation"
        );

        let (status, _) = call(&app, Method::GET, "/api/corridor/list", None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
