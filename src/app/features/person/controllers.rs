use super::{
    entities::ListPeopleFilters,
    requests::{
        ClaimConfirmRequest, CreatePersonRequest, ListPeopleQueryRequest, VouchPersonRequest,
    },
    usecases::{ClaimConfirmUsecaseInput, CreatePersonUsecaseInput},
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

/// The claim endpoints are public, so the auth middleware never populates
/// extensions for them. When a valid Bearer token IS supplied we decode it
/// here ourselves to link the confirming person to their account (an invalid
/// token is simply ignored — the claim token is the real credential).
fn optional_caller_user_id(req: &HttpRequest) -> Option<Uuid> {
    if let Some(user_id) = req.extensions().get::<Uuid>().copied() {
        return Some(user_id);
    }

    let header = req.headers().get("Authorization")?.to_str().ok()?;
    let token = header.strip_prefix("Bearer ")?;

    crate::utils::token::decode_token(token)
        .ok()
        .map(|claims| claims.sub)
}

fn parse_csv(value: &Option<String>) -> Option<Vec<String>> {
    let items: Vec<String> = value
        .as_deref()?
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if items.is_empty() { None } else { Some(items) }
}

// [authorship] AI-generated (Claude) — the whole person feature is new.
#[utoipa::path(
    post,
    path = "/person/create",
    context_path = "/api",
    request_body = CreatePersonRequest,
    responses(
        (status = 200, description = "Person recommended; claim link returned for manual sending", body = super::presenters::PersonCreatedContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 422, description = "Unprocessable entity", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Person"
)]
pub async fn create_person(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<CreatePersonRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .person_usecase
        .create_person(CreatePersonUsecaseInput {
            name: form.name.clone(),
            bio: form.bio.clone(),
            city: form.city.clone(),
            location_country_id: form.location_country_id,
            skills: form.skills.clone().unwrap_or_default(),
            language_ids: form.language_ids.clone().unwrap_or_default(),
            email: form.email.clone(),
            whatsapp: form.whatsapp.clone(),
            send_via: form.send_via.clone(),
            consent_given: form.consent_given,
            show_whatsapp: form.show_whatsapp.unwrap_or(false),
            show_email: form.show_email.unwrap_or(false),
            show_city: form.show_city.unwrap_or(true),
            allow_reviews: form.allow_reviews.unwrap_or(true),
            recommended_by: caller,
        })
}

// [authorship] AI-generated (Claude) — the whole person feature is new.
#[utoipa::path(
    get,
    path = "/person/list",
    context_path = "/api",
    responses(
        (status = 200, description = "Public people list (confirmed/claimed only)", body = super::presenters::MultiplePeopleResponse),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    params(ListPeopleQueryRequest),
    tag = "Person"
)]
pub async fn list_people(
    state: Data<AppState>,
    query: Query<ListPeopleQueryRequest>,
) -> Result<HttpResponse, AppError> {
    let language_ids = parse_csv(&query.language_ids)
        .map(|ids| {
            ids.iter()
                .map(|s| Uuid::parse_str(s))
                .collect::<Result<Vec<Uuid>, _>>()
        })
        .transpose()?;

    state
        .di_container
        .person_usecase
        .list_people(ListPeopleFilters {
            city: query.city.clone(),
            skills: parse_csv(&query.skills),
            language_ids,
            limit: query.limit.unwrap_or(20).clamp(0, 100),
            offset: query.offset.unwrap_or(0).clamp(0, 500),
        })
}

// [authorship] AI-generated (Claude) — the whole person feature is new.
#[utoipa::path(
    get,
    path = "/person/fetch/{id}",
    context_path = "/api",
    params(
        ("id" = Uuid, Path, description = "Person ID")
    ),
    responses(
        (status = 200, description = "Person profile (contacts gated by status + privacy)", body = super::presenters::PersonContent),
        (status = 404, description = "Not found", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Person"
)]
pub async fn fetch_person(state: Data<AppState>, id: Path<Uuid>) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .person_usecase
        .fetch_person(id.into_inner())
}

