use dotenv::dotenv;
use landly_server::utils::{
    db::establish_connection,
    orphan_cleanup::{self, SweepOptions},
    s3::S3ClientWrapper,
};
use std::process::ExitCode;

const USAGE: &str = "\
Find (and optionally delete) image objects in the S3 bucket that no `images` row points to.

Usage: orphan_images [--delete] [--min-age-hours N] [--allow-empty-db]

  (default)          dry run: only report
  --delete           delete the orphans
  --min-age-hours N  ignore objects younger than N hours (default 24):
                     an upload writes the object before its row
  --allow-empty-db   allow --delete when the database knows no images at all
                     (normally a sign of the wrong DATABASE_URL)

Needs DATABASE_URL and the S3_* variables.";

fn parse_args() -> Result<SweepOptions, String> {
    let mut options = SweepOptions::default();
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--delete" => options.delete = true,
            "--allow-empty-db" => options.allow_empty_db = true,
            "--min-age-hours" => {
                let hours: i64 = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|h| *h >= 0)
                    .ok_or("--min-age-hours needs a non-negative number")?;
                options.min_age_secs = hours * 60 * 60;
            }
            "--help" | "-h" => return Err(USAGE.to_string()),
            other => return Err(format!("unknown argument `{other}`\n\n{USAGE}")),
        }
    }

    Ok(options)
}

fn main() -> ExitCode {
    dotenv().ok();

    let options = match parse_args() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    let storage = match S3ClientWrapper::new() {
        Ok(storage) => storage,
        Err(_) => {
            eprintln!("S3 is not configured (S3_ENDPOINT_URL, S3_ACCESS_KEY_ID, S3_SECRET_ACCESS_KEY, S3_BUCKET)");
            return ExitCode::from(1);
        }
    };
    let pool = establish_connection();
    let known = match orphan_cleanup::known_image_keys(&pool) {
        Ok(known) => known,
        Err(e) => {
            eprintln!("could not read the images table: {e:?}");
            return ExitCode::from(1);
        }
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let report = match orphan_cleanup::block_on(orphan_cleanup::sweep(
        &storage, &known, now, &options,
    )) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };

    println!(
        "bucket {}: {} objects under images/, {} in use, {} too new to judge, {} orphaned",
        storage.get_bucket(),
        report.scanned,
        report.in_use,
        report.too_new,
        report.orphans.len()
    );
    for key in &report.orphans {
        println!("  orphan: {key}");
    }
    if options.delete {
        println!("deleted {}, failed {}", report.deleted, report.failed);
        if report.failed > 0 {
            return ExitCode::from(1);
        }
    } else if !report.orphans.is_empty() {
        println!("dry run — rerun with --delete to remove them");
    }

    ExitCode::SUCCESS
}
