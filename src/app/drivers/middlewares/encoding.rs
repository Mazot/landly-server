//! Refuses request bodies sent with `Content-Encoding: zstd`.
//!
//! actix-web transparently decompresses `gzip`/`deflate`/`br`/`zstd` request
//! bodies before the JSON extractor sees them. The zstd binding that actix-http
//! 3.x ships (zstd 0.13 / zstd-safe 7) has open advisories without a patched
//! release (`.cargo/audit.toml`), and no client of this API sends zstd, so the
//! decoder is made unreachable instead of waiting for the upstream bump.

use actix_web::{
    Error,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http::header::CONTENT_ENCODING,
    middleware::Next,
};

use crate::error::AppError;

/// `middleware::from_fn` handler: 422 for a zstd-encoded request body.
pub async fn reject_zstd_requests(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let zstd = req
        .headers()
        .get_all(CONTENT_ENCODING)
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .any(|coding| coding.trim().eq_ignore_ascii_case("zstd"));

    if zstd {
        return Err(AppError::UnprocessableEntity(serde_json::json!({
            "error": "Content-Encoding zstd is not supported; send the body uncompressed or gzip"
        }))
        .into());
    }

    next.call(req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, HttpResponse, http::StatusCode, middleware::from_fn, test, web};

    async fn app_call(encoding: Option<&str>) -> StatusCode {
        let app = test::init_service(App::new().wrap(from_fn(reject_zstd_requests)).route(
            "/echo",
            web::post().to(|| async { HttpResponse::Ok().finish() }),
        ))
        .await;
        let mut req = test::TestRequest::post().uri("/echo");
        if let Some(value) = encoding {
            req = req.insert_header((CONTENT_ENCODING, value));
        }
        // An error returned by the middleware surfaces as `Err`, not a response.
        match test::try_call_service(&app, req.to_request()).await {
            Ok(res) => res.status(),
            Err(e) => e.as_response_error().status_code(),
        }
    }

    #[actix_web::test]
    async fn test_zstd_request_bodies_are_refused() {
        for value in ["zstd", "ZSTD", "gzip, zstd", " zstd "] {
            assert_eq!(
                app_call(Some(value)).await,
                StatusCode::UNPROCESSABLE_ENTITY,
                "{value}"
            );
        }
    }

    #[actix_web::test]
    async fn test_other_encodings_pass_through() {
        for value in [None, Some("gzip"), Some("br"), Some("identity")] {
            assert_eq!(app_call(value).await, StatusCode::OK, "{value:?}");
        }
    }
}
