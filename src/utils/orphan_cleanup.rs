//! Finds (and optionally deletes) image objects in storage that no `images`
//! row points to (audit M7).
//!
//! Objects are orphaned when an organisation is deleted while its S3 cleanup
//! failed, when an upload reached S3 but the database insert did not, and —
//! before this audit — on every organisation delete. The bucket allows
//! anonymous reads, so an orphan stays reachable by its old URL.
//!
//! An upload writes the object *before* the row, so a young object without a
//! row may be an upload still in flight; only objects older than a minimum
//! age (24 h by default) are ever treated as orphans.
//!
//! The pass lives in the library so it is testable; the `orphan_images`
//! binary in `scripts/crates` is only argument parsing around [`sweep`].

use crate::data::schema::images;
use crate::error::AppError;
use crate::utils::{db::DbPool, storage::StorageService};
use diesel::prelude::*;
use std::{collections::HashSet, future::Future, pin::Pin};

/// Where the app writes organisation images (`images/{org_id}/{uuid}.{ext}`).
pub const IMAGE_PREFIX: &str = "images/";

pub type ListFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vec<StoredObject>, AppError>> + Send + 'a>>;

/// One object found in the bucket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredObject {
    pub key: String,
    /// Last modification, Unix seconds.
    pub last_modified_unix: i64,
}

/// Lists a bucket. Separate from `StorageService` because only this
/// maintenance pass needs it.
pub trait ObjectLister: Send + Sync {
    /// Every object whose key starts with `prefix`.
    fn list_objects<'a>(&'a self, prefix: &'a str) -> ListFuture<'a>;
}

/// Keys of every image the database knows about.
pub fn known_image_keys(pool: &DbPool) -> Result<HashSet<String>, AppError> {
    let conn = &mut pool.get()?;
    let keys = images::table.select(images::s3_key).load::<String>(conn)?;

    Ok(keys.into_iter().collect())
}

/// Objects with no row that were last modified at or before `cutoff_unix`.
/// Sorted, so reports are stable.
pub fn find_orphans(
    objects: &[StoredObject],
    known: &HashSet<String>,
    cutoff_unix: i64,
) -> Vec<String> {
    let mut orphans: Vec<String> = objects
        .iter()
        .filter(|o| !known.contains(&o.key) && o.last_modified_unix <= cutoff_unix)
        .map(|o| o.key.clone())
        .collect();
    orphans.sort();
    orphans
}

#[derive(Debug, Clone)]
pub struct SweepOptions {
    /// Skip objects younger than this many seconds (in-flight uploads).
    pub min_age_secs: i64,
    /// Actually delete; otherwise only report.
    pub delete: bool,
    /// Allow deleting when the database knows no images at all. Off by
    /// default: an empty set usually means the wrong database, and every
    /// object would look orphaned.
    pub allow_empty_db: bool,
}

