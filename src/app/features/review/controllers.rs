use super::{
    requests::{CreateReviewRequest, ListReviewsQueryRequest},
    usecases::CreateReviewUsecaseInput,
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

// [authorship] AI-generated (Claude) — the whole review feature is new.
#[utoipa::path(
    post,
    path = "/review/create",
    context_path = "/api",
    request_body = CreateReviewRequest,
    responses(
        (status = 200, description = "Review created; target aggregates refreshed", body = super::presenters::ReviewContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 403, description = "Reviews disabled by the person", body = AppError),
        (status = 422, description = "Unprocessable entity (bad target/rating/duplicate)", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Review"
)]
pub async fn create_review(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<CreateReviewRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .review_usecase
        .create_review(CreateReviewUsecaseInput {
            author_id: caller,
            organisation_id: form.organisation_id,
            person_id: form.person_id,
            rating: form.rating,
            topic: form.topic.clone(),
            text: form.text.clone(),
        })
}

// [authorship] AI-generated (Claude) — the whole review feature is new.
#[utoipa::path(
    get,
    path = "/review/list",
    context_path = "/api",
    responses(
        (status = 200, description = "Reviews of one target", body = super::presenters::MultipleReviewsResponse),
        (status = 422, description = "Exactly one target must be given", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    params(ListReviewsQueryRequest),
    tag = "Review"
)]
pub async fn list_reviews(
    state: Data<AppState>,
    query: Query<ListReviewsQueryRequest>,
) -> Result<HttpResponse, AppError> {
    state.di_container.review_usecase.list_reviews(
        query.organisation_id,
        query.person_id,
        query.limit.unwrap_or(20).clamp(0, 100),
        query.offset.unwrap_or(0).clamp(0, 500),
    )
}

// [authorship] AI-generated (Claude) — the whole review feature is new.
#[utoipa::path(
    delete,
    path = "/review/delete/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Review ID to delete")
    ),
    responses(
        (status = 200, description = "Review deleted; target aggregates refreshed"),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 403, description = "Forbidden (author or moderator only)", body = AppError),
        (status = 404, description = "Not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Review"
)]
pub async fn delete_review(
    req: HttpRequest,
    state: Data<AppState>,
    id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .review_usecase
        .delete_review(id.into_inner(), caller)
}

/// DB-backed HTTP tests (skipped without TEST_DATABASE_URL, see test_support).
#[cfg(test)]
mod http_tests {
    use crate::test_support::{self as ts, OrgSpec, call, require_db, test_app};
    use actix_web::http::{Method, StatusCode};
    use serde_json::json;
    use serial_test::serial;

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_reviews_update_the_target_aggregates() {
        let pool = require_db!();
        let app = test_app!(pool);
        let (alice, bob, moderator) = (
            ts::user(&pool, "user"),
            ts::user(&pool, "user"),
            ts::user(&pool, "moderator"),
        );
        let org = ts::org(&pool, OrgSpec::default());

        let (status, first) = call(&app, Method::POST, "/api/review/create", Some(&alice.token),
            Some(json!({ "organisation_id": org.id, "rating": 5, "topic": "Anmeldung help", "text": "Great" }))).await;
        assert_eq!(status, StatusCode::OK, "{first}");
        let (status, second) = call(
            &app,
            Method::POST,
            "/api/review/create",
            Some(&bob.token),
            Some(json!({ "organisation_id": org.id, "rating": 2 })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (_, detail) = call(
            &app,
            Method::GET,
            &format!("/api/organisation/fetch/{}", org.id),
            None,
            None,
        )
        .await;
        assert_eq!(
            (
                detail["ratingAvg"].as_f64(),
                detail["reviewsCount"].as_i64()
            ),
            (Some(3.5), Some(2))
        );

        let (status, list) = call(
            &app,
            Method::GET,
            &format!("/api/review/list?organisation_id={}", org.id),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(list["total"], 2);
        assert_eq!(list["items"][0]["id"], second["id"], "newest first");

        let (status, _) = call(
            &app,
            Method::POST,
            "/api/review/create",
            Some(&alice.token),
            Some(json!({ "organisation_id": org.id, "rating": 4 })),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "one review per author and target"
        );

        let first_id = first["id"].as_str().unwrap();
        let (status, _) = call(
            &app,
            Method::DELETE,
            &format!("/api/review/delete/{first_id}"),
            Some(&bob.token),
            None,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "only the author or a moderator"
        );
        let (status, _) = call(
            &app,
            Method::DELETE,
            &format!("/api/review/delete/{first_id}"),
            Some(&moderator.token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (_, detail) = call(
            &app,
            Method::GET,
            &format!("/api/organisation/fetch/{}", org.id),
            None,
            None,
        )
        .await;
        assert_eq!(
            (
                detail["ratingAvg"].as_f64(),
                detail["reviewsCount"].as_i64()
            ),
            (Some(2.0), Some(1))
        );
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_review_validation() {
        let pool = require_db!();
        let app = test_app!(pool);
        let me = ts::user(&pool, "user");
        let live = ts::org(&pool, OrgSpec::default());
        let pending = ts::org(
            &pool,
            OrgSpec {
                status: Some("pending"),
                ..Default::default()
            },
        );

        for body in [
            json!({ "organisation_id": live.id, "rating": 6 }),
            json!({ "organisation_id": live.id, "rating": 0 }),
            json!({ "rating": 3 }),
            json!({ "organisation_id": live.id, "person_id": live.id, "rating": 3 }),
            json!({ "organisation_id": pending.id, "rating": 3 }),
        ] {
            let (status, resp) = call(
                &app,
                Method::POST,
                "/api/review/create",
                Some(&me.token),
                Some(body.clone()),
            )
            .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body} -> {resp}");
        }
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/review/create",
            Some(&me.token),
            Some(json!({ "organisation_id": uuid::Uuid::new_v4(), "rating": 3 })),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = call(&app, Method::GET, "/api/review/list", None, None).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "a target is required"
        );
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/review/create",
            None,
            Some(json!({ "organisation_id": live.id, "rating": 3 })),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
