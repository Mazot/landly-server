# Landly Server - AI Coding Agent Instructions

> `AGENTS.md` in the repo root is the long-form version of this file (RBAC table,
> domain rules, sub-agents, env vars). Keep the two in sync.

## Architecture Overview

**Rust/Actix Web backend** with **layered hexagonal architecture** and strict separation of concerns:

```
HTTP → Authentication middleware → Controller → Usecase → Repository → Diesel/DB
                                                        ↓
                                            Cache (Redis or NoOp)
                                            Storage (S3/R2 or NoOp)
```

**Critical architectural patterns:**
- **Feature-based modules** (`src/app/features/{feature}/`) - standardized 8-file structure per feature
- **Dependency Injection via DiContainer** (`src/utils/di.rs`) - single source of truth for service construction
- **Redis caching with graceful degradation** - `TypedCache<Arc<dyn CacheService>>` falls back to NoOp when Redis unavailable
- **Object storage with graceful degradation** - `Arc<dyn StorageService>` falls back to NoOp when the `S3_*` vars are absent
- **Centralized error handling** - `AppError` enum with automatic HTTP status mapping and `From` trait implementations
- **RBAC in usecases** - the middleware only authenticates; every role/ownership check lives in a usecase

Current features: `common`, `corridor`, `country_connection`, `healthcheck`, `images`,
`moderation`, `organisation`, `person`, `report`, `review`, `saved`, `user`.

## Core Patterns

### 1. Feature Module Structure (CRITICAL)
Every feature follows this exact structure in `src/app/features/{feature}/`:
```
mod.rs          - exports all submodules + the feature's #[derive(OpenApi)] ApiDoc
entities.rs     - domain structs, feature enums, Diesel models + business logic methods
repositories.rs - trait + impl for data access (includes cache integration)
usecases.rs     - business logic orchestration, validation and RBAC
controllers.rs  - HTTP handlers (utoipa-annotated)
presenters.rs   - response formatting (trait + impl)
requests.rs     - request DTOs
config.rs       - routing configuration (plain fn, or a closure when the feature needs the cache middleware)
```

**Example**: See `src/app/features/organisation/` for the canonical implementation, or
`src/app/features/corridor/` for the smallest complete one.

Domain types (enums, Diesel models) belong in `entities.rs`, never in `repositories.rs`;
`repositories.rs` only holds the trait, its impl and the `*RepositoryInput` structs.

### 2. Dependency Injection Pattern
**DiContainer** (`src/utils/di.rs`) is the ONLY place where concrete types are constructed:
```rust
// Always Arc-wrap trait objects for thread safety
pub struct DiContainer {
    pub organisation_usecase: OrganisationUsecase,
    pub redis_cache_service: TypedCache<Arc<dyn CacheService>>,
    // ...
}
```

Repositories receive `TypedCache<Arc<dyn CacheService>>` in constructors for caching.

### 3. Cache Integration (Redis + Fallback)
**Key files**: `src/utils/cache.rs`, `src/utils/redis.rs`

Repositories use `TypedCache` wrapper for type-safe caching:
```rust
impl OrganisationRepositoryImpl {
    pub fn new(pool: DbPool, cache_service: TypedCache<Arc<dyn CacheService>>) -> Self {
        Self { pool, cache_service }
    }

    fn fetch_organisation(&self, id: Uuid) -> Result<Organisation, AppError> {
        let key = CacheKeys::organisation_by_id(&id);

        // Try cache first
        if let Some(org) = self.cache_service.get::<Organisation>(&key)? {
            return Ok(org);
        }

        // Fetch from DB and cache
        let org = /* diesel query */;
        self.cache_service.set(&key, &org, Some(Duration::from_secs(3600)))?;
        Ok(org)
    }
}
```

Every key goes through the `CacheKeys` namespace helper: `org:*`, `cc:*`, `img:*`,
`cor:*`, `per:*`, `common:*`. Namespaces must stay disjoint, and repositories invalidate
their own pattern on every create/update/delete. Cache invalidation on mutating routes is
additionally wired through the middleware in `src/app/drivers/middlewares/cache.rs`.

### 4. Routing Configuration
Features that need the cache-invalidation middleware expose a **closure-based config
function** (`organisation`, `country_connection`, `images`):
```rust
// src/app/features/organisation/config.rs
pub fn create_configure_services_closure(
    middleware: TypedCache<Arc<dyn CacheService>>
) -> impl Fn(&mut ServiceConfig) {
    move |cfg: &mut ServiceConfig| {
        cfg.service(
            web::scope("/organisation")
                .wrap(middleware.clone())  // Cache invalidation middleware
                .route("/create", web::post().to(create_organisation))
                // ...
        );
    }
}
```

