# AGENTS.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build

# Run dev server (requires .env with DATABASE_URL, REDIS_URL, JWT_SECRET)
cargo run

# Run all tests
cargo test

# Run a single test by name
cargo test test_name

# Run tests with stdout
cargo test -- --nocapture

# Format code (required before committing)
cargo fmt

# Lint
cargo clippy

# Database migrations
diesel migration generate <name>
diesel migration run
diesel migration revert
```

## Architecture

This is an Actix Web REST API following a clean layered architecture. The main module is declared as a `[lib]` in `Cargo.toml`, so `src/main.rs` is the crate root.

### Layer order per request

```
HTTP → Authentication middleware → Controller → Usecase → Repository → Diesel/DB
                                                         ↓
                                             Cache (Redis or NoOp)
                                             Storage (S3/R2 or NoOp)
```

### Feature structure

Each domain feature lives under `src/app/features/<feature>/` with a consistent set of files:

- `config.rs` — registers Actix routes for the feature; wraps scope with cache middleware
- `controllers.rs` — Actix handler functions; extracts request data and delegates to usecase
- `requests.rs` — serde-deserializable input structs for query params / JSON bodies
- `usecases.rs` — business logic; accepts typed input structs, calls repository, calls presenter
- `repositories.rs` — Diesel queries; implements a `<Feature>Repository` trait
- `presenters.rs` — converts DB models to HTTP responses (`HttpResponse`)
- `entities.rs` — domain structs (distinct from Diesel models in `src/data/models.rs`)

Current features: `common`, `country_connection`, `healthcheck`, `images`, `organisation`, `user`

### Dependency injection

`src/utils/di.rs` contains `DiContainer`, which is constructed once at startup in `AppState::new` (`src/app/drivers/middlewares/state.rs`). It holds all usecase instances (each wrapping `Arc<dyn Repository>` + `Arc<dyn Presenter>`), the cache service, and the storage service. `AppState` is shared via `web::Data<AppState>`.

Controllers access dependencies via:
```rust
state: web::Data<AppState>
// then: state.di_container.organisation_usecase
```

### Authentication middleware

`src/app/drivers/middlewares/auth.rs` is a global Actix middleware (`Authentication`) that protects routes defined in the static `AUTH_REQUIRED_ROUTES` array. When a route requires auth, the middleware validates the JWT and inserts the `Uuid` user ID into request extensions. When adding new protected routes, update `AUTH_REQUIRED_ROUTES` in that file.

### Cache abstraction

`src/utils/cache.rs` defines `CacheService` trait and `TypedCache<T>` wrapper. At startup:
- If `REDIS_URL` is set → `RedisCacheService`
- Otherwise → `NoOpCacheService` (silently ignores all cache operations)

Cache keys follow the `CacheKeys` namespace helper in the same file (`org:*`, `cc:*`, `img:*`).

### Storage abstraction

`src/utils/storage.rs` defines `StorageService` trait (object-safe via boxed futures). `src/utils/s3.rs` provides `S3ClientWrapper` (also works with Cloudflare R2). At startup:
- If `S3_ENDPOINT_URL`, `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY`, `S3_BUCKET` are all set → S3
- Otherwise → `NoOpStorageService` (returns `InternalServerError` on upload/delete)

### Error handling

`src/error.rs` defines `AppError` (Unauthorized, Forbidden, NotFound, UnprocessableEntity, InternalServerError). It implements `ResponseError` for Actix and `From<_>` for all library error types (Diesel, JWT, bcrypt, Redis, UUID). All controller and usecase functions return `Result<HttpResponse, AppError>`.

### OpenAPI

All public endpoints and schemas must be registered in the `#[openapi(...)]` macro in `src/main.rs` (both `paths` and `components/schemas`). The Swagger UI is available at `/swagger-ui`.

## Environment Variables

| Variable | Required | Description |
|---|---|---|
| `DATABASE_URL` | yes | PostgreSQL connection string |
| `REDIS_URL` | no | Redis connection string; cache disabled if absent |
| `JWT_SECRET` | yes | JWT signing secret |
| `JWT_EXP_SECS` | yes | JWT expiry in seconds |
| `GOOGLE_CLIENT_ID` | no | OAuth Google client ID |
| `GOOGLE_CLIENT_SECRET` | no | OAuth Google client secret |
| `OAUTH_GOOGLE_REDIRECT_URL` | no | Google OAuth callback URL |
| `S3_ENDPOINT_URL` | no | S3/R2 endpoint; storage disabled if absent |
| `S3_ACCESS_KEY_ID` | no | S3/R2 access key |
| `S3_SECRET_ACCESS_KEY` | no | S3/R2 secret key |
| `S3_BUCKET` | no | S3/R2 bucket name |
| `S3_REGION` | no | Region (defaults to `"auto"`) |
| `S3_PUBLIC_URL` | no | Public base URL for object access |
| `HOST` | yes | Server bind host |
| `PORT` | yes | Server bind port |
