# Landly Server - AI Coding Agent Instructions

## Architecture Overview

This is a **Rust/Actix Web backend** following a **layered hexagonal architecture** with strict separation of concerns:

```
Controllers → Usecases → Repositories → Entities
     ↓           ↓            ↓
Presenters ← DI Container → Cache Layer
```

**Key architectural decisions:**
- **Feature-based modules** (`src/app/features/{feature}/`) - each contains: `entities`, `repositories`, `usecases`, `controllers`, `presenters`, `requests`, `config`
- **Dependency Injection via DiContainer** (`src/utils/di.rs`) - constructs all services with Arc-wrapped trait objects
- **Redis caching with trait abstraction** - `TypedCache<Arc<dyn CacheService>>` enables fallback to NoOp when Redis unavailable
- **Error handling via AppError enum** - automatic conversion from Diesel, Redis, JWT, bcrypt errors with proper HTTP status codes

## Core Patterns

### 1. Feature Module Structure (CRITICAL)
Every feature follows this exact structure in `src/app/features/{feature}/`:
```
mod.rs          - exports all submodules
entities.rs     - Diesel models + business logic methods
repositories.rs - trait + impl for data access (includes cache integration)
usecases.rs     - business logic orchestration
controllers.rs  - HTTP handlers (utoipa-annotated)
presenters.rs   - response formatting (trait + impl)
requests.rs     - request DTOs with validation
config.rs       - routing configuration closure
```

**Example**: See `src/app/features/organisation/` for the canonical implementation.

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

**Cache invalidation** happens via middleware (`src/app/drivers/middlewares/cache.rs`) on POST/PUT/DELETE routes.

### 4. Routing Configuration Closures
Features expose a **closure-based config function** to inject middleware:
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

Called in `main.rs`:
```rust
let org_config = app::features::organisation::config::create_configure_services_closure(
    app_state.di_container.redis_cache_service.clone()
);
App::new().configure(org_config)
```

### 5. Error Handling
**AppError** (`src/error.rs`) uses thiserror and implements ResponseError:
```rust
#[derive(Error, ToSchema, Debug)]
pub enum AppError {
    #[error("Unauthorized: {}", _0)]
    Unauthorized(JsonValue),
    
    #[error("Not Found: {}", _0)]
    NotFound(JsonValue),
    // ...
}
```

**From implementations** handle conversion from library errors (Diesel, Redis, JWT). Controllers just use `?` operator.

### 6. OpenAPI Documentation
Use `utoipa` macros everywhere:
- Controllers: `#[utoipa::path(...)]`
- DTOs: `#[derive(ToSchema)]`
- Register in `main.rs` ApiDoc struct

Swagger UI available at `/swagger-ui`.

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

Diesel auto-generates `src/data/schema.rs` - **never edit manually**.

### Docker Development
```bash
# Start all services (Postgres, Redis, app)
docker compose up -d

# Rebuild after Cargo.toml changes
docker compose build --no-cache

# View logs
docker compose logs -f landly-server
```

**Environment variables**: Copy `.env.example` to `.env` and configure:
- `DATABASE_URL`, `REDIS_URL`, `REDIS_USER`, `REDIS_PASSWORD`
- OAuth: `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`
- JWT: `JWT_SECRET`, `JWT_EXP_SECS`

### Testing & Code Quality
```bash
cargo test              # Run tests
cargo fmt              # Format code
cargo clippy           # Lint
cargo build --release  # Production build
```

## Authentication Flow

**Traditional Auth**: JWT-based with bcrypt password hashing
- Signup/Signin: `POST /api/user/signup`, `/api/user/signin`
- Returns: `{ "user": {...}, "token": "..." }`
- Use: `Authorization: Bearer <token>` header

**OAuth 2.0 (Google)**: PKCE + state stored in Redis
- Start: `GET /api/user/oauth/google/login` → redirects to Google
- Callback: `GET /api/user/oauth/google/callback?code=...&state=...`
- Links to `user_providers` table for multi-provider support
- Code: `src/app/features/user/oauth/google.rs`

**Middleware**: `src/app/drivers/middlewares/auth.rs` validates JWT tokens.

## Common Pitfalls

1. **Don't bypass DiContainer** - always inject dependencies through it
2. **Cache keys must use CacheKeys struct** - defined in `src/utils/cache.rs`
3. **TypedCache needs Arc<dyn CacheService>** - not concrete types
4. **Feature configs must use closure pattern** - to inject middleware at runtime
5. **Entities live in repositories** - `User::signup()`, `Organisation::create()` are repository methods
6. **Presenters format responses** - don't build JSON manually in controllers

## Key Files Reference

- `src/main.rs` - Application entry, OpenAPI setup, routing
- `src/utils/di.rs` - Dependency injection container
- `src/utils/cache.rs` - Cache abstraction (TypedCache, CacheService trait)
- `src/error.rs` - Error handling and HTTP status mapping
- `src/app/drivers/middlewares/` - Auth, CORS, cache invalidation
- `src/app/features/{feature}/` - Feature modules following standard structure
- `docker-compose.yml` - Development environment (Postgres 17, Redis 7)

## Database Schema Highlights

- **users** - with `user_providers` for OAuth linking (provider, provider_user_id)
- **organisations** - with types, country connections
- **countries_to_languages**, **user_to_languages** - many-to-many relationships

Migrations: `migrations/{timestamp}_{name}/up.sql` and `down.sql`
