//! Access log that cannot leak credentials (audit H1).
//!
//! actix's default `Logger` writes the whole request line and the `Referer`
//! header. Here that line carries secrets: the person claim token is a path
//! segment (`/api/person/claim/{token}`), the OAuth callback carries `code`
//! and `state` in the query, and the frontend's `Referer` can be a page whose
//! URL holds a claim token. Anyone who can read the logs could take over a
//! claim or replay a login code.
//!
//! So the log line has no query string, no `Referer`, and the path has its
//! secret segments masked ([`redact_path`]).

use actix_web::middleware::Logger;

/// What replaces a masked path segment.
const MASK: &str = "{redacted}";

/// Segments of this length or longer that look like opaque tokens are masked
/// even on routes not known to carry one (a UUID is 36 characters but has
/// dashes in fixed places and is recognised separately).
const OPAQUE_TOKEN_MIN_LEN: usize = 32;

/// `path` (no query) with credential-bearing segments masked: the one after
/// `claim`, and any segment that is a long opaque token rather than a UUID.
pub fn redact_path(path: &str) -> String {
    let mut after_claim = false;
    path.split('/')
        .map(|segment| {
            let mask = after_claim || is_opaque_token(segment);
            after_claim = segment == "claim";
            if mask { MASK } else { segment }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn is_opaque_token(segment: &str) -> bool {
    segment.len() >= OPAQUE_TOKEN_MIN_LEN
        && uuid::Uuid::parse_str(segment).is_err()
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~' | '%'))
}

/// The application's access logger: client address, method, redacted path,
/// status, size, user agent, duration.
pub fn request_logger() -> Logger {
    Logger::new("%a \"%m %{path}xi\" %s %b \"%{User-Agent}i\" %T")
        .custom_request_replace("path", |req| redact_path(req.path()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, HttpResponse, test as actix_test, web};
    use std::sync::{Mutex, OnceLock};

    const TOKEN: &str = "Rk3xV9mQ2pLw8ZtYc4HnBd7sJaUe5GfO1iXz6TqNvMo";

    #[test]
    fn test_redact_path_masks_the_claim_token() {
        for (path, expected) in [
            (
                format!("/api/person/claim/{TOKEN}"),
                "/api/person/claim/{redacted}".to_string(),
            ),
            (
                format!("/api/person/claim/{TOKEN}/confirm"),
                "/api/person/claim/{redacted}/confirm".to_string(),
            ),
            // Short tokens are masked by position, not by length.
            (
                "/api/person/claim/abc/decline".into(),
                "/api/person/claim/{redacted}/decline".into(),
            ),
        ] {
            assert_eq!(redact_path(&path), expected);
        }
    }

    #[test]
    fn test_redact_path_keeps_ordinary_paths() {
        for path in [
            "/api/organisation/list",
            "/api/organisation/update/6f1c2b9e-1d3a-4c55-8e0a-0d2f4b7a9c11",
            "/api/user/oauth/google/callback",
            "/",
        ] {
            assert_eq!(redact_path(path), path);
        }
    }

    #[test]
    fn test_redact_path_masks_unknown_long_tokens() {
        assert_eq!(
            redact_path(&format!("/api/other/{TOKEN}")),
            "/api/other/{redacted}"
        );
    }

    /// Captures everything logged through the `log` facade.
    struct Capture(Mutex<Vec<String>>);

    impl log::Log for Capture {
        fn enabled(&self, _: &log::Metadata) -> bool {
            true
        }
        fn log(&self, record: &log::Record) {
            self.0.lock().unwrap().push(record.args().to_string());
        }
        fn flush(&self) {}
    }

    fn capture() -> &'static Capture {
        static CAPTURE: OnceLock<&'static Capture> = OnceLock::new();
        CAPTURE.get_or_init(|| {
            let capture: &'static Capture = Box::leak(Box::new(Capture(Mutex::new(vec![]))));
            log::set_logger(capture).expect("no other test installs a logger");
            log::set_max_level(log::LevelFilter::Info);
            capture
        })
    }

    /// End to end: the line that reaches the logger holds no token, no
    /// OAuth code/state and no Referer.
    #[actix_web::test]
    async fn test_access_log_line_has_no_secrets() {
        let capture = capture();
        let app = actix_test::init_service(App::new().wrap(request_logger()).route(
            "/api/{tail:.*}",
            web::to(|| async { HttpResponse::Ok().finish() }),
        ))
        .await;

        for uri in [
            format!("/api/person/claim/{TOKEN}"),
            "/api/user/oauth/google/callback?code=SECRETCODE123&state=SECRETSTATE456".to_string(),
        ] {
            let req = actix_test::TestRequest::get()
                .uri(&uri)
                .insert_header(("Referer", format!("https://app.test/claim/{TOKEN}")))
                .to_request();
            let res = actix_test::call_service(&app, req).await;
            assert!(res.status().is_success());
        }

        let lines = capture.0.lock().unwrap().join("\n");
        // Failure messages deliberately carry no log content: on a failure
        // that content is the very secret the test hunts for.
        for (n, forbidden) in [TOKEN, "SECRETCODE123", "SECRETSTATE456", "app.test"]
            .iter()
            .enumerate()
        {
            assert!(
                !lines.contains(forbidden),
                "forbidden value #{n} reached the access log"
            );
        }
        assert!(
            lines.contains("/api/person/claim/{redacted}"),
            "claim path was not masked"
        );
        assert!(
            lines.contains("/api/user/oauth/google/callback"),
            "callback path missing from the access log"
        );
    }
}
