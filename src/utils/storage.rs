use crate::error::AppError;
use std::{future::Future, pin::Pin};

/// Type alias for a boxed, pinned, Send future — used for dyn-compatible async trait methods.
pub type StorageFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, AppError>> + Send + 'a>>;

/// Abstract interface for object storage (S3, R2, MinIO, local disk, …).
///
/// Methods return boxed futures so the trait is object-safe and can be used as
/// `Arc<dyn StorageService>` / `Box<dyn StorageService>`.
pub trait StorageService: Send + Sync + 'static {
    /// Upload raw bytes under the given `key` with the supplied MIME `content_type`.
    fn upload<'a>(
        &'a self,
        key: &'a str,
        data: Vec<u8>,
        content_type: &'a str,
    ) -> StorageFuture<'a, ()>;

    /// Permanently remove the object identified by `key`.
    fn delete<'a>(&'a self, key: &'a str) -> StorageFuture<'a, ()>;

    /// Return the publicly-accessible URL for `key`.
    ///
    /// This is a synchronous call because the URL is derived from configuration
    /// (e.g. `S3_PUBLIC_URL` env var) and does not require a network round-trip.
    fn get_public_url(&self, key: &str) -> String;
}

// ---------------------------------------------------------------------------
// NoOp implementation — used when no storage backend is configured.
// All mutating operations return an error; `get_public_url` echoes the key.
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
pub struct NoOpStorageService;

impl StorageService for NoOpStorageService {
    fn upload<'a>(
        &'a self,
        _key: &'a str,
        _data: Vec<u8>,
        _content_type: &'a str,
    ) -> StorageFuture<'a, ()> {
        Box::pin(async {
            log::error!("NoOpStorageService: upload called — storage backend is not configured");
            Err(AppError::InternalServerError)
        })
    }

    fn delete<'a>(&'a self, _key: &'a str) -> StorageFuture<'a, ()> {
        Box::pin(async {
            log::error!("NoOpStorageService: delete called — storage backend is not configured");
            Err(AppError::InternalServerError)
        })
    }

    fn get_public_url(&self, key: &str) -> String {
        key.to_string()
    }
}