// [authorship] AI-generated (Claude) — the whole person feature is new.
#[utoipa::path(
    post,
    path = "/person/vouch/{id}",
    context_path = "/api",
    request_body = VouchPersonRequest,
    params(
        ("id" = Uuid, Path, description = "Person ID to vouch for")
    ),
    responses(
        (status = 200, description = "Vouch recorded; updated person returned", body = super::presenters::PersonContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 422, description = "Unprocessable entity (not public / already vouched)", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Person"
)]
pub async fn vouch_person(
    req: HttpRequest,
    state: Data<AppState>,
    id: Path<Uuid>,
    form: Json<VouchPersonRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;

    state
        .di_container
        .person_usecase
        .vouch_person(id.into_inner(), caller, form.note.clone())
}

// [authorship] AI-generated (Claude) — the whole person feature is new.
// Public: the claim token itself is the credential.
#[utoipa::path(
    get,
    path = "/person/claim/{token}",
    context_path = "/api",
    params(
        ("token" = String, Path, description = "Claim token from the invite link")
    ),
    responses(
        (status = 200, description = "Claim preview for the recommended person", body = super::presenters::ClaimPreviewContent),
        (status = 404, description = "Unknown token", body = AppError),
        (status = 422, description = "Token used or expired", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Person"
)]
pub async fn claim_preview(
    state: Data<AppState>,
    token: Path<String>,
) -> Result<HttpResponse, AppError> {
    state.di_container.person_usecase.claim_preview(&token)
}

// [authorship] AI-generated (Claude) — the whole person feature is new.
#[utoipa::path(
    post,
    path = "/person/claim/{token}/confirm",
    context_path = "/api",
    request_body = ClaimConfirmRequest,
    params(
        ("token" = String, Path, description = "Claim token from the invite link")
    ),
    responses(
        (status = 200, description = "Person confirmed (claimed when authenticated)", body = super::presenters::ClaimPreviewContent),
        (status = 403, description = "The recommender tried to claim the profile", body = AppError),
        (status = 404, description = "Unknown token", body = AppError),
        (status = 422, description = "Not yet approved by moderation, token used/expired or person already resolved", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Person"
)]
pub async fn claim_confirm(
    req: HttpRequest,
    state: Data<AppState>,
    token: Path<String>,
    form: Json<ClaimConfirmRequest>,
) -> Result<HttpResponse, AppError> {
    state.di_container.person_usecase.claim_confirm(
        &token,
        ClaimConfirmUsecaseInput {
            claimed_by: optional_caller_user_id(&req),
            show_whatsapp: form.show_whatsapp,
            show_email: form.show_email,
            show_city: form.show_city,
            allow_reviews: form.allow_reviews,
        },
    )
}

// [authorship] AI-generated (Claude) — the whole person feature is new.
#[utoipa::path(
    post,
    path = "/person/claim/{token}/decline",
    context_path = "/api",
    params(
        ("token" = String, Path, description = "Claim token from the invite link")
    ),
    responses(
        (status = 200, description = "Recommendation declined"),
        (status = 404, description = "Unknown token", body = AppError),
        (status = 422, description = "Token used/expired", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "Person"
)]
pub async fn claim_decline(
    state: Data<AppState>,
    token: Path<String>,
) -> Result<HttpResponse, AppError> {
    state.di_container.person_usecase.claim_decline(&token)
}

/// DB-backed HTTP tests (skipped without TEST_DATABASE_URL, see test_support).
#[cfg(test)]
mod http_tests {
    use crate::data::schema::{people, person_claim_tokens};
    use crate::test_support::{self as ts, TestUser, call, require_db, test_app};
    use actix_web::http::{Method, StatusCode};
    use diesel::prelude::*;
    use serde_json::{Value, json};
    use serial_test::serial;

