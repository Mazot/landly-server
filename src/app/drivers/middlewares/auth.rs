use crate::{error::AppError, utils::token};
use actix_web::{
    Error, HttpMessage, HttpResponse,
    body::EitherBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    http::Method,
};
use serde_json::json;
use std::{
    future::{Ready, ready},
    pin::Pin,
};
use uuid::Uuid;

const AUTH_HEADER: &str = "Authorization";
const BEARER: &str = "Bearer ";

pub struct Authentication;

impl<S, B> Transform<S, ServiceRequest> for Authentication
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = AuthenticationMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthenticationMiddleware { service }))
    }
}

pub struct AuthenticationMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;
    type Error = Error;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let need_auth = is_auth_required_route(&req);

        if !need_auth {
            let fut = self.service.call(req);

            return Box::pin(async move {
                let res = fut.await?.map_into_left_body();

                Ok(res)
            });
        }

        let is_user_authenticated = is_authenticated_user(&req);

        if is_user_authenticated.is_err() || !is_user_authenticated.as_ref().unwrap().0 {
            return Box::pin(async move {
                let (req, _res) = req.into_parts();

                let res = HttpResponse::Unauthorized().finish().map_into_right_body();
                let srv = ServiceResponse::new(req, res);

                Ok(srv)
            });
        }

        req.extensions_mut()
            .insert(is_user_authenticated.unwrap().1);

        // TODO: Мы должны проверить токен и если он валидный, то пропустить запрос дальше
        // Если токен не валидный, то вернуть ошибку 401
        // А в Request мы инсертнем user или user_id
        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?.map_into_left_body();

            Ok(res)
        })
    }
}

fn is_skip_auth_route(req: &ServiceRequest) -> bool {
    let method = req.method();
    if method == Method::OPTIONS {
        return true;
    }

    for route in SKIP_AUTH_ROUTES.iter() {
        if req.path().starts_with(route.path) && method == route.method {
            return true;
        }
    }

    false
}

fn is_auth_required_route(req: &ServiceRequest) -> bool {
    let method = req.method();

    AUTH_REQUIRED_ROUTES
        .iter()
        .any(|route| method == route.method && path_matches(route.path, req.path()))
}

/// Matches a request path against a route pattern.
///
/// - A pattern ending with `/` is a prefix match (e.g. `/api/images/upload/`
///   matches `/api/images/upload/<uuid>`).
/// - `{param}` segments match exactly one non-empty path segment
///   (e.g. `/api/organisation/update/{id}` matches `/api/organisation/update/<uuid>`).
/// - All other segments must match literally, and segment counts must be equal.
fn path_matches(pattern: &str, path: &str) -> bool {
    if pattern.ends_with('/') {
        return path.starts_with(pattern);
    }

    let mut pattern_segments = pattern.split('/');
    let mut path_segments = path.split('/');

    loop {
        match (pattern_segments.next(), path_segments.next()) {
            (None, None) => return true,
            (Some(expected), Some(actual)) => {
                let is_param = expected.starts_with('{') && expected.ends_with('}');
                if is_param {
                    if actual.is_empty() {
                        return false;
                    }
                } else if expected != actual {
                    return false;
                }
            }
            _ => return false,
        }
    }
}

fn is_authenticated_user(req: &ServiceRequest) -> Result<(bool, Uuid), AppError> {
    let token_opt = get_auth_token(req);
    let user_id = get_user_id_from_token(token_opt);

    match user_id {
        Ok(uid) => Ok((true, uid)),
        Err(e) => Err(e),
    }
}

fn get_auth_token(req: &ServiceRequest) -> Option<String> {
    req.headers()
        .get(AUTH_HEADER)
        .and_then(|v| v.to_str().ok())
        .filter(|s| s.starts_with(BEARER))
        .map(|s| s.trim_start_matches(BEARER).to_string())
}

fn get_user_id_from_token(token_opt: Option<String>) -> Result<Uuid, AppError> {
    if let Some(token) = token_opt {
        let jwt_claims = token::decode_token(&token)?;

        return Ok(jwt_claims.sub);
    }

    Err(AppError::Unauthorized(
        json!({ "error": "Authorization token is missing" }),
    ))
}

struct AuthRequiredRoute {
    path: &'static str,
    method: Method,
}

