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

    // TODO: improve this logic
    for route in SKIP_AUTH_ROUTES.iter() {
        if req.path().starts_with(route.path) && method == &route.method {
            return true;
        }
    }

    false
}

fn is_auth_required_route(req: &ServiceRequest) -> bool {
    let method = req.method();

    // TODO: Надо заменять {id} в path
    for route in AUTH_REQUIRED_ROUTES.iter() {
        if req.path().starts_with(route.path) && method == &route.method {
            return true;
        }
    }

    false
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
    if let Some(auth_header_value) = req.headers().get(AUTH_HEADER) {
        if let Ok(auth_str) = auth_header_value.to_str() {
            if auth_str.starts_with(BEARER) {
                let token = auth_str.trim_start_matches(BEARER).to_string();
                return Some(token);
            }
        }
    }

    None
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

const AUTH_REQUIRED_ROUTES: [AuthRequiredRoute; 7] = [
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
        assert_eq!(AUTH_REQUIRED_ROUTES.len(), 7);
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