    fn recommendation(extra: Value) -> Value {
        let mut body = json!({
            "name": format!("Helper {}", ts::unique()), "bio": "Knows the paperwork", "city": "Berlin",
            "skills": ["Paperwork", "Housing"], "email": "helper@example.test", "whatsapp": "+4915112345678",
            "send_via": "email", "consent_given": true, "show_email": true,
        });
        body.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        body
    }

    /// Recommends a person as `recommender` and approves it as `moderator`;
    /// returns (person id, claim token).
    async fn approved_person<S, B>(
        app: &S,
        recommender: &TestUser,
        moderator: &TestUser,
        extra: Value,
    ) -> (String, String)
    where
        S: actix_web::dev::Service<
                actix_http::Request,
                Response = actix_web::dev::ServiceResponse<B>,
                Error = actix_web::Error,
            >,
        B: actix_web::body::MessageBody,
    {
        let (status, created) = call(
            app,
            Method::POST,
            "/api/person/create",
            Some(&recommender.token),
            Some(recommendation(extra)),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{created}");
        let id = created["person"]["id"].as_str().unwrap().to_string();
        let token = created["claimUrl"]
            .as_str()
            .unwrap()
            .trim_start_matches("/claim/")
            .to_string();
        let (status, body) = call(
            app,
            Method::POST,
            "/api/moderation/approve",
            Some(&moderator.token),
            Some(json!({ "kind": "person", "target_id": id })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        (id, token)
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_recommendation_claim_flow_and_contact_gating() {
        let pool = require_db!();
        let app = test_app!(pool);
        let (recommender, moderator, claimer) = (
            ts::user(&pool, "user"),
            ts::user(&pool, "moderator"),
            ts::user(&pool, "user"),
        );

        let (status, created) = call(
            &app,
            Method::POST,
            "/api/person/create",
            Some(&recommender.token),
            Some(recommendation(json!({}))),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(created["person"]["status"], "pending");
        assert!(
            created["person"].get("email").is_none(),
            "contacts hidden before confirmation"
        );
        let id = created["person"]["id"].as_str().unwrap().to_string();
        let token = created["claimUrl"]
            .as_str()
            .unwrap()
            .trim_start_matches("/claim/")
            .to_string();

        // Pending: preview works, confirming does not (the recommender holds the link).
        let (status, preview) = call(
            &app,
            Method::GET,
            &format!("/api/person/claim/{token}"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            preview["email"], "helper@example.test",
            "the claimed person sees their contacts"
        );
        let (status, _) = call(
            &app,
            Method::POST,
            &format!("/api/person/claim/{token}/confirm"),
            None,
            Some(json!({})),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        let (status, _) = call(
            &app,
            Method::POST,
            "/api/moderation/approve",
            Some(&moderator.token),
            Some(json!({ "kind": "person", "target_id": id })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        // The recommender cannot confirm on the person's behalf.
        let (status, _) = call(
            &app,
            Method::POST,
            &format!("/api/person/claim/{token}/confirm"),
            Some(&recommender.token),
            Some(json!({})),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);

        let (status, confirmed) = call(
            &app,
            Method::POST,
            &format!("/api/person/claim/{token}/confirm"),
            Some(&claimer.token),
            Some(json!({ "show_whatsapp": false, "show_email": true })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{confirmed}");
        assert_eq!(
            confirmed["status"], "claimed",
            "a Bearer token links the account"
        );

        let (_, public) = call(
            &app,
            Method::GET,
            &format!("/api/person/fetch/{id}"),
            None,
            None,
        )
        .await;
        assert_eq!(public["email"], "helper@example.test");
        assert!(
            public.get("whatsapp").is_none(),
            "show_whatsapp=false keeps it hidden"
        );

        let (status, _) = call(
            &app,
            Method::POST,
            &format!("/api/person/claim/{token}/decline"),
            None,
            None,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "a used token cannot be reused"
        );
        let (status, _) = call(
            &app,
            Method::GET,
            "/api/person/claim/not-a-token",
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_confirm_without_account_decline_and_expiry() {
        let pool = require_db!();
        let app = test_app!(pool);
        let (recommender, moderator) = (ts::user(&pool, "user"), ts::user(&pool, "moderator"));

        let (_, token) = approved_person(&app, &recommender, &moderator, json!({})).await;
        let (_, body) = call(
            &app,
            Method::POST,
            &format!("/api/person/claim/{token}/confirm"),
            None,
            Some(json!({})),
        )
        .await;
        assert_eq!(
            body["status"], "confirmed",
            "no Bearer token → confirmed, not claimed"
        );

        let (id, token) = approved_person(&app, &recommender, &moderator, json!({})).await;
        let (status, _) = call(
            &app,
            Method::POST,
            &format!("/api/person/claim/{token}/decline"),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = call(
            &app,
            Method::GET,
            &format!("/api/person/claim/{token}"),
            None,
            None,
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "a used token no longer previews"
        );
        let declined: String = people::table
            .find(id.parse::<uuid::Uuid>().unwrap())
            .select(people::status)
            .first(&mut pool.get().unwrap())
            .unwrap();
        assert_eq!(declined, "declined");

        let (_, token) = approved_person(&app, &recommender, &moderator, json!({})).await;
        diesel::update(person_claim_tokens::table.filter(person_claim_tokens::token.eq(&token)))
            .set(
                person_claim_tokens::expires_at
                    .eq(chrono::Utc::now().naive_utc() - chrono::Duration::days(1)),
            )
            .execute(&mut pool.get().unwrap())
            .unwrap();
        let (status, body) = call(
            &app,
            Method::POST,
            &format!("/api/person/claim/{token}/confirm"),
            None,
            Some(json!({})),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "expired: {body}");
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_list_vouch_and_validation() {
        let pool = require_db!();
        let app = test_app!(pool);
        let (recommender, moderator, voucher) = (
            ts::user(&pool, "user"),
            ts::user(&pool, "moderator"),
            ts::user(&pool, "user"),
        );
        let city = format!("Town{}", ts::unique());

        let (id, token) = approved_person(
            &app,
            &recommender,
            &moderator,
            json!({ "city": city, "skills": ["Relocation"] }),
        )
        .await;
        let (status, _) = call(
            &app,
            Method::POST,
            &format!("/api/person/vouch/{id}"),
            Some(&voucher.token),
            Some(json!({})),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "awaiting people cannot be vouched for yet"
        );
        call(
            &app,
            Method::POST,
            &format!("/api/person/claim/{token}/confirm"),
            None,
            Some(json!({})),
        )
        .await;

        let (status, _) = call(
            &app,
            Method::POST,
            &format!("/api/person/vouch/{id}"),
            Some(&voucher.token),
            Some(json!({ "note": "great" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = call(
            &app,
            Method::POST,
            &format!("/api/person/vouch/{id}"),
            Some(&voucher.token),
            Some(json!({})),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "one vouch per user"
        );

        let (status, list) = call(
            &app,
            Method::GET,
            &format!(
                "/api/person/list?city={}&skills=Relocation",
                city.to_lowercase()
            ),
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(list["total"], 1);
        assert_eq!(list["items"][0]["vouches"], 1);
        let (_, none) = call(
            &app,
            Method::GET,
            &format!("/api/person/list?city={city}&skills=Cooking"),
            None,
            None,
        )
        .await;
        assert_eq!(none["total"], 0);

        for (extra, why) in [
            (json!({ "consent_given": false }), "consent"),
            (json!({ "email": null, "whatsapp": null }), "a contact"),
            (json!({ "send_via": "pigeon" }), "send_via"),
        ] {
            let (status, _) = call(
                &app,
                Method::POST,
                "/api/person/create",
                Some(&recommender.token),
                Some(recommendation(extra)),
            )
            .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{why}");
        }
        let (status, _) = call(
            &app,
            Method::POST,
            "/api/person/create",
            None,
            Some(recommendation(json!({}))),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