All other features expose a plain `pub fn configure_services(cfg: &mut ServiceConfig)`.
Both are registered in `main.rs` inside the `/api` scope.

### 5. Error Handling
**AppError** (`src/error.rs`) uses thiserror and implements ResponseError:
```rust
#[derive(Error, ToSchema, Debug)]
pub enum AppError {
    #[error("Unauthorized: {}", _0)]
    Unauthorized(JsonValue),

    #[error("Not Found: {}", _0)]
    NotFound(JsonValue),
    // ... Forbidden, UnprocessableEntity, InternalServerError, ServiceUnavailable
}
```

**From implementations** handle conversion from library errors (Diesel, Redis, JWT, bcrypt,
UUID). Controllers just use `?`. Two rules worth remembering:
- Diesel constraint violations (unique / FK / CHECK / NOT NULL) become **422** — they are
  caused by client input. Every other `DatabaseError` stays a 500.
- **503 `ServiceUnavailable`** means an integration is not configured (Google OAuth without
  the `GOOGLE_*` vars).

### 6. Authentication & RBAC
`src/app/drivers/middlewares/auth.rs` holds `AUTH_REQUIRED_ROUTES` (the single source of
truth for protected routes) and `SKIP_AUTH_ROUTES` (docs + healthcheck). The middleware
validates the JWT and inserts the `Uuid` into request extensions; controllers read it via
the `caller_user_id` helper, **never** from the request body.

Route patterns are matched by `path_matches`: `{param}` matches exactly one non-empty
segment, and a pattern ending in `/` is a prefix match. A bare `starts_with` would silently
disable auth for `{id}` routes — `test_every_auth_required_route_matches_a_real_path`
guards against that.

Roles live in `users.role` (`user` < `moderator` < `admin`) and are read through
`User::fetch_role`, exposed per feature as `fetch_user_role`. All checks happen in usecases:
`ensure_can_manage` (ownership), `ensure_admin` (system reference tables),
`ensure_moderator` (moderation queue).

### 7. Enum Convention
No SQL enums. Enum-like columns are `TEXT` + `CHECK (col IN (...))` in the migration, plus a
Rust enum in the feature's `entities.rs` with `as_str()` and `TryFrom<&str>`. Usecases
validate incoming strings through `TryFrom` before writing, so an unknown value is a 422
instead of a database error.

### 8. OpenAPI Documentation
Use `utoipa` macros everywhere:
- Controllers: `#[utoipa::path(...)]`
- DTOs: `#[derive(ToSchema)]`
- Each feature declares its own `#[derive(OpenApi)] pub struct ApiDoc` in `<feature>/mod.rs`
  and is merged into the root doc by `build_openapi()` in `src/main.rs`. The root
  `#[openapi(...)]` macro only carries `info` + `servers` — never add paths to it.

Docs UI is **Scalar** at `/scalar`; the raw spec is at `/api-docs/openapi.json`, and
`/swagger-ui` 307-redirects to `/scalar`. New public doc routes also need an entry in
`SKIP_AUTH_ROUTES`.

## Development Workflows

### Database Migrations
```bash
# Create migration
diesel migration generate {name}

# Run migrations
diesel migration run

# Revert last
diesel migration revert
```

Diesel auto-generates `src/data/schema.rs` - **never edit manually**. Write a `down.sql`
that fully reverses `up.sql` and verify with `diesel migration redo`.

### Docker Development
```bash
# Start all services (Postgres, Redis, MinIO, app)
docker compose up -d

# Rebuild after Cargo.toml changes
docker compose build --no-cache

# View logs
docker compose logs -f landly-server
```