impl Default for SweepOptions {
    fn default() -> Self {
        Self {
            min_age_secs: 24 * 60 * 60,
            delete: false,
            allow_empty_db: false,
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SweepReport {
    /// Objects found under the image prefix.
    pub scanned: usize,
    /// Objects whose key is in the database.
    pub in_use: usize,
    /// Rowless objects skipped because they are too young.
    pub too_new: usize,
    /// Rowless objects older than the minimum age.
    pub orphans: Vec<String>,
    pub deleted: usize,
    pub failed: usize,
}

/// One pass: list the bucket, compare with `known`, optionally delete.
pub async fn sweep<S>(
    store: &S,
    known: &HashSet<String>,
    now_unix: i64,
    options: &SweepOptions,
) -> anyhow::Result<SweepReport>
where
    S: ObjectLister + StorageService,
{
    let objects = store
        .list_objects(IMAGE_PREFIX)
        .await
        .map_err(|e| anyhow::anyhow!("listing the bucket failed: {e:?}"))?;

    if options.delete && known.is_empty() && !objects.is_empty() && !options.allow_empty_db {
        anyhow::bail!(
            "the database knows no images but the bucket holds {} objects — probably the wrong \
             DATABASE_URL; refusing to delete (pass --allow-empty-db if that is really intended)",
            objects.len()
        );
    }

    let cutoff = now_unix - options.min_age_secs;
    let orphans = find_orphans(&objects, known, cutoff);
    // Everything without a row, whatever its age.
    let rowless = find_orphans(&objects, known, i64::MAX).len();
    let mut report = SweepReport {
        scanned: objects.len(),
        in_use: objects.len() - rowless,
        too_new: rowless - orphans.len(),
        orphans,
        ..Default::default()
    };

    if options.delete {
        for key in &report.orphans {
            match store.delete(key).await {
                Ok(()) => report.deleted += 1,
                Err(e) => {
                    report.failed += 1;
                    log::error!("could not delete orphaned object {key}: {e:?}");
                }
            }
        }
    }

    Ok(report)
}

/// Runs a future to completion on a fresh actix runtime — for binaries that
/// have no async `main` of their own.
pub fn block_on<F: Future>(future: F) -> F::Output {
    actix_web::rt::System::new().block_on(future)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::storage::StorageFuture;
    use std::sync::Mutex;

    const NOW: i64 = 1_800_000_000;
    const DAY: i64 = 24 * 60 * 60;

    fn object(key: &str, age_secs: i64) -> StoredObject {
        StoredObject {
            key: key.to_string(),
            last_modified_unix: NOW - age_secs,
        }
    }

    /// An in-memory bucket.
    struct FakeStore {
        objects: Vec<StoredObject>,
        deleted: Mutex<Vec<String>>,
        failing: Vec<String>,
    }

    impl FakeStore {
        fn new(objects: Vec<StoredObject>) -> Self {
            Self {
                objects,
                deleted: Mutex::new(vec![]),
                failing: vec![],
            }
        }
    }

    impl ObjectLister for FakeStore {
        fn list_objects<'a>(&'a self, prefix: &'a str) -> ListFuture<'a> {
            Box::pin(async move {
                Ok(self
                    .objects
                    .iter()
                    .filter(|o| o.key.starts_with(prefix))
                    .cloned()
                    .collect())
            })
        }
    }

    impl StorageService for FakeStore {
        fn upload<'a>(&'a self, _: &'a str, _: Vec<u8>, _: &'a str) -> StorageFuture<'a, ()> {
            unimplemented!()
        }

        fn delete<'a>(&'a self, key: &'a str) -> StorageFuture<'a, ()> {
            Box::pin(async move {
                if self.failing.iter().any(|k| k == key) {
                    return Err(AppError::InternalServerError);
                }
                self.deleted.lock().unwrap().push(key.to_string());
                Ok(())
            })
        }

        fn get_public_url(&self, key: &str) -> String {
            key.to_string()
        }
    }

    fn known(keys: &[&str]) -> HashSet<String> {
        keys.iter().map(|k| k.to_string()).collect()
    }

    #[test]
    fn test_find_orphans_skips_known_and_young_objects() {
        let objects = [
            object("images/a/used.png", 5 * DAY),
            object("images/a/orphan-old.png", 3 * DAY),
            object("images/a/orphan-new.png", 60),
            object("images/a/boundary.png", DAY),
        ];
        let orphans = find_orphans(&objects, &known(&["images/a/used.png"]), NOW - DAY);

        assert_eq!(
            orphans,
            vec!["images/a/boundary.png", "images/a/orphan-old.png"]
        );
    }

    #[actix_web::test]
    async fn test_sweep_reports_without_deleting_by_default() {
        let store = FakeStore::new(vec![
            object("images/o/used.png", 9 * DAY),
            object("images/o/orphan.png", 9 * DAY),
            object("images/o/fresh.png", 120),
            object("other/not-ours.png", 9 * DAY),
        ]);
        let report = sweep(
            &store,
            &known(&["images/o/used.png"]),
            NOW,
            &SweepOptions::default(),
        )
        .await
        .unwrap();

        assert_eq!(
            report,
            SweepReport {
                scanned: 3,
                in_use: 1,
                too_new: 1,
                orphans: vec!["images/o/orphan.png".to_string()],
                deleted: 0,
                failed: 0,
            },
            "only the images/ prefix is looked at"
        );
        assert!(
            store.deleted.lock().unwrap().is_empty(),
            "dry run deletes nothing"
        );
    }

    #[actix_web::test]
    async fn test_sweep_deletes_only_old_orphans_and_counts_failures() {
        let mut store = FakeStore::new(vec![
            object("images/o/used.png", 9 * DAY),
            object("images/o/orphan-1.png", 9 * DAY),
            object("images/o/orphan-2.png", 9 * DAY),
            object("images/o/fresh.png", 120),
        ]);
        store.failing = vec!["images/o/orphan-2.png".to_string()];
        let options = SweepOptions {
            delete: true,
            ..Default::default()
        };

        let report = sweep(&store, &known(&["images/o/used.png"]), NOW, &options)
            .await
            .unwrap();

        assert_eq!((report.deleted, report.failed, report.too_new), (1, 1, 1));
        assert_eq!(
            *store.deleted.lock().unwrap(),
            vec!["images/o/orphan-1.png"]
        );
    }

    /// A wrong DATABASE_URL looks like "everything is an orphan".
    #[actix_web::test]
    async fn test_sweep_refuses_to_delete_with_an_empty_database() {
        let store = FakeStore::new(vec![object("images/o/a.png", 9 * DAY)]);
        let options = SweepOptions {
            delete: true,
            ..Default::default()
        };

        assert!(sweep(&store, &HashSet::new(), NOW, &options).await.is_err());
        assert!(store.deleted.lock().unwrap().is_empty());

        let forced = SweepOptions {
            delete: true,
            allow_empty_db: true,
            ..Default::default()
        };
        assert_eq!(
            sweep(&store, &HashSet::new(), NOW, &forced)
                .await
                .unwrap()
                .deleted,
            1
        );
        // A dry run is always allowed.
        assert!(
            sweep(&store, &HashSet::new(), NOW, &SweepOptions::default())
                .await
                .is_ok()
        );
    }
}
