
use actix_web::{http::StatusCode, HttpResponse};
use thiserror::Error;
use serde_json::Value as JsonValue;

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

impl actix_web::error::ResponseError for AppError {
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