**Environment variables**: Copy `.env.example` to `.env` and configure:
- `DATABASE_URL`, `REDIS_URL`, `REDIS_USER`, `REDIS_PASSWORD`
- OAuth: `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, `OAUTH_GOOGLE_REDIRECT_URL`
- JWT: `JWT_SECRET`, `JWT_EXPIRATION` (seconds; both are required)
- Storage: `S3_ENDPOINT_URL`, `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY`, `S3_BUCKET`,
  optional `S3_REGION` / `S3_PUBLIC_URL`
- Server: `HOST`, `PORT`
- Connection pools: `DB_POOL_*`, `REDIS_POOL_*` settings

The container healthcheck targets `/api/healthcheck` — there is no `/health` route.

### Scripts Workspace
`scripts/crates` (`country_parser`, `country_loader`) is a **separate Cargo workspace** that
links the main crate as a library. Root `cargo build`/`cargo test` does NOT cover it — after
changing structs it consumes (notably `Country`/`CreateCountry` in `src/data/models.rs`),
run `cd scripts/crates && cargo build`.

### Testing & Code Quality
```bash
cargo test              # Run tests
cargo fmt               # Format code (enforced in CI)
cargo clippy            # Lint
cargo build --release   # Production build
```

Tests are inline `#[cfg(test)] mod tests` modules next to the code and must run without a
database: usecases are tested against hand-written stub repositories (see
`review/usecases.rs` or `moderation/usecases.rs`), presenters and enums directly.

**CI/CD**: GitHub Actions runs
1. `rust.yml` — fmt check, build, tests, a separate `scripts-workspace` job, coverage via tarpaulin → Codecov
2. `clippy.yml` — `cargo clippy --all-targets --all-features -- -D warnings`
3. `audit.yml` — `cargo audit` for both lockfiles, on dependency changes and weekly
4. `dependencies.yml` — weekly `cargo update` PR; `release.yml` — binary on `v*` tags

Always run `cargo fmt` before committing to pass CI checks.

## Authentication Flow

**Traditional Auth**: JWT-based with bcrypt password hashing
- Signup/Signin: `POST /api/user/signup`, `/api/user/signin`
- Returns: `{ "user": {...}, "token": "..." }`
- Use: `Authorization: Bearer <token>` header
- Signup v2 also accepts optional profile fields and a default corridor, created in one
  transaction (`User::signup_v2`)

**OAuth 2.0 (Google)**: PKCE + state stored in Redis
- Start: `GET /api/user/oauth/google/login` → redirects to Google
- Callback: `GET /api/user/oauth/google/callback?code=...&state=...`
- Links to `user_providers` table for multi-provider support
- Code: `src/app/features/user/oauth/google.rs`; returns 503 when unconfigured

## Common Pitfalls

1. **Don't bypass DiContainer** - always inject dependencies through it
2. **Cache keys must use CacheKeys struct** - defined in `src/utils/cache.rs`
3. **TypedCache needs Arc<dyn CacheService>** - not concrete types
4. **Feature configs use the closure pattern** - whenever the feature needs cache middleware
5. **Entity methods hold the SQL** - `User::signup()`, `Organisation::create()` etc.; the
   repository wraps them with pooling and cache invalidation
6. **Presenters format responses** - don't build JSON manually in controllers
7. **Update the per-feature `ApiDoc`** whenever you add, remove or rename a route
8. **Add the route to `AUTH_REQUIRED_ROUTES`** when it mutates state — and keep the count
   assertion in the auth tests in sync
9. **Format before committing** - CI enforces `cargo fmt -- --check` and will fail if not formatted

## Key Files Reference

- `src/main.rs` - Application entry, `build_openapi()`, routing
- `src/utils/di.rs` - Dependency injection container
- `src/utils/cache.rs` - Cache abstraction (TypedCache, CacheService trait, CacheKeys)
- `src/utils/storage.rs`, `src/utils/s3.rs` - Object storage abstraction (S3/R2 or NoOp)
- `src/error.rs` - Error handling and HTTP status mapping
- `src/app/drivers/middlewares/` - Auth, CORS, cache invalidation
- `src/app/features/{feature}/` - Feature modules following standard structure
- `docker-compose.yml` - Development environment (Postgres 17, Redis 7, MinIO)

## Database Schema Highlights

- **users** - profile + RBAC `role`, with `user_providers` for OAuth linking
- **corridors** - user "from country → to country" pairs, one default per user
- **organisations** - moderation `status`, `created_by` ownership, opening hours + timezone,
  visit/rating counters; **organisation_types** carries a stable `slug`
- **people** (+ `people_to_languages`, `person_claim_tokens`, `person_vouches`) - recommended
  helpers with hidden contacts and the claim-and-verify flow
- **reviews** - polymorphic (exactly one of org/person via CHECK), unique per author-target
- **saved_items**, **org_checkins**, **reports**, **moderation_events** - bookmarks, community
  check-ins, user reports and the moderation audit trail
- **countries**, **languages**, **countries_connections**, **countries_to_languages**,
  **users_to_languages** - reference data and many-to-many relationships

Migrations: `migrations/{timestamp}_{name}/up.sql` and `down.sql`
