use super::{
    requests::{CreateSavedRequest, ListSavedQueryRequest},
    usecases::CreateSavedUsecaseInput,
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

// [authorship] AI-generated (Claude) — the whole saved feature is new.
#[utoipa::path(
    post,
    path = "/saved/create",
    context_path = "/api",
    request_body = CreateSavedRequest,
    responses(
        (status = 200, description = "Bookmark saved", body = super::presenters::SavedItemContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 422, description = "Unknown kind or already saved", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Saved"
)]
pub async fn create_saved(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<CreateSavedRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .saved_usecase
        .create_saved(CreateSavedUsecaseInput {
            user_id: caller,
            kind: form.kind.clone(),
            target_id: form.target_id,
            note: form.note.clone(),
            list_name: form.list_name.clone(),
        })
}

// [authorship] AI-generated (Claude) — the whole saved feature is new.
#[utoipa::path(
    delete,
    path = "/saved/delete/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Saved item ID")
    ),
    responses(
        (status = 200, description = "Bookmark removed"),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 404, description = "Not found (or not yours)", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Saved"
)]
pub async fn delete_saved(
    req: HttpRequest,
    state: Data<AppState>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .saved_usecase
        .delete_saved(id.into_inner(), caller)
}

// [authorship] AI-generated (Claude) — the whole saved feature is new.
#[utoipa::path(
    get,
    path = "/saved/list",
    context_path = "/api",
    responses(
        (status = 200, description = "The caller's bookmarks", body = super::presenters::MultipleSavedItemsResponse),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    params(ListSavedQueryRequest),
    tag = "Saved"
)]
pub async fn list_saved(
    req: HttpRequest,
    state: Data<AppState>,
    query: Query<ListSavedQueryRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .saved_usecase
        .list_saved(caller, query.kind.clone())
}

// [authorship] AI-generated (Claude) — the whole saved feature is new.
#[utoipa::path(
    get,
    path = "/saved/counts",
    context_path = "/api",
    responses(
        (status = 200, description = "Bookmark counters per kind", body = super::presenters::SavedCountsContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Saved"
)]
pub async fn counts_saved(
    req: HttpRequest,
    state: Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state.di_container.saved_usecase.counts_saved(caller)
}

/// DB-backed HTTP tests (skipped without TEST_DATABASE_URL, see test_support).
#[cfg(test)]
mod http_tests {
    use crate::test_support::{self as ts, call, require_db, test_app};
    use actix_web::http::{Method, StatusCode};
    use serde_json::json;
    use serial_test::serial;
    use uuid::Uuid;

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_saved_items_are_private_and_counted_by_kind() {
        let pool = require_db!();
        let app = test_app!(pool);
        let (me, other) = (ts::user(&pool, "user"), ts::user(&pool, "user"));

        let mut ids = vec![];
        for (kind, note) in [
            ("org", Some("call on Monday")),
            ("org", None),
            ("country", None),
            ("person", None),
        ] {
            let (status, item) = call(&app, Method::POST, "/api/saved/create", Some(&me.token),
                Some(json!({ "kind": kind, "target_id": Uuid::new_v4(), "note": note, "list_name": "Moving" }))).await;
            assert_eq!(status, StatusCode::OK, "{item}");
            ids.push(item["id"].as_str().unwrap().to_string());
        }

        let (_, counts) = call(
            &app,
            Method::GET,
            "/api/saved/counts",
            Some(&me.token),
            None,
        )
        .await;
        assert_eq!(
            counts,
            json!({ "org": 2, "person": 1, "country": 1, "corridor": 0, "total": 4 })
        );
        let (_, orgs) = call(
            &app,
            Method::GET,
            "/api/saved/list?kind=org",
            Some(&me.token),
            None,
        )
        .await;
        assert_eq!(orgs["total"], 2);
        let (_, all) = call(&app, Method::GET, "/api/saved/list", Some(&me.token), None).await;
        assert_eq!(all["items"][0]["id"], json!(ids[3]), "newest first");
        let (_, theirs) = call(
            &app,
            Method::GET,
            "/api/saved/list",
            Some(&other.token),
            None,
        )
        .await;
        assert_eq!(theirs["total"], 0);

        let (status, _) = call(
            &app,
            Method::DELETE,
            &format!("/api/saved/delete/{}", ids[0]),
            Some(&other.token),
            None,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "another user's item looks like it does not exist"
        );
        let (status, _) = call(
            &app,
            Method::DELETE,
            &format!("/api/saved/delete/{}", ids[0]),
            Some(&me.token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (_, counts) = call(
            &app,
            Method::GET,
            "/api/saved/counts",
            Some(&me.token),
            None,
        )
        .await;
        assert_eq!(counts["total"], 3);
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_saved_validation() {
        let pool = require_db!();
        let app = test_app!(pool);
        let me = ts::user(&pool, "user");
        let target = Uuid::new_v4();
        let item = json!({ "kind": "corridor", "target_id": target });

        let (status, _) = call(
            &app,
            Method::POST,
            "/api/saved/create",
            Some(&me.token),
            Some(item.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/saved/create",
            Some(&me.token),
            Some(item),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "saved once per user and target"
        );
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/saved/create",
            Some(&me.token),
            Some(json!({ "kind": "song", "target_id": target })),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let (status, _) = call(
            &app,
            Method::GET,
            "/api/saved/list?kind=song",
            Some(&me.token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let (status, _) = call(&app, Method::GET, "/api/saved/counts", None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
