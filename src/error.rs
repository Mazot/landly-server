use actix_web::{
    http::StatusCode, HttpResponse,
    error::ResponseError
};
use thiserror::Error;
use diesel::result::{Error as DieselError, DatabaseErrorKind};
use diesel::r2d2::{Error as R2D2Error, PoolError};
use serde_json::{json, Value as JsonValue};
use uuid::Error as UuidError;

#[derive(Error, Debug)]
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
            },
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
