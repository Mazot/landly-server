use actix_web::{HttpResponse, error::ResponseError, http::StatusCode};
use bcrypt::BcryptError;
use diesel::r2d2::{Error as R2D2Error, PoolError};
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use jsonwebtoken::errors::{Error as JwtError, ErrorKind as JwtErrorKind};
use redis::{ErrorKind as RedisErrorKind, RedisError};
use serde_json::{Value as JsonValue, json};
use std::env::VarError;
use thiserror::Error;
use utoipa::ToSchema;
use uuid::Error as UuidError;

#[derive(Error, ToSchema, Debug)]
pub enum AppError {
    // 401
    #[error("Unauthorized: {}", _0)]
    Unauthorized(JsonValue),

    // 403
    #[error("Forbidden: {}", _0)]
    Forbidden(JsonValue),

    // 404
    #[error("Not Found: {}", _0)]
    NotFound(JsonValue),

    // 422
    #[error("Unprocessable Entity: {}", _0)]
    UnprocessableEntity(JsonValue),

    // 500
    #[error("Internal Server Error")]
    InternalServerError,
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match *self {
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            AppError::Unauthorized(msg) => HttpResponse::Unauthorized().json(msg),
            AppError::Forbidden(msg) => HttpResponse::Forbidden().json(msg),
            AppError::NotFound(msg) => HttpResponse::NotFound().json(msg),
            AppError::UnprocessableEntity(msg) => HttpResponse::UnprocessableEntity().json(msg),
            AppError::InternalServerError => {
                HttpResponse::InternalServerError().json("Internal Server Error")
            }
        }
    }
}

impl From<DieselError> for AppError {
    fn from(error: DieselError) -> Self {
        match error {
            DieselError::DatabaseError(kind, info) => {
                if let DatabaseErrorKind::UniqueViolation = kind {
                    let message = info.details().unwrap_or_else(|| info.message()).to_string();
                    AppError::UnprocessableEntity(json!({ "error": message }))
                } else {
                    AppError::InternalServerError
                }
            }
            DieselError::NotFound => {
                AppError::NotFound(json!({ "error": "requested record was not found" }))
            }
            _ => AppError::InternalServerError,
        }
    }
}

impl From<R2D2Error> for AppError {
    fn from(error: R2D2Error) -> Self {
        match error {
            R2D2Error::ConnectionError(_) => AppError::InternalServerError,
            R2D2Error::QueryError(_) => AppError::InternalServerError,
        }
    }
}

impl From<PoolError> for AppError {
    fn from(_error: PoolError) -> Self {
        AppError::InternalServerError
    }
}

impl From<UuidError> for AppError {
    fn from(value: UuidError) -> Self {
        AppError::UnprocessableEntity(json!({
            "error": "Invalid UUID format",
            "details": value.to_string()
        }))
    }
}

impl From<RedisError> for AppError {
    fn from(error: RedisError) -> Self {
        match error.kind() {
            // TODO: Handle specific Redis error kinds as needed
            RedisErrorKind::IoError => AppError::InternalServerError,
            RedisErrorKind::ResponseError => AppError::InternalServerError,
            _ => AppError::InternalServerError,
        }
    }
}

impl From<VarError> for AppError {
    fn from(value: VarError) -> Self {
        match value {
            VarError::NotPresent => AppError::InternalServerError,
            VarError::NotUnicode(_) => AppError::InternalServerError,
        }
    }
}

impl From<BcryptError> for AppError {
    fn from(_err: BcryptError) -> Self {
        AppError::InternalServerError
    }
}

impl From<JwtError> for AppError {
    fn from(err: JwtError) -> Self {
        match err.kind() {
            JwtErrorKind::InvalidToken => {
                AppError::Unauthorized(json!({ "error": "Invalid Token" }))
            }
            JwtErrorKind::InvalidIssuer => {
                AppError::Unauthorized(json!({ "error": "Invalid Issuer" }))
            }
            _ => AppError::Unauthorized(
                json!({ "error": "An issue was found with the token provided" }),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;

    #[test]
    fn test_app_error_unauthorized_status() {
        let error = AppError::Unauthorized(json!({ "error": "unauthorized" }));
        assert_eq!(error.status_code(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_app_error_forbidden_status() {
        let error = AppError::Forbidden(json!({ "error": "forbidden" }));
        assert_eq!(error.status_code(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_app_error_not_found_status() {
        let error = AppError::NotFound(json!({ "error": "not found" }));
        assert_eq!(error.status_code(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_app_error_unprocessable_entity_status() {
        let error = AppError::UnprocessableEntity(json!({ "error": "invalid data" }));
        assert_eq!(error.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn test_app_error_internal_server_error_status() {
        let error = AppError::InternalServerError;
        assert_eq!(error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_diesel_not_found_conversion() {
        let diesel_error = DieselError::NotFound;
        let app_error: AppError = diesel_error.into();

        match app_error {
            AppError::NotFound(_) => (),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_uuid_error_conversion() {
        use uuid::Uuid;
        let parse_result = Uuid::parse_str("not-a-valid-uuid");
        assert!(parse_result.is_err());

        let uuid_error = parse_result.unwrap_err();
        let app_error: AppError = uuid_error.into();

        match app_error {
            AppError::UnprocessableEntity(json) => {
                assert!(json.get("error").is_some());
            }
            _ => panic!("Expected UnprocessableEntity error"),
        }
    }

    #[test]
    fn test_bcrypt_error_conversion() {
        use bcrypt::BcryptError;
        let bcrypt_error = BcryptError::CostNotAllowed(1);
        let app_error: AppError = bcrypt_error.into();

        match app_error {
            AppError::InternalServerError => (),
            _ => panic!("Expected InternalServerError"),
        }
    }

    #[test]
    fn test_var_error_not_present_conversion() {
        let var_error = VarError::NotPresent;
        let app_error: AppError = var_error.into();

        match app_error {
            AppError::InternalServerError => (),
            _ => panic!("Expected InternalServerError"),
        }
    }

    #[test]
    fn test_error_response_unauthorized() {
        let error = AppError::Unauthorized(json!({ "message": "test" }));
        let response = error.error_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_error_response_forbidden() {
        let error = AppError::Forbidden(json!({ "message": "test" }));
        let response = error.error_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_error_response_not_found() {
        let error = AppError::NotFound(json!({ "message": "test" }));
        let response = error.error_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_error_response_unprocessable_entity() {
        let error = AppError::UnprocessableEntity(json!({ "message": "test" }));
        let response = error.error_response();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn test_error_response_internal_server_error() {
        let error = AppError::InternalServerError;
        let response = error.error_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_error_display_unauthorized() {
        let error = AppError::Unauthorized(json!({ "message": "test" }));
        let display = format!("{}", error);
        assert!(display.contains("Unauthorized"));
    }

    #[test]
    fn test_error_display_internal_server_error() {
        let error = AppError::InternalServerError;
        let display = format!("{}", error);
        assert_eq!(display, "Internal Server Error");
    }
}
