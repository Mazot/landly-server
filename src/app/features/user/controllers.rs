use super::oauth::google::{STATE_COOKIE, state_matches_cookie};
use super::requests::{
    AddLanguagesRequest, DeleteLanguageRequest, OAuthCallbackParams, OAuthExchangeRequest,
    SignInRequest, SignUpRequest, UpdateNotificationSettingsRequest, UpdateProfileRequest,
};
use super::usecases::{SignUpUsecaseInput, UpdateProfileUsecaseInput};
use crate::{app::drivers::middlewares::state::AppState, constants::env_key, error::AppError};
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
    post,
    path = "/user/signin",
    context_path = "/api",
    request_body = SignInRequest,
    responses(
        (status = 200, description = "User signed in successfully", body = super::presenters::AuthUserContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 401, description = "Invalid email or password (same response for unknown emails)", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn signin(
    state: Data<AppState>,
    form: Json<SignInRequest>,
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .user_usecase
        .signin(form.email.to_owned(), form.password.to_owned())
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// signup v2 — optional profile fields + default corridor in one transaction.
#[utoipa::path(
    post,
    path = "/user/signup",
    context_path = "/api",
    request_body = SignUpRequest,
    responses(
        (status = 200, description = "User signed up successfully", body = super::presenters::AuthUserContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 422, description = "Invalid email, password shorter than 8 characters / over 72 bytes, bad username or oversized field", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn signup(
    state: Data<AppState>,
    form: Json<SignUpRequest>,
) -> Result<HttpResponse, AppError> {
    state.di_container.user_usecase.signup(SignUpUsecaseInput {
        username: form.username.to_owned(),
        email: form.email.to_owned(),
        password: form.password.to_owned(),
        name: form.name.to_owned(),
        locale: form.locale.to_owned(),
        here_as: form.here_as.to_owned(),
        home_country_id: form.home_country_id,
        avatar_color: form.avatar_color.to_owned(),
        corridor_from_country_id: form.corridor_from_country_id,
        corridor_to_country_id: form.corridor_to_country_id,
    })
}

// [authorship] AI-generated (Claude).
#[utoipa::path(
    get,
    path = "/user/me",
    context_path = "/api",
    responses(
        (status = 200, description = "Authenticated user profile", body = super::presenters::UserProfileContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn fetch_me(req: HttpRequest, state: Data<AppState>) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state.di_container.user_usecase.fetch_profile(user_id)
}

// [authorship] AI-generated (Claude).
#[utoipa::path(
    put,
    path = "/user/me",
    context_path = "/api",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Profile updated", body = super::presenters::UserProfileContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 422, description = "Unprocessable entity", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn update_me(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<UpdateProfileRequest>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state.di_container.user_usecase.update_profile(
        user_id,
        UpdateProfileUsecaseInput {
            name: form.name.to_owned(),
            bio: form.bio.to_owned(),
            city: form.city.to_owned(),
            home_country_id: form.home_country_id,
            avatar_color: form.avatar_color.to_owned(),
            locale: form.locale.to_owned(),
            here_as: form.here_as.to_owned(),
        },
    )
}

// [authorship] AI-generated (Claude).
#[utoipa::path(
    put,
    path = "/user/me/notifications",
    context_path = "/api",
    request_body = UpdateNotificationSettingsRequest,
    responses(
        (status = 200, description = "Notification settings updated", body = super::presenters::UserProfileContent),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn update_notification_settings(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<UpdateNotificationSettingsRequest>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state
        .di_container
        .user_usecase
        .update_notification_settings(user_id, form.notification_settings.to_owned())
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// user_id now comes from the JWT instead of the request body.
#[utoipa::path(
    post,
    path = "/user/languages",
    context_path = "/api",
    request_body = AddLanguagesRequest,
    responses(
        (status = 200, description = "Languages added successfully", body = super::presenters::UserLanguagesContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn add_languages(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<AddLanguagesRequest>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state
        .di_container
        .user_usecase
        .add_languages(user_id, form.languages_ids.clone())
}

// [authorship] Human-written (original codebase); extended by AI (Claude):
// user_id now comes from the JWT instead of the request body.
#[utoipa::path(
    delete,
    path = "/user/languages",
    context_path = "/api",
    request_body = DeleteLanguageRequest,
    responses(
        (status = 200, description = "Language deleted successfully"),
        (status = 400, description = "Bad request", body = AppError),
        (status = 401, description = "Unauthorized", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn delete_language(
    req: HttpRequest,
    state: Data<AppState>,
    form: Json<DeleteLanguageRequest>,
) -> Result<HttpResponse, AppError> {
    let user_id = caller_user_id(&req)?;

    state
        .di_container
        .user_usecase
        .delete_language(user_id, form.language_id)
}

// [authorship] Human-written (original codebase).
#[utoipa::path(
    get,
    path = "/user/{user_id}/languages",
    context_path = "/api",
    params(
        ("user_id" = Uuid, Path, description = "User ID to fetch languages for")
    ),
    responses(
        (status = 200, description = "Languages fetched successfully", body = super::presenters::UserLanguagesContent),
        (status = 400, description = "Bad request", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn fetch_languages(
    state: Data<AppState>,
    user_id: Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    state
        .di_container
        .user_usecase
        .fetch_languages(user_id.into_inner())
}

// [authorship] Human-written (original codebase).
#[utoipa::path(
    get,
    path = "/user/oauth/google/login",
    context_path = "/api",
    responses(
        (status = 302, description = "Redirect to Google OAuth; sets the HttpOnly state cookie checked by the callback"),
        (status = 503, description = "Google OAuth is not configured", body = AppError)
    ),
    tag = "User"
)]
pub async fn oauth_google_login(state: Data<AppState>) -> Result<HttpResponse, AppError> {
    let oauth = state.di_container.oauth_google.as_ref().ok_or_else(|| {
        AppError::ServiceUnavailable(json!({ "error": "Google OAuth is not configured" }))
    })?;
    let (url, state) = oauth.auth_url()?;

    Ok(HttpResponse::Found()
        .append_header(("Location", url.to_string()))
        .cookie(oauth.state_cookie(&state))
        .finish())
}

// [authorship] Human-written (original codebase).
#[utoipa::path(
    get,
    path = "/user/oauth/google/callback",
    context_path = "/api",
    params(OAuthCallbackParams),
    responses(
        (status = 302, description = "Redirect to {FRONTEND_ORIGIN}/auth/google/callback?code=<single-use code>; exchange it via POST /api/user/oauth/google/exchange"),
        (status = 401, description = "Invalid state, or the Google email is missing/unverified", body = AppError),
        (status = 403, description = "A password account with this email already exists", body = AppError),
        (status = 503, description = "Google OAuth is not configured", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn oauth_google_callback(
    req: HttpRequest,
    state: Data<AppState>,
    query: Query<OAuthCallbackParams>,
) -> Result<HttpResponse, AppError> {
    let oauth = state.di_container.oauth_google.as_ref().ok_or_else(|| {
        AppError::ServiceUnavailable(json!({ "error": "Google OAuth is not configured" }))
    })?;

    // The state must come back to the same browser that started the flow.
    let cookie_state = req.cookie(STATE_COOKIE);
    if !state_matches_cookie(cookie_state.as_ref().map(|c| c.value()), &query.state) {
        return Err(AppError::Unauthorized(
            json!({ "error": "OAuth state does not match this browser session" }),
        ));
    }
    let user_info = oauth
        .exchange_and_userinfo(query.code.to_owned(), query.state.to_owned())
        .await?;
    let email = user_info.verified_email()?;

    let auth = state
        .di_container
        .user_usecase
        .oauth_google_upsert_content(email, user_info.sub)?;

    let frontend_origin = std::env::var(env_key::FRONTEND_ORIGIN)
        .unwrap_or_else(|_| "http://localhost:5173".to_string());

    // Only a single-use code travels in the URL; the frontend trades it for
    // the session via POST /user/oauth/google/exchange.
    let login_code = oauth.issue_login_code(&auth)?;
    let redirect_url = format!(
        "{}/auth/google/callback?code={}",
        frontend_origin,
        urlencoding::encode(&login_code),
    );

    Ok(HttpResponse::Found()
        .append_header(("Location", redirect_url))
        .cookie(oauth.clear_state_cookie())
        .finish())
}

// [authorship] AI-generated (Claude) — replaces the JWT-in-URL callback
// redirect with a single-use code exchange.
#[utoipa::path(
    post,
    path = "/user/oauth/google/exchange",
    context_path = "/api",
    request_body = OAuthExchangeRequest,
    responses(
        (status = 200, description = "Session for the Google login the code was issued for", body = super::presenters::AuthUserContent),
        (status = 401, description = "Unknown, expired or already used code", body = AppError),
        (status = 503, description = "Google OAuth is not configured", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "User"
)]
pub async fn oauth_google_exchange(
    state: Data<AppState>,
    form: Json<OAuthExchangeRequest>,
) -> Result<HttpResponse, AppError> {
    let oauth = state.di_container.oauth_google.as_ref().ok_or_else(|| {
        AppError::ServiceUnavailable(json!({ "error": "Google OAuth is not configured" }))
    })?;

    Ok(HttpResponse::Ok().json(oauth.redeem_login_code(&form.code)?))
}

/// DB-backed HTTP tests (skipped without TEST_DATABASE_URL, see test_support).
#[cfg(test)]
mod http_tests {
    use crate::data::schema::languages;
    use crate::test_support::{self as ts, call, require_db, test_app};
    use actix_web::http::{Method, StatusCode};
    use diesel::prelude::*;
    use serde_json::json;
    use serial_test::serial;
    use uuid::Uuid;

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_signup_signin_and_credential_rules() {
        let pool = require_db!();
        let app = test_app!(pool);
        let (home, dest) = (ts::country(&pool), ts::country(&pool));
        let u = ts::unique();
        let email = format!("Mixed.{u}@Example.TEST");

        let (status, auth) = call(
            &app,
            Method::POST,
            "/api/user/signup",
            None,
            Some(json!({
                "username": format!("new{u}"), "email": email, "password": "correct-horse",
                "name": "Daria", "locale": "ru", "here_as": "newcomer",
                "corridor_from_country_id": home.id, "corridor_to_country_id": dest.id,
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{auth}");
        assert_eq!(
            auth["email"],
            email.to_lowercase(),
            "emails are stored normalized"
        );
        let token = auth["token"].as_str().unwrap().to_string();

        let (_, corridors) =
            call(&app, Method::GET, "/api/corridor/list", Some(&token), None).await;
        assert_eq!(
            corridors["total"], 1,
            "signup v2 creates the default corridor"
        );
        assert_eq!(corridors["items"][0]["isDefault"], true);

        let (status, again) = call(&app, Method::POST, "/api/user/signup", None, Some(json!({
            "username": format!("dup{u}"), "email": email.to_uppercase(), "password": "correct-horse",
        }))).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "case-only duplicate email: {again}"
        );

        let (status, signed_in) = call(
            &app,
            Method::POST,
            "/api/user/signin",
            None,
            Some(json!({ "email": email.to_uppercase(), "password": "correct-horse" })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(signed_in["id"], auth["id"]);

        let (wrong_status, wrong_body) = call(
            &app,
            Method::POST,
            "/api/user/signin",
            None,
            Some(json!({ "email": email, "password": "wrong-password" })),
        )
        .await;
        let (unknown_status, unknown_body) = call(
            &app,
            Method::POST,
            "/api/user/signin",
            None,
            Some(
                json!({ "email": format!("nobody{u}@example.test"), "password": "wrong-password" }),
            ),
        )
        .await;
        assert_eq!(wrong_status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            (unknown_status, unknown_body),
            (wrong_status, wrong_body),
            "no account enumeration"
        );
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_signup_rejects_invalid_input() {
        let pool = require_db!();
        let app = test_app!(pool);
        let u = ts::unique();
        let (home, _) = (ts::country(&pool), ());
        for body in [
            json!({ "username": format!("a{u}"), "email": "not-an-email", "password": "long-enough" }),
            json!({ "username": format!("b{u}"), "email": format!("b{u}@example.test"), "password": "short" }),
            json!({ "username": "x y", "email": format!("c{u}@example.test"), "password": "long-enough" }),
            json!({ "username": format!("d{u}"), "email": format!("d{u}@example.test"), "password": "long-enough", "locale": "fr" }),
            json!({ "username": format!("e{u}"), "email": format!("e{u}@example.test"), "password": "long-enough", "corridor_from_country_id": home.id }),
        ] {
            let (status, resp) = call(
                &app,
                Method::POST,
                "/api/user/signup",
                None,
                Some(body.clone()),
            )
            .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body} -> {resp}");
        }
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_profile_read_and_update() {
        let pool = require_db!();
        let app = test_app!(pool);
        let me = ts::user(&pool, "user");
        let home = ts::country(&pool);
        crate::test_support::org(
            &pool,
            crate::test_support::OrgSpec {
                created_by: Some(me.id),
                ..Default::default()
            },
        );

        let (status, profile) =
            call(&app, Method::GET, "/api/user/me", Some(&me.token), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(profile["email"], me.email);
        assert_eq!(profile["role"], "user");
        assert_eq!(profile["stats"]["placesAdded"], 1);
        assert!(profile.get("passwordHash").is_none());

        let (status, updated) = call(
            &app,
            Method::PUT,
            "/api/user/me",
            Some(&me.token),
            Some(json!({
                "name": "Ivan", "bio": "Hi", "city": "Berlin", "home_country_id": home.id,
                "avatar_color": "#ff8800", "locale": "uk", "here_as": "helping",
            })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{updated}");
        assert_eq!(
            (
                updated["name"].as_str(),
                updated["locale"].as_str(),
                updated["hereAs"].as_str()
            ),
            (Some("Ivan"), Some("uk"), Some("helping"))
        );

        for bad in [
            json!({ "locale": "de" }),
            json!({ "here_as": "tourist" }),
            json!({ "bio": "x".repeat(1001) }),
        ] {
            let (status, _) = call(
                &app,
                Method::PUT,
                "/api/user/me",
                Some(&me.token),
                Some(bad.clone()),
            )
            .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
        }

        let settings = json!({ "newPlaces": true, "digest": "weekly" });
        let (status, body) = call(
            &app,
            Method::PUT,
            "/api/user/me/notifications",
            Some(&me.token),
            Some(json!({ "notification_settings": settings })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["notificationSettings"], settings);

        let (status, _) = call(&app, Method::GET, "/api/user/me", None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_spoken_languages() {
        let pool = require_db!();
        let app = test_app!(pool);
        let me = ts::user(&pool, "user");
        let conn = &mut pool.get().unwrap();
        let ids: Vec<Uuid> = (0..2)
            .map(|i| {
                diesel::insert_into(languages::table)
                    .values((
                        languages::name.eq(format!("Lang{i} {}", ts::unique())),
                        languages::symbol.eq("xx"),
                    ))
                    .returning(languages::id)
                    .get_result(conn)
                    .unwrap()
            })
            .collect();

        let (status, body) = call(
            &app,
            Method::POST,
            "/api/user/languages",
            Some(&me.token),
            Some(json!({ "languages_ids": ids })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");

        let path = format!("/api/user/{}/languages", me.id);
        let (status, body) = call(&app, Method::GET, &path, None, None).await;
        assert_eq!(status, StatusCode::OK, "languages are public");
        assert_eq!(body["languageIds"].as_array().unwrap().len(), 2);

        let (status, _) = call(
            &app,
            Method::DELETE,
            "/api/user/languages",
            Some(&me.token),
            Some(json!({ "language_id": ids[0] })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (_, body) = call(&app, Method::GET, &path, None, None).await;
        assert_eq!(body["languageIds"], json!([ids[1]]));

        let (status, _) = call(
            &app,
            Method::POST,
            "/api/user/languages",
            None,
            Some(json!({ "languages_ids": ids })),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    /// Without the GOOGLE_* env vars every OAuth endpoint answers 503.
    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_oauth_endpoints_without_configuration() {
        let pool = require_db!();
        let app = test_app!(pool);
        for (method, path, body) in [
            (Method::GET, "/api/user/oauth/google/login", None),
            (
                Method::GET,
                "/api/user/oauth/google/callback?code=c&state=s",
                None,
            ),
            (
                Method::POST,
                "/api/user/oauth/google/exchange",
                Some(json!({ "code": "c" })),
            ),
        ] {
            let (status, resp) = call(&app, method, path, None, body).await;
            assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{path}: {resp}");
        }
    }
}
