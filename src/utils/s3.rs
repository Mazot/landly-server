use aws_config::BehaviorVersion;
use aws_sdk_s3::{
    Client, Config,
    config::{Credentials, Region},
};
use std::env;

use crate::error::AppError;
use crate::utils::storage::{StorageFuture, StorageService};

/// Thin wrapper around the AWS SDK S3 client that is compatible with both
/// Amazon S3 and Cloudflare R2 (via `force_path_style = true`).
///
/// Configuration is driven entirely by environment variables:
///
/// | Variable            | Required | Description                                         |
/// |---------------------|----------|-----------------------------------------------------|
/// | `S3_ENDPOINT_URL`   | yes      | Full endpoint URL (e.g. `https://<id>.r2.cloudflarestorage.com`) |
/// | `S3_ACCESS_KEY_ID`  | yes      | Access key / R2 token ID                            |
/// | `S3_SECRET_ACCESS_KEY` | yes   | Secret key / R2 token secret                        |
/// | `S3_BUCKET`         | yes      | Bucket name                                         |
/// | `S3_REGION`         | no       | Region string (defaults to `"auto"`)                |
/// | `S3_PUBLIC_URL`     | no       | Base URL for public object access.  When absent the |
/// |                     |          | value is derived as `{endpoint}/{bucket}`.          |
pub struct S3ClientWrapper {
    client: Client,
    bucket: String,
    /// Base URL used to construct public object URLs, e.g. `https://cdn.example.com`.
    /// The full URL for a key is `{public_url_base}/{key}`.
    public_url_base: String,
}

impl S3ClientWrapper {
    /// Construct a new wrapper from environment variables.
    ///
    /// This is intentionally **synchronous**: building the SDK config and
    /// constructing the `Client` from it involves no network I/O.
    pub fn new() -> Result<Self, AppError> {
        let endpoint_url = env::var("S3_ENDPOINT_URL").map_err(|_| {
            log::error!("S3_ENDPOINT_URL environment variable is not set");
            AppError::InternalServerError
        })?;

        let access_key = env::var("S3_ACCESS_KEY_ID").map_err(|_| {
            log::error!("S3_ACCESS_KEY_ID environment variable is not set");
            AppError::InternalServerError
        })?;

        let secret_key = env::var("S3_SECRET_ACCESS_KEY").map_err(|_| {
            log::error!("S3_SECRET_ACCESS_KEY environment variable is not set");
            AppError::InternalServerError
        })?;

        let region = env::var("S3_REGION").unwrap_or_else(|_| "auto".to_string());

        let bucket = env::var("S3_BUCKET").map_err(|_| {
            log::error!("S3_BUCKET environment variable is not set");
            AppError::InternalServerError
        })?;

        // Derive a sensible default for the public base URL when the explicit
        // override is not provided.
        let public_url_base = env::var("S3_PUBLIC_URL")
            .unwrap_or_else(|_| format!("{}/{}", endpoint_url.trim_end_matches('/'), bucket));

        let credentials = Credentials::new(access_key, secret_key, None, None, "landly-custom");

        let config = Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(region))
            .endpoint_url(&endpoint_url)
            .credentials_provider(credentials)
            // Required for Cloudflare R2 and MinIO which use path-style addressing.
            .force_path_style(true)
            .build();

        let client = Client::from_conf(config);

        Ok(Self {
            client,
            bucket,
            public_url_base,
        })
    }

    // ------------------------------------------------------------------
    // Lower-level helpers kept for direct use where the full trait is not
    // needed (e.g. generating presigned URLs).
    // ------------------------------------------------------------------

    /// Generate a short-lived presigned GET URL for `key`.
    pub async fn get_presigned_url(
        &self,
        key: &str,
        expires_in_secs: u64,
    ) -> Result<String, AppError> {
        let presigned = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(
                aws_sdk_s3::presigning::PresigningConfig::expires_in(
                    std::time::Duration::from_secs(expires_in_secs),
                )
                .map_err(|e| {
                    log::error!("Presigning config error: {:?}", e);
                    AppError::InternalServerError
                })?,
            )
            .await
            .map_err(|e| {
                log::error!("Presigning error: {:?}", e);
                AppError::InternalServerError
            })?;

        Ok(presigned.uri().to_string())
    }

    /// Download raw bytes for `key`.
    pub async fn download(&self, key: &str) -> Result<Vec<u8>, AppError> {
        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                log::error!("S3 download error: {:?}", e);
                AppError::InternalServerError
            })?;

        let body = response
            .body
            .collect()
            .await
            .map_err(|e| {
                log::error!("S3 body collection error: {:?}", e);
                AppError::InternalServerError
            })?
            .into_bytes();

        Ok(body.to_vec())
    }

    pub fn get_bucket(&self) -> &str {
        &self.bucket
    }
}

