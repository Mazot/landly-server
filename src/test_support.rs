//! Helpers for the DB-backed HTTP tests (test builds only).
//!
//! These tests run the real stack below the network: actix routing + auth
//! middleware → controllers → usecases → repositories → Postgres. They need a
//! dedicated database in `TEST_DATABASE_URL` (env var, or that single key in
//! `.env`); without it every such test returns early, so `cargo test` stays
//! green on machines without Postgres. Migrations are applied once per test
//! process. The database name must contain `test` — the tests write freely.
//!
//! Tests never assume an empty database: every row they create uses a unique
//! name/email/coordinate, and assertions look for those rows only.
//!
//! Every test that issues a JWT must be `#[serial(jwt_env)]` (shared with the
//! token/auth unit tests that rewrite `JWT_SECRET`).

use crate::app::features::organisation::entities::{CreateOrganisation, Organisation};
use crate::app::features::user::entities::{CreateUser, User};
use crate::data::models::{Country, CreateCountry, CreateOrganisationType, OrganisationType};
use crate::data::schema::{organisation_types, organisations, users};
use crate::utils::{db::DbPool, token};
use actix_http::Request;
use actix_web::{
    body::MessageBody,
    dev::{Service, ServiceResponse},
    http::{Method, StatusCode},
    test::{self, TestRequest},
};
use bigdecimal::{BigDecimal, FromPrimitive};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use serde_json::Value;
use std::sync::OnceLock;
use std::time::Duration;
use uuid::Uuid;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
const TEST_JWT_SECRET: &str = "landly-db-test-secret";

fn database_url() -> Option<String> {
    if let Ok(url) = std::env::var("TEST_DATABASE_URL") {
        return Some(url).filter(|u| !u.is_empty());
    }
    // Local convenience: read only this one key from .env (never DATABASE_URL).
    let env = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).ok()?;
    env.lines()
        .find_map(|l| l.strip_prefix("TEST_DATABASE_URL="))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Pool for the test database, or `None` when `TEST_DATABASE_URL` is unset.
pub fn pool() -> Option<DbPool> {
    static POOL: OnceLock<Option<DbPool>> = OnceLock::new();

    POOL.get_or_init(|| {
        let url = database_url()?;
        let pool = Pool::builder()
            .max_size(8)
            .connection_timeout(Duration::from_secs(5))
            .build(ConnectionManager::<PgConnection>::new(url))
            .expect("cannot connect to TEST_DATABASE_URL");
        let conn = &mut pool.get().expect("test database connection");

        // Check the database the server actually selected, not the URL text:
        // a key-value DSN (`host=… dbname=landly`), PGDATABASE or a query
        // parameter could otherwise point the writes at a real database.
        let db_name: String = diesel::select(diesel::dsl::sql::<diesel::sql_types::Text>(
            "current_database()",
        ))
        .get_result(conn)
        .expect("SELECT current_database()");
        assert!(
            db_name.contains("test"),
            "TEST_DATABASE_URL connects to `{db_name}`, whose name does not contain `test`: \
             the DB tests write to it, so they refuse to run there"
        );

        conn.run_pending_migrations(MIGRATIONS)
            .expect("applying migrations to the test database");

        Some(pool)
    })
    .clone()
}

/// `let pool = require_db!();` — skips the test (returns) without a test DB.
macro_rules! require_db {
    () => {
        match $crate::test_support::pool() {
            Some(pool) => pool,
            None => {
                eprintln!("skipped: set TEST_DATABASE_URL to run DB-backed tests");
                return;
            }
        }
    };
}
pub(crate) use require_db;

/// The production `/api` routing + auth middleware over a `DiContainer` with
/// explicit test backends — NoOp cache, NoOp storage, no Google OAuth (503) —
/// whatever `REDIS_URL` / `S3_*` / `GOOGLE_*` the developer has exported.
macro_rules! test_app {
    ($pool:expr) => {{
        $crate::test_support::jwt_env();
        let state = $crate::app::drivers::middlewares::state::AppState {
            di_container: $crate::utils::di::DiContainer::with_backends(
                &$pool,
                $crate::utils::cache::TypedCache::new(std::sync::Arc::new(
                    $crate::utils::cache::NoOpCacheService,
                )
                    as std::sync::Arc<dyn $crate::utils::cache::CacheService>),
                std::sync::Arc::new($crate::utils::storage::NoOpStorageService),
                None,
            ),
        };
        let cache = state.di_container.redis_cache_service.clone();
        actix_web::test::init_service(
            actix_web::App::new()
                .app_data(actix_web::web::Data::new(state))
                .wrap($crate::app::drivers::middlewares::auth::Authentication)
                .service(actix_web::web::scope("/api").configure(|cfg| {
                    $crate::configure_api(
                        cfg,
                        cache,
                        $crate::app::drivers::middlewares::rate_limit::auth_rate_limit(),
                        None,
                    )
                })),
        )
        .await
    }};
}
pub(crate) use test_app;