const AUTH_REQUIRED_ROUTES: [AuthRequiredRoute; 20] = [
    AuthRequiredRoute {
        path: "/api/user/languages",
        method: Method::POST,
    },
    AuthRequiredRoute {
        path: "/api/user/languages",
        method: Method::DELETE,
    },
    AuthRequiredRoute {
        path: "/api/user/me",
        method: Method::GET,
    },
    AuthRequiredRoute {
        path: "/api/user/me",
        method: Method::PUT,
    },
    AuthRequiredRoute {
        path: "/api/user/me/notifications",
        method: Method::PUT,
    },
    AuthRequiredRoute {
        path: "/api/corridor/create",
        method: Method::POST,
    },
    AuthRequiredRoute {
        path: "/api/corridor/list",
        method: Method::GET,
    },
    AuthRequiredRoute {
        path: "/api/corridor/set-default/{id}",
        method: Method::PUT,
    },
    AuthRequiredRoute {
        path: "/api/corridor/delete/{id}",
        method: Method::DELETE,
    },
    AuthRequiredRoute {
        path: "/api/corridor/stats/{id}",
        method: Method::GET,
    },
    AuthRequiredRoute {
        path: "/api/common/org_types",
        method: Method::POST,
    },
    AuthRequiredRoute {
        path: "/api/organisation/create",
        method: Method::POST,
    },
    AuthRequiredRoute {
        path: "/api/organisation/delete/{id}",
        method: Method::DELETE,
    },
    AuthRequiredRoute {
        path: "/api/organisation/update/{id}",
        method: Method::PUT,
    },
    AuthRequiredRoute {
        path: "/api/country-connection/create",
        method: Method::POST,
    },
    AuthRequiredRoute {
        path: "/api/country-connection/delete/{id}",
        method: Method::DELETE,
    },
    AuthRequiredRoute {
        path: "/api/country-connection/update/{id}",
        method: Method::PUT,
    },
    AuthRequiredRoute {
        path: "/api/images/upload/",
        method: Method::POST,
    },
    AuthRequiredRoute {
        path: "/api/images/delete/",
        method: Method::DELETE,
    },
    AuthRequiredRoute {
        path: "/api/images/set-primary/",
        method: Method::PUT,
    },
];

struct AuthSkipRoute {
    path: &'static str,
    method: Method,
}

