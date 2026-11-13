use crate::constants::env_key;
use actix_cors::Cors;
use actix_web::http;
use std::env;

/// The browser origin of the web client when `FRONTEND_ORIGIN` is unset (the Vite dev server).
/// Also the OAuth redirect target then (`user/controllers.rs`).
pub const DEV_FRONTEND_ORIGIN: &str = "http://localhost:5173";

/// `FRONTEND_ORIGIN`, without a trailing slash, or the dev default.
pub fn frontend_origin() -> String {
    env::var(env_key::FRONTEND_ORIGIN)
        .ok()
        .map(|origin| origin.trim().trim_end_matches('/').to_string())
        .filter(|origin| !origin.is_empty())
        .unwrap_or_else(|| DEV_FRONTEND_ORIGIN.to_string())
}

/// CORS for the web client: only `frontend_origin()` may call the API from a browser.
///
/// Must be the **outermost** middleware (`main.rs`): every response the browser can get —
/// including the 401 of the auth middleware, a 429 and a refused zstd body — needs the CORS
/// headers, or the browser hides it behind a CORS error and the client never sees the status.
pub fn cors() -> Cors {
    Cors::default()
        .allowed_origin(&frontend_origin())
        .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"])
        .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
        .allowed_header(http::header::CONTENT_TYPE)
        .max_age(3600)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::drivers::middlewares::auth::Authentication;
    use actix_web::{App, HttpResponse, http::StatusCode, test as actix_test, web};
    use serial_test::serial;

    fn set_origin(value: Option<&str>) {
        // SAFETY: every test touching FRONTEND_ORIGIN is #[serial(frontend_origin_env)].
        unsafe {
            match value {
                Some(value) => std::env::set_var("FRONTEND_ORIGIN", value),
                None => std::env::remove_var("FRONTEND_ORIGIN"),
            }
        }
    }

    #[test]
    #[serial(frontend_origin_env)]
    fn test_frontend_origin_defaults_to_the_dev_client_and_drops_a_trailing_slash() {
        set_origin(None);
        assert_eq!(frontend_origin(), DEV_FRONTEND_ORIGIN);
        set_origin(Some(" "));
        assert_eq!(frontend_origin(), DEV_FRONTEND_ORIGIN);
        set_origin(Some("https://landly.app/"));
        assert_eq!(frontend_origin(), "https://landly.app");
        set_origin(None);
    }

    /// The order of `main.rs`: CORS outside the auth middleware.
    macro_rules! app {
        () => {
            actix_test::init_service(
                App::new()
                    .wrap(Authentication)
                    .wrap(cors())
                    .route("/api/user/me", web::get().to(HttpResponse::Ok)),
            )
            .await
        };
    }

    #[actix_web::test]
    #[serial(frontend_origin_env, jwt_env)]
    async fn test_a_refused_token_still_carries_the_cors_headers() {
        crate::test_support::jwt_env();
        set_origin(Some("https://landly.app"));
        let app = app!();
        let request = actix_test::TestRequest::get()
            .uri("/api/user/me")
            .insert_header(("Origin", "https://landly.app"))
            .insert_header(("Authorization", "Bearer not-a-jwt"))
            .to_request();
        let response = actix_test::call_service(&app, request).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response
                .headers()
                .get("access-control-allow-origin")
                .unwrap(),
            "https://landly.app",
            "the browser must see the 401 to end the session"
        );
        set_origin(None);
    }

    #[actix_web::test]
    #[serial(frontend_origin_env)]
    async fn test_other_origins_get_no_cors_headers() {
        set_origin(Some("https://landly.app"));
        let app = app!();
        for origin in ["https://evil.example", "https://play.rust-lang.org"] {
            let request = actix_test::TestRequest::default()
                .method(actix_web::http::Method::OPTIONS)
                .uri("/api/user/me")
                .insert_header(("Origin", origin))
                .insert_header(("Access-Control-Request-Method", "GET"))
                .to_request();
            let response = actix_test::call_service(&app, request).await;
            assert!(
                response
                    .headers()
                    .get("access-control-allow-origin")
                    .is_none(),
                "{origin} must not be allowed"
            );
        }
        set_origin(None);
    }
}
