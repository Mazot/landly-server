//! Test-only inventory of every HTTP route the app registers, read from the
//! feature `config.rs` sources. Actix cannot enumerate its routes, so the
//! guard tests (OpenAPI coverage in `main.rs`, auth coverage in
//! `middlewares/auth.rs`) parse the route table instead. Features are
//! discovered on disk, so a new feature is covered without touching this file.

use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegisteredRoute {
    /// Upper-case HTTP method, e.g. `POST`.
    pub method: String,
    /// Full path as registered, e.g. `/api/corridor/delete/{id}`.
    pub path: String,
    pub feature: String,
}

impl RegisteredRoute {
    /// Path with every `{param}` collapsed to `{}` so differently named
    /// params (`{id}` vs `{organisation_id}`) compare equal.
    pub fn normalized_path(&self) -> String {
        normalize_params(&self.path)
    }
}

pub fn normalize_params(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut in_param = false;

    for c in path.chars() {
        match c {
            '{' => {
                in_param = true;
                out.push_str("{}");
            }
            '}' => in_param = false,
            _ if in_param => {}
            _ => out.push(c),
        }
    }

    out
}

/// Every route registered under `/api`: all feature `config.rs` files plus
/// the healthcheck, which `main.rs` registers inline.
pub fn registered_routes() -> Vec<RegisteredRoute> {
    let features_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app/features");
    let mut routes = vec![RegisteredRoute {
        method: "GET".to_string(),
        path: "/api/healthcheck".to_string(),
        feature: "healthcheck".to_string(),
    }];

    for entry in fs::read_dir(&features_dir).expect("src/app/features is readable") {
        let dir = entry.expect("feature dir entry").path();
        let config = dir.join("config.rs");
        if !config.is_file() {
            continue;
        }
        let feature = dir.file_name().unwrap().to_string_lossy().into_owned();
        let source = fs::read_to_string(&config).expect("config.rs is readable");
        routes.extend(parse_config(&feature, &source));
    }

    routes.sort();
    routes
}

/// Parses `web::scope("/x")` and every `.route("/y", web::<method>()...)` of
/// one feature config, tolerating rustfmt's multi-line layouts.
pub fn parse_config(feature: &str, source: &str) -> Vec<RegisteredRoute> {
    let scope = string_after(source, "web::scope(")
        .unwrap_or_else(|| panic!("{feature}/config.rs has no web::scope(\"...\")"));

    let mut routes = vec![];
    let mut rest = source;
    while let Some(pos) = rest.find(".route(") {
        rest = &rest[pos + ".route(".len()..];
        let path = string_after(rest, "")
            .unwrap_or_else(|| panic!("{feature}/config.rs: .route( without a path literal"));
        let method_start = rest
            .find("web::")
            .unwrap_or_else(|| panic!("{feature}/config.rs: route {path} has no web::<method>()"))
            + "web::".len();
        let method: String = rest[method_start..]
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();

        routes.push(RegisteredRoute {
            method: method.to_uppercase(),
            path: format!("/api{scope}{path}"),
            feature: feature.to_string(),
        });
    }

    routes
}

/// The first string literal after `marker` (skipping whitespace).
fn string_after(source: &str, marker: &str) -> Option<String> {
    let start = source.find(marker)? + marker.len();
    let rest = source[start..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;

    Some(rest[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config_handles_single_and_multi_line_routes() {
        let source = r#"
            cfg.service(
                web::scope("/user")
                    .route(
                        "/signin",
                        web::post().to(signin).wrap(Governor::new(auth_limit)),
                    )
                    .route("/me", web::get().to(fetch_me))
                    .route("/{user_id}/languages", web::get().to(fetch_languages)),
            );
        "#;

        let routes = parse_config("user", source);
        let pairs: Vec<(&str, &str)> = routes
            .iter()
            .map(|r| (r.method.as_str(), r.path.as_str()))
            .collect();

        assert_eq!(
            pairs,
            vec![
                ("POST", "/api/user/signin"),
                ("GET", "/api/user/me"),
                ("GET", "/api/user/{user_id}/languages"),
            ]
        );
    }

    #[test]
    fn test_normalize_params_collapses_names() {
        assert_eq!(
            normalize_params("/api/images/upload/{organisation_id}"),
            "/api/images/upload/{}"
        );
        assert_eq!(
            normalize_params("/api/person/claim/{token}/confirm"),
            "/api/person/claim/{}/confirm"
        );
        assert_eq!(normalize_params("/api/user/me"), "/api/user/me");
    }

    /// Sanity floor: if parsing silently broke, every guard test built on
    /// this inventory would pass vacuously.
    #[test]
    fn test_registered_routes_finds_every_feature() {
        let routes = registered_routes();
        assert!(routes.len() >= 50, "only {} routes parsed", routes.len());

        for feature in [
            "common",
            "corridor",
            "country_connection",
            "images",
            "moderation",
            "organisation",
            "person",
            "report",
            "review",
            "saved",
            "user",
        ] {
            assert!(
                routes.iter().any(|r| r.feature == feature),
                "no routes parsed for feature `{feature}`"
            );
        }
    }
}