const SKIP_AUTH_ROUTES: [AuthSkipRoute; 3] = [
    AuthSkipRoute {
        path: "/swagger-ui",
        method: Method::GET,
    },
    AuthSkipRoute {
        path: "/api-docs",
        method: Method::GET,
    },
    AuthSkipRoute {
        path: "/api/healthcheck",
        method: Method::GET,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_user_id_from_token_without_token() {
        let result = get_user_id_from_token(None);
        assert!(result.is_err());

        match result {
            Err(AppError::Unauthorized(_)) => (),
            _ => panic!("Expected Unauthorized error"),
        }
    }

    #[test]
    fn test_get_user_id_from_token_with_invalid_token() {
        unsafe {
            std::env::set_var("JWT_SECRET", "test_secret");
        }

        let result = get_user_id_from_token(Some("invalid_token".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_get_user_id_from_token_with_valid_token() {
        unsafe {
            std::env::set_var("JWT_SECRET", "test_secret");
            std::env::set_var("JWT_EXPIRATION", "3600");
        }

        let user_id = Uuid::new_v4();
        let token = crate::utils::token::generate_token(user_id).unwrap();

        let result = get_user_id_from_token(Some(token));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), user_id);
    }

    #[test]
    fn test_auth_constants() {
        assert_eq!(AUTH_HEADER, "Authorization");
        assert_eq!(BEARER, "Bearer ");
    }

    #[test]
    fn test_auth_required_routes_count() {
        assert_eq!(AUTH_REQUIRED_ROUTES.len(), 20);
    }

    /// `PUT /api/user/me` must not accidentally protect (or be shadowed by)
    /// `PUT /api/user/me/notifications` — they are separate exact patterns.
    #[test]
    fn test_user_me_routes_are_exact() {
        assert!(path_matches("/api/user/me", "/api/user/me"));
        assert!(!path_matches("/api/user/me", "/api/user/me/notifications"));
        assert!(path_matches(
            "/api/user/me/notifications",
            "/api/user/me/notifications"
        ));
    }

    /// Every entry in AUTH_REQUIRED_ROUTES must match a realistic request path.
    /// Guards against patterns (e.g. containing a literal `{id}`) that can never
    /// match real traffic and would silently disable auth for that route.
    #[test]
    fn test_every_auth_required_route_matches_a_real_path() {
        let id = Uuid::new_v4().to_string();

        for route in AUTH_REQUIRED_ROUTES.iter() {
            let real_path = if route.path.ends_with('/') {
                format!("{}{}", route.path, id)
            } else {
                route.path.replace("{id}", &id)
            };

            assert!(
                path_matches(route.path, &real_path),
                "route pattern `{}` does not match real path `{}`",
                route.path,
                real_path
            );
        }
    }

    #[test]
    fn test_path_matches_organisation_update_with_uuid() {
        let id = Uuid::new_v4();
        assert!(path_matches(
            "/api/organisation/update/{id}",
            &format!("/api/organisation/update/{}", id)
        ));
        assert!(path_matches(
            "/api/organisation/delete/{id}",
            &format!("/api/organisation/delete/{}", id)
        ));
    }

    #[test]
    fn test_path_matches_country_connection_with_uuid() {
        let id = Uuid::new_v4();
        assert!(path_matches(
            "/api/country-connection/update/{id}",
            &format!("/api/country-connection/update/{}", id)
        ));
        assert!(path_matches(
            "/api/country-connection/delete/{id}",
            &format!("/api/country-connection/delete/{}", id)
        ));
    }

    #[test]
    fn test_path_matches_exact_route() {
        assert!(path_matches(
            "/api/organisation/create",
            "/api/organisation/create"
        ));
        assert!(path_matches("/api/user/languages", "/api/user/languages"));
        assert!(path_matches(
            "/api/common/org_types",
            "/api/common/org_types"
        ));
    }

    #[test]
    fn test_path_matches_prefix_route() {
        let id = Uuid::new_v4();
        assert!(path_matches(
            "/api/images/upload/",
            &format!("/api/images/upload/{}", id)
        ));
        assert!(path_matches(
            "/api/images/delete/",
            &format!("/api/images/delete/{}", id)
        ));
        assert!(path_matches(
            "/api/images/set-primary/",
            &format!("/api/images/set-primary/{}", id)
        ));
    }

    #[test]
    fn test_path_matches_rejects_different_paths() {
        assert!(!path_matches(
            "/api/organisation/update/{id}",
            "/api/organisation/update"
        ));
        assert!(!path_matches(
            "/api/organisation/update/{id}",
            "/api/organisation/update/"
        ));
        assert!(!path_matches(
            "/api/organisation/create",
            "/api/organisation/create/extra"
        ));
        assert!(!path_matches("/api/user/languages", "/api/user/language"));
        assert!(!path_matches(
            "/api/organisation/update/{id}",
            "/api/organisation/fetch/123"
        ));
    }

    #[test]
    fn test_skip_auth_routes_count() {
        assert_eq!(SKIP_AUTH_ROUTES.len(), 3);
    }

    #[test]
    fn test_auth_required_routes_contains_organisation_create() {
        let has_route = AUTH_REQUIRED_ROUTES
            .iter()
            .any(|r| r.path == "/api/organisation/create" && r.method == Method::POST);
        assert!(has_route);
    }

    #[test]
    fn test_auth_required_routes_contains_country_connection_create() {
        let has_route = AUTH_REQUIRED_ROUTES
            .iter()
            .any(|r| r.path == "/api/country-connection/create" && r.method == Method::POST);
        assert!(has_route);
    }

    #[test]
    fn test_auth_required_routes_contains_images_upload() {
        let has_route = AUTH_REQUIRED_ROUTES
            .iter()
            .any(|r| r.path == "/api/images/upload/" && r.method == Method::POST);
        assert!(has_route);
    }

    #[test]
    fn test_auth_required_routes_contains_images_delete() {
        let has_route = AUTH_REQUIRED_ROUTES
            .iter()
            .any(|r| r.path == "/api/images/delete/" && r.method == Method::DELETE);
        assert!(has_route);
    }

    #[test]
    fn test_auth_required_routes_contains_images_set_primary() {
        let has_route = AUTH_REQUIRED_ROUTES
            .iter()
            .any(|r| r.path == "/api/images/set-primary/" && r.method == Method::PUT);
        assert!(has_route);
    }

    #[test]
    fn test_skip_auth_routes_contains_swagger() {
        let has_route = SKIP_AUTH_ROUTES
            .iter()
            .any(|r| r.path == "/swagger-ui" && r.method == Method::GET);
        assert!(has_route);
    }

    #[test]
    fn test_skip_auth_routes_contains_healthcheck() {
        let has_route = SKIP_AUTH_ROUTES
            .iter()
            .any(|r| r.path == "/api/healthcheck" && r.method == Method::GET);
        assert!(has_route);
    }
}