// ---------------------------------------------------------------------------
// StorageService implementation
// ---------------------------------------------------------------------------

impl StorageService for S3ClientWrapper {
    fn upload<'a>(
        &'a self,
        key: &'a str,
        data: Vec<u8>,
        content_type: &'a str,
    ) -> StorageFuture<'a, ()> {
        Box::pin(async move {
            self.client
                .put_object()
                .bucket(&self.bucket)
                .key(key)
                .body(data.into())
                .content_type(content_type)
                .send()
                .await
                .map_err(|e| {
                    log::error!("S3 upload error for key '{}': {:?}", key, e);
                    AppError::InternalServerError
                })?;

            Ok(())
        })
    }

    fn delete<'a>(&'a self, key: &'a str) -> StorageFuture<'a, ()> {
        Box::pin(async move {
            self.client
                .delete_object()
                .bucket(&self.bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| {
                    log::error!("S3 delete error for key '{}': {:?}", key, e);
                    AppError::InternalServerError
                })?;

            Ok(())
        })
    }

    fn get_public_url(&self, key: &str) -> String {
        format!(
            "{}/{}",
            self.public_url_base.trim_end_matches('/'),
            key.trim_start_matches('/')
        )
    }
}

impl Clone for S3ClientWrapper {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            bucket: self.bucket.clone(),
            public_url_base: self.public_url_base.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    const VARS: [&str; 6] = [
        "S3_ENDPOINT_URL",
        "S3_ACCESS_KEY_ID",
        "S3_SECRET_ACCESS_KEY",
        "S3_BUCKET",
        "S3_REGION",
        "S3_PUBLIC_URL",
    ];

    /// Sets the S3_* vars (None = unset). These tests hold the `jwt_env` lock
    /// because `DiContainer::new` in the DB-backed tests reads the same vars.
    fn s3_env(values: [Option<&str>; 6]) {
        for (key, value) in VARS.iter().zip(values) {
            // SAFETY: serialized by #[serial(jwt_env)].
            unsafe {
                match value {
                    Some(v) => env::set_var(key, v),
                    None => env::remove_var(key),
                }
            }
        }
    }

    fn unreachable_storage(public_url: Option<&str>) -> S3ClientWrapper {
        // Port 1 refuses connections immediately: no real network traffic.
        s3_env([
            Some("http://127.0.0.1:1"),
            Some("key"),
            Some("secret"),
            Some("photos"),
            None,
            public_url,
        ]);
        let storage = S3ClientWrapper::new().expect("all required vars are set");
        s3_env([None; 6]);
        storage
    }

    #[test]
    #[serial(jwt_env)]
    fn test_new_requires_every_mandatory_var() {
        for missing in 0..4 {
            let mut values = [
                Some("http://127.0.0.1:1"),
                Some("key"),
                Some("secret"),
                Some("photos"),
                None,
                None,
            ];
            values[missing] = None;
            s3_env(values);
            assert!(S3ClientWrapper::new().is_err(), "{} missing", VARS[missing]);
        }
        s3_env([None; 6]);
    }

    #[test]
    #[serial(jwt_env)]
    fn test_public_url_defaults_to_endpoint_and_bucket() {
        let storage = unreachable_storage(None);
        assert_eq!(storage.get_bucket(), "photos");
        assert_eq!(
            storage.get_public_url("images/a.png"),
            "http://127.0.0.1:1/photos/images/a.png"
        );
        assert_eq!(
            storage.clone().get_public_url("x"),
            storage.get_public_url("x")
        );
    }

    #[test]
    #[serial(jwt_env)]
    fn test_public_url_override() {
        let storage = unreachable_storage(Some("https://cdn.example.test/media/"));
        assert_eq!(
            storage.get_public_url("images/a.png"),
            "https://cdn.example.test/media/images/a.png"
        );
    }

    /// Presigning is computed locally (no request is sent).
    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_presigned_url_is_signed_for_the_key() {
        let storage = unreachable_storage(None);
        let url = storage
            .get_presigned_url("images/a.png", 300)
            .await
            .unwrap();
        assert!(
            url.starts_with("http://127.0.0.1:1/photos/images/a.png?"),
            "{url}"
        );
        assert!(
            url.contains("X-Amz-Signature=") && url.contains("X-Amz-Expires=300"),
            "{url}"
        );
    }

    #[actix_web::test]
    #[serial(jwt_env)]
    async fn test_network_failures_become_internal_server_errors() {
        let storage = unreachable_storage(None);
        assert!(matches!(
            storage.upload("k", b"data".to_vec(), "image/png").await,
            Err(AppError::InternalServerError)
        ));
        assert!(matches!(
            storage.delete("k").await,
            Err(AppError::InternalServerError)
        ));
        assert!(matches!(
            storage.download("k").await,
            Err(AppError::InternalServerError)
        ));
    }
}