/// Sets the JWT env the auth middleware and `token::generate_token` read.
/// Callers must hold the `jwt_env` serial lock.
pub fn jwt_env() {
    // SAFETY: every test touching these vars is #[serial(jwt_env)].
    unsafe {
        std::env::set_var("JWT_SECRET", TEST_JWT_SECRET);
        std::env::set_var("JWT_EXPIRATION", "3600");
    }
}

/// One request through the app; returns the status and the JSON body
/// (`Value::Null` for empty / non-JSON bodies).
pub async fn call<S, B>(
    app: &S,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let mut req = TestRequest::default()
        .method(method)
        .uri(path)
        // The rate limiter keys clients by peer IP.
        .peer_addr("127.0.0.1:40000".parse().unwrap());
    if let Some(token) = token {
        req = req.insert_header(("Authorization", format!("Bearer {token}")));
    }
    if let Some(body) = body {
        req = req.set_json(body);
    }

    let resp = test::call_service(app, req.to_request()).await;
    let status = resp.status();
    let bytes = test::read_body(resp).await;

    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Short unique suffix for names, emails and slugs.
pub fn unique() -> String {
    Uuid::new_v4().simple().to_string()[..12].to_string()
}

pub struct TestUser {
    pub id: Uuid,
    pub email: String,
    pub token: String,
}

/// A user inserted directly (no bcrypt — it cannot sign in with a password)
/// with the given role, plus a valid JWT. Needs `jwt_env` (call after
/// `test_app!`, which sets it).
pub fn user(pool: &DbPool, role: &str) -> TestUser {
    let conn = &mut pool.get().unwrap();
    let suffix = unique();
    let user: User = diesel::insert_into(users::table)
        .values(&CreateUser {
            username: format!("u{suffix}"),
            email: format!("u{suffix}@example.test"),
            password_hash: "not-a-bcrypt-hash".to_string(),
            name: None,
            locale: None,
            here_as: None,
            home_country_id: None,
            avatar_color: None,
        })
        .get_result(conn)
        .unwrap();
    if role != "user" {
        diesel::update(users::table.find(user.id))
            .set(users::role.eq(role))
            .execute(conn)
            .unwrap();
    }

    TestUser {
        id: user.id,
        email: user.email,
        token: token::generate_token(user.id).unwrap(),
    }
}

pub fn country(pool: &DbPool) -> Country {
    let conn = &mut pool.get().unwrap();
    Country::create(
        conn,
        &CreateCountry {
            name: format!("Testland {}", unique()),
            geo_json: None,
            flag: Some("🏳️".to_string()),
            capital_city: Some("Capital".to_string()),
            description: None,
            currency: Some("EUR".to_string()),
            phone_code: Some("+00".to_string()),
            top_cities: None,
        },
    )
    .unwrap()
}

pub fn org_type(pool: &DbPool) -> OrganisationType {
    let conn = &mut pool.get().unwrap();
    let slug = format!("type-{}", unique());
    diesel::insert_into(organisation_types::table)
        .values(&CreateOrganisationType {
            org_type: slug.clone(),
            color: Some("#123456".to_string()),
            title: Some("Test type".to_string()),
            slug: Some(slug),
        })
        .get_result(conn)
        .unwrap()
}

/// Options for [`org`]; `Default` gives a live org with no location.
#[derive(Default)]
pub struct OrgSpec {
    pub created_by: Option<Uuid>,
    pub status: Option<&'static str>,
    pub country: Option<Uuid>,
    pub org_type: Option<Uuid>,
    pub lat_lng: Option<(f64, f64)>,
    pub name: Option<String>,
}

pub fn org(pool: &DbPool, spec: OrgSpec) -> Organisation {
    let conn = &mut pool.get().unwrap();
    let decimal = |v: f64| BigDecimal::from_f64(v).unwrap();
    let created = Organisation::create(
        conn,
        &CreateOrganisation {
            name: spec.name.unwrap_or_else(|| format!("Org {}", unique())),
            tel: None,
            email: None,
            address: None,
            description: None,
            location_country_id: spec.country,
            organisation_type_id: spec.org_type,
            latitude: spec.lat_lng.map(|(lat, _)| decimal(lat)),
            longitude: spec.lat_lng.map(|(_, lng)| decimal(lng)),
            founder_country_id: None,
            created_by: spec.created_by,
            status: spec.status.unwrap_or("live").to_string(),
            added_by: None,
            city: None,
            website: None,
            telegram: None,
            whatsapp: None,
            services: vec![],
            languages: vec![],
            opening_hours: None,
            timezone: None,
            cost: None,
            google_place_id: None,
        },
    )
    .unwrap();

    organisations::table.find(created.id).first(conn).unwrap()
}

/// A random point far from everything else the tests create, so searches
/// around it only see the rows a test puts there.
pub fn random_point() -> (f64, f64) {
    let n = Uuid::new_v4().as_u128();
    let lat = -60.0 + (n % 120_000) as f64 / 1000.0; // -60..60
    let lng = -170.0 + ((n >> 32) % 340_000) as f64 / 1000.0; // -170..170
    (lat, lng)
}
