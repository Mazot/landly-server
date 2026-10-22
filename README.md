# 🌍 Landly APP Server

[![Rust](https://img.shields.io/badge/rust-1.94+-orange.svg)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-ready-blue.svg)](https://www.docker.com)
[![PostgreSQL](https://img.shields.io/badge/postgresql-17-blue.svg)](https://www.postgresql.org)
[![Redis](https://img.shields.io/badge/redis-7-red.svg)](https://redis.io)
[![codecov](https://codecov.io/gh/Mazot/landly-server/branch/master/graph/badge.svg)](https://codecov.io/gh/Mazot/landly-server)

## About

Landly is a backend server for an application that helps people find businesses, helpers, and official representatives of different nationalities in foreign countries. Whether you're a traveler, expatriate, or immigrant, Landly connects you with culturally familiar services and support networks in your new location.

**Key Features:**

- 🏢 **Business Discovery** - Find businesses run by or catering to your nationality
- 🤝 **Community Helpers** - Connect with volunteers and community organizers
- 🏛️ **Official Representatives** - Locate embassies, consulates, and cultural centers
- 🌐 **Multi-language Support** - Browse content in your preferred language

## 🛠️ Technology Stack

### Core Technologies

- **[Rust](https://www.rust-lang.org)** - Systems programming language for performance and safety
- **[Actix Web](https://actix.rs)** - Powerful, pragmatic, and extremely fast web framework
- **[Diesel](https://diesel.rs)** - Safe, extensible ORM and Query Builder
- **[Tokio](https://tokio.rs)** - Asynchronous runtime for Rust

### Database & Caching

- **[PostgreSQL 17](https://www.postgresql.org)** - Advanced open source relational database
- **[Redis 7](https://redis.io)** - In-memory data structure store for caching

### Authentication & Security

- **[jsonwebtoken](https://crates.io/crates/jsonwebtoken)** - JWT implementation
- **[oauth2](https://crates.io/crates/oauth2)** - OAuth 2.0 client library
- **[bcrypt](https://crates.io/crates/bcrypt)** - Password hashing
- **[actix-governor](https://crates.io/crates/actix-governor)** - Per-client rate limiting

### Storage & API docs

- **[aws-sdk-s3](https://crates.io/crates/aws-sdk-s3)** - Image storage on S3 / Cloudflare R2 (MinIO locally)
- **[utoipa](https://crates.io/crates/utoipa)** + **[utoipa-scalar](https://crates.io/crates/utoipa-scalar)** - OpenAPI 3.1 spec and the Scalar docs UI

### Development & Deployment

- **[Docker](https://www.docker.com)** - Containerization platform
- **[Docker Compose](https://docs.docker.com/compose/)** - Multi-container Docker applications

## 📋 Prerequisites

- **Rust 1.94+** (edition 2024; the locked `aws-sdk-s3` needs 1.94) - [Install Rust](https://rustup.rs/)
- **Docker & Docker Compose** - [Install Docker](https://docs.docker.com/get-docker/)
- **PostgreSQL 17** (if running locally)
- **Redis 7** (if running locally)

## 🚀 Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/Mazot/landly-server.git
cd landly-server
```

### 2. Environment Setup

Copy the example environment file and configure your settings:

```bash
cp .env.example .env
```

Edit `.env` with your configuration:

```env
# Database
DATABASE_URL=postgres://landly-user:change-me-postgres@localhost:5432/landly

# Redis
REDIS_URL=redis://redis:change-me-redis@localhost:6379/

# JWT
JWT_SECRET=your-super-secret-jwt-key
JWT_EXPIRATION=3600

# OAuth (Google)
GOOGLE_CLIENT_ID=your-google-client-id
GOOGLE_CLIENT_SECRET=your-google-client-secret
OAUTH_GOOGLE_REDIRECT_URL=http://localhost:8080/api/user/oauth/google/callback

# Object storage (S3 / Cloudflare R2 / MinIO) - optional, uploads are disabled without it
S3_ENDPOINT_URL=http://localhost:9000
S3_ACCESS_KEY_ID=change-me-minio-user
S3_SECRET_ACCESS_KEY=change-me-minio-password
S3_BUCKET=landly-images
S3_REGION=auto
S3_PUBLIC_URL=http://localhost:9000/landly-images

# Server
FRONTEND_ORIGIN=http://localhost:5173   # CORS origin + OAuth redirect target
TRUST_PROXY_HEADERS=false               # true only behind a proxy that overwrites X-Forwarded-For
```

Only `DATABASE_URL`, `JWT_SECRET` and `JWT_EXPIRATION` are required. Everything else degrades
gracefully: without `REDIS_URL` the cache is a no-op (and Google OAuth cannot complete — it
keeps its PKCE verifier and login code in Redis), without the `S3_*` block image
upload/delete fail at runtime, and without the `GOOGLE_*` block the OAuth endpoints answer
`503 Service Unavailable`. Without `FRONTEND_ORIGIN`, CORS allows `*` and the OAuth callback
redirects to `http://localhost:5173`. Connection pools are tuned with the optional
`DB_POOL_*` / `REDIS_POOL_*` variables (see `.env.example`).

The server always listens on `0.0.0.0:8080` (`constants::BIND`); the `HOST`/`PORT` variables
that compose and `scripts/start.sh` export are currently not read by the binary.

`docker compose` additionally requires the service credentials `POSTGRES_PASSWORD`,
`REDIS_PASSWORD`, `MINIO_ROOT_USER`, `MINIO_ROOT_PASSWORD` and `JWT_SECRET` (see
`.env.example`) — there are no built-in defaults, and it refuses to start without them.
Generate them URL-safe (e.g. `openssl rand -hex 32`): the Postgres and Redis passwords are
embedded into the `DATABASE_URL`/`REDIS_URL` the server container receives.
Postgres, Redis and MinIO are published on `127.0.0.1` only (override with
`SERVICES_BIND_ADDRESS`), and so is the API itself (`BIND_ADDRESS`, port `EXTERNAL_PORT`,
default `8080`); the image bucket is anonymously readable, never writable.

### 3. Run with Docker (Recommended)

```bash
# Start all services
docker compose up -d

# View logs
docker compose logs -f landly-server

# Stop services
docker compose down
```

### 4. Local Development Setup

If you prefer to run locally:

```bash
# Install Diesel CLI
cargo install diesel_cli --no-default-features --features postgres

# Run database migrations
diesel migration run

# Start the development server
cargo run
```

## 🔗 API Documentation

### Interactive API Documentation

The API comes with comprehensive OpenAPI 3.1 documentation rendered by [Scalar](https://github.com/scalar/scalar) (via `utoipa-scalar`):

```url
http://localhost:8080/scalar
```

The raw spec is available at `http://localhost:8080/api-docs/openapi.json`, and the legacy `/swagger-ui` path redirects to `/scalar`.

This interactive documentation provides:

- **Complete API reference** with all endpoints
- **Request/response schemas** with examples
- **Built-in API client** to test endpoints directly
- **Authentication setup** for protected routes
- **Model definitions** for all data structures

All API endpoints are documented with `utoipa` and served via Scalar at `/scalar`. The spec is split per feature — each `<feature>/mod.rs` declares its own `#[derive(OpenApi)] pub struct ApiDoc`, and they're merged into a single doc in `build_openapi()` in `src/main.rs`.

### Authentication Endpoints

#### Traditional Auth

```http
POST /api/user/signup
Content-Type: application/json

{
  "username": "johndoe",
  "email": "john@example.com",
  "password": "securepassword123"
}
```

```http
POST /api/user/signin
Content-Type: application/json

{
  "email": "john@example.com",
  "password": "securepassword123"
}
```

Both return `{ "id", "username", "email", "token" }`; send the token as `Authorization: Bearer <token>`.

Signup v2 additionally accepts optional profile fields (`name`, `locale`, `here_as`, `home_country_id`, `avatar_color`) and a default corridor (`corridor_from_country_id`, `corridor_to_country_id`) — the user and the corridor are created in one transaction.

Credential rules:

- Emails are trimmed and lowercased; uniqueness is case-insensitive (`users_email_lower_key`).
- Passwords: 8 characters to 72 bytes (bcrypt limit). Usernames: 3–50 characters, no whitespace.
- Profile free text (name, city, bio, …) is length-capped.
- Signin answers the same `401 Invalid email or password` for an unknown email and a wrong password.
- There is no email confirmation yet.

#### OAuth 2.0 (Google)

```http
# 1. Start: sets an HttpOnly state cookie and redirects to Google (PKCE)
GET /api/user/oauth/google/login

# 2. Callback from Google: checks the state against the cookie, requires a verified
#    Google email, then redirects to
#    {FRONTEND_ORIGIN}/auth/google/callback?code=<single-use code, valid 60 s>
GET /api/user/oauth/google/callback?code=...&state=...

# 3. The frontend trades the code for the session ({ id, username, email, token })
POST /api/user/oauth/google/exchange
Content-Type: application/json

{ "code": "<code from the redirect>" }
```

The JWT never appears in a URL, and a Google login never links to an existing password account by email. All three endpoints answer `503 Service Unavailable` when the `GOOGLE_*` variables are not configured; the flow also needs Redis.

#### Rate limiting

Every client IP gets a burst of 100 requests, then 20 req/s across the whole API. The credential endpoints (`signin`, `signup`, `oauth/google/*`) are additionally limited to a burst of 10, then one request per 6 s. Over the limit the server answers `429 Too Many Requests`. Behind a reverse proxy set `TRUST_PROXY_HEADERS=true` (only if the proxy overwrites `X-Forwarded-For`/`Forwarded`), otherwise all clients share the proxy's IP.

### Profile

```http
# Current user profile with stats (places added, etc.)
GET /api/user/me
Authorization: Bearer <jwt_token>

# Update profile (name, bio, city, locale, here_as, ...)
PUT /api/user/me

# Update notification settings (free-form JSON object)
PUT /api/user/me/notifications

# Spoken languages
GET    /api/user/{user_id}/languages   # public
POST   /api/user/languages             # auth; language_ids are added for the caller
DELETE /api/user/languages             # auth; removes one of the caller's languages
```

### Corridors

A corridor is the user's "from country → to country" pair the map opens to.

```http
POST   /api/corridor/create            # { from_country_id, to_country_id, is_default? }
GET    /api/corridor/list
PUT    /api/corridor/set-default/{id}
DELETE /api/corridor/delete/{id}
GET    /api/corridor/stats/{id}        # live-place counters by org type + "new this week"
```

All corridor endpoints require authentication and operate only on the caller's corridors.

### Organization Management

```http
# List organisations (only status=live)
GET /api/organisation/list

# Geo search for the map: bbox or origin+radius, filters and sorting
GET /api/organisation/search?min_lat=52&min_lng=13&max_lat=53&max_lng=14
GET /api/organisation/search?lat=52.5&lng=13.4&radius_km=25&sort=nearest
#   filters: types=embassy,community  open_now=true  languages=Russian,English
#            verified=true  min_rating=4.5  added_by=volunteer  cost=free
#   sort:    nearest | recent | verified

# Fetch one organisation (openNow computed from opening_hours + timezone)
GET /api/organisation/fetch/{id}

# Create organisation (requires auth; new submissions get status=pending)
POST /api/organisation/create
Authorization: Bearer <jwt_token>
#   { name, google_place_id?, latitude?, longitude?, ... }
#   latitude/longitude come as a pair. With google_place_id the server takes the
#   point from Google (when GOOGLE_PLACES_API_KEY is set); one organisation per place.
#   Responses carry locationSource: google | user_pin | moderator.
#   Throttled: 429 over the per-IP / per-user rate limit, or over the quota
#   (regular users: at most 10 organisations waiting for moderation and 20
#   submissions per day; moderators are exempt). A place that already has an
#   organisation is a 422 with constraint organisations_google_place_id_key.

# Update / delete — only the creator or a moderator/admin
PUT    /api/organisation/update/{id}
DELETE /api/organisation/delete/{id}

# Count a visit (public)
POST /api/organisation/visit/{id}
```

When the creator (not a moderator) edits a `live` organisation, it goes back to `pending` and re-enters the moderation queue — approved content never changes unreviewed.

### Images

Organisation photos live in S3/R2 (MinIO locally); the API stores only the metadata.

The map is Google Maps. From Google the server keeps only `google_place_id` and a refreshable
copy of its coordinates; hours, photos and Google ratings are fetched by the client live with
the place ID (Google's terms do not allow storing them).

```http
POST   /api/images/upload/{organisation_id}   # auth; multipart/form-data
GET    /api/images/list/{organisation_id}
GET    /api/images/fetch/{id}
PUT    /api/images/set-primary/{id}           # auth
DELETE /api/images/delete/{id}                # auth
```

Uploads accept JPEG, PNG, WebP and GIF up to 10 MB; permissions are checked before the body is read. Only the organisation's creator or a moderator/admin may upload or set the primary image; an image can be deleted by its uploader, the organisation's creator or a moderator/admin. Upload and delete return `500` while object storage is unconfigured (`S3_*` unset).

### Countries

```http
# All countries
GET /api/common/countries

# Country page payload: country + live-place breakdown by org type
GET /api/common/countries/{id}
```

### Languages

```http
# public; ?name= (case-insensitive substring), limit (default 100, max 500), offset
# → [{ id, name, symbol }] ordered by name; `id` is what people's `language_ids`
#   and POST/DELETE /api/user/languages `languages_ids` reference
GET /api/common/languages
```

### Organisation types & Country connections

```http
GET  /api/common/org_types                    # public
POST /api/common/org_types                    # admin; { org_type, color, title, slug? }

GET    /api/country-connection/list           # filters: embassy_org_id, consulate_org_id, location_country_id
GET    /api/country-connection/fetch/{id}
POST   /api/country-connection/create         # admin
PUT    /api/country-connection/update/{id}    # admin
DELETE /api/country-connection/delete/{id}    # admin
```

### People / Helpers

A person is a recommended human helper with a claim-and-verify flow: `pending` (moderation) → `awaiting` (approved, claim link sent) → `confirmed` / `claimed` (linked an account) or `declined`. Hidden contacts (email/whatsapp) are **never** serialized until the person confirms — and then only per their privacy toggles.

```http
POST /api/person/create          # auth; requires consent_given + a contact; returns claimUrl for manual sending (send_via)
GET  /api/person/list            # public; confirmed/claimed only; filters: skills, city, language_ids
GET  /api/person/fetch/{id}      # public; contacts gated by status + privacy toggles
POST /api/person/vouch/{id}      # auth; one vouch per user

# Claim flow — PUBLIC, the token from the invite link is the credential:
GET  /api/person/claim/{token}           # preview for the recommended person
POST /api/person/claim/{token}/confirm   # -> confirmed (or claimed with a Bearer token); optional privacy toggles
POST /api/person/claim/{token}/decline   # -> declined
```

### Reviews

```http
POST   /api/review/create        # auth; exactly one of organisation_id/person_id; rating 1-5; one per author per target
GET    /api/review/list?organisation_id=|person_id=
DELETE /api/review/delete/{id}   # author or moderator
```

Creating/deleting a review atomically refreshes the target's `ratingAvg`/`reviewsCount`. People can disable reviews (`allow_reviews`).

### Saved / Bookmarks (all auth)

```http
POST   /api/saved/create         # { kind: org|person|country|corridor, target_id, note?, list_name? }
DELETE /api/saved/delete/{id}
GET    /api/saved/list?kind=
GET    /api/saved/counts         # per-kind counters for the Saved tab
```

### Community check-ins & Reports

```http
POST /api/organisation/checkin/{id}   # auth; { still_active?, tip? }; detail payload gains a `community` block
POST /api/report/create               # auth; { target_kind: org|person|conversation, target_id, reason }
```

### Moderation (moderator/admin role only)

```http
GET  /api/moderation/queue?kind=      # pending orgs + people, with submit-time auto-check flags and open report counts
POST /api/moderation/approve          # org -> live; person -> awaiting
POST /api/moderation/request-changes  # note REQUIRED; item stays pending
POST /api/moderation/reject           # org -> rejected; person -> declined
```

Submissions record automatic checks into the queue: duplicate-nearby (same name within ~1 km), phone format sanity, and creator trust (≥3 live orgs / approved people).

### Roles & Permissions

Every user has a role (`users.role`): `user` (default), `moderator`, or `admin`.

| Action | anonymous | user | moderator | admin |
|---|---|---|---|---|
| Browse (list/search/fetch), signup/signin, visit counter | ✓ | ✓ | ✓ | ✓ |
| Own profile, languages, corridors | — | ✓ | ✓ | ✓ |
| Add organisation (goes to moderation as `pending`) | — | ✓ | ✓ | ✓ |
| Edit/delete organisation | — | own only | any | any |
| Upload / set-primary organisation image | — | own organisation only | any | any |
| Delete organisation image | — | own upload or own organisation | any | any |
| Recommend people, vouch, review, save, report, check in | — | ✓ | ✓ | ✓ |
| Delete review | — | own only | any | any |
| Moderation queue and decisions | — | — | ✓ | ✓ |
| Manage system tables (organisation types, country connections) | — | — | — | ✓ |

Violations return `401 Unauthorized` (no/invalid token) or `403 Forbidden` (insufficient role/ownership). Invalid input — including database constraint violations such as a duplicate email — is `422 Unprocessable Entity`, with a generic message and the constraint name but never raw database details.

## 🗄️ Database Schema

### Core Tables

- **chats** - Chat links (legacy table, no API yet)
- **users** - User accounts (email unique case-insensitively), authentication and profile (name, bio, city, home country, locale `en/ru/uk`, `here_as`, RBAC `role`, notification settings)
- **user_providers** - OAuth provider linkages
- **corridors** - User corridors (from country → to country, one default per user)
- **organisations** - Organization entities (v2: moderation `status`, `created_by` ownership, `verified`, contacts, `services[]`/`languages[]`, `opening_hours` JSONB + `timezone`, `cost`, Google import fields, visit/rating counters)
- **organisation_types** - Organization classifications with stable `slug` (canonical: `embassy`, `business`, `helper`, `community`, `volunteer`)
- **countries** - Country master data (+`currency`, `phone_code`, `top_cities`)
- **languages** - Language master data
- **images** - Organisation photo metadata (the objects themselves live in S3/R2)
- **countries_connections** - Country relationships
- **countries_to_languages** - Country-language mappings
- **users_to_languages** - User language preferences
- **people** (+`people_to_languages`, `person_claim_tokens`, `person_vouches`) - Recommended helpers with hidden contacts, privacy toggles and the claim flow
- **reviews** - Polymorphic reviews (exactly one of org/person via CHECK), unique per author-target
- **saved_items** - Bookmarks (`kind` + `target_id` without FK), private notes and lists
- **org_checkins** - "Still active" community check-ins with tips
- **reports** - User reports on orgs/people/conversations
- **moderation_events** - Moderation audit trail incl. submit-time auto-check `flags`

## 🔧 Development

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

Tests are inline `#[cfg(test)]` modules next to the code:

- **Unit tests** run anywhere: usecases against hand-written stub repositories, presenters,
  domain enums and helpers.
- **DB-backed HTTP tests** (`http_tests` in each feature's `controllers.rs`) drive the real
  routing, auth middleware, usecases and SQL against a dedicated Postgres database. They run
  when `TEST_DATABASE_URL` points at a database whose name contains `test` and are skipped
  otherwise:

  ```bash
  docker exec landly_postgres psql -U landly-user -d landly -c "CREATE DATABASE landly_test"
  echo 'TEST_DATABASE_URL=postgres://landly-user:<password>@localhost:5432/landly_test' >> .env
  cargo test
  ```

- **Redis cache tests** run with `REDIS_TEST_URL` (use a scratch DB index, e.g.
  `redis://user:pass@localhost:6379/15`).

CI provides both services, so Codecov sees every test.

Guard tests also parse every feature `config.rs` and fail when a route is missing from the
OpenAPI spec or a mutating route is not behind authentication.

### Smoke Test (real stack)

With Postgres (+ Redis, MinIO) and `cargo run` up:

```bash
scripts/smoke.sh                  # auth gates, credentials, moderation flow, RBAC
SMOKE_IMAGES=1 scripts/smoke.sh   # + image upload/delete through MinIO
```

It creates throw-away users/organisations with a unique prefix and removes them on exit;
roles and cleanup go through `docker exec landly_postgres psql`. It uses 6 of the 10
credential requests the rate limiter allows per burst, so don't loop it.

### Database Migrations

```bash
# Create new migration
diesel migration generate migration_name

# Run migrations
diesel migration run

# Rollback last migration
diesel migration revert
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Check for security vulnerabilities (ignores with reasons live in .cargo/audit.toml)
cargo audit
```

CI runs `cargo fmt -- --check`, build and tests (plus a separate build of `scripts/crates`), `cargo clippy --all-targets --all-features -- -D warnings`, `cargo audit` for both lockfiles, and tarpaulin coverage uploaded to Codecov.

## 📊 Monitoring & Health Checks

### Health Check Endpoint

```http
GET /api/healthcheck
```

This is a liveness probe: it answers `200` whenever the process is serving requests and does
not query Postgres or Redis.

### Container Health Checks

`docker compose` adds its own checks: `pg_isready` for Postgres, an authenticated `PING` for
Redis, MinIO's `/minio/health/live`, and `curl /api/healthcheck` for the server.

## 📜 Scripts

The `scripts/` folder holds tooling used at container start and for seeding:

- **`scripts/start.sh`** — container entrypoint: waits for Postgres (with a configurable `DB_WAIT_TIMEOUT`), runs `diesel migration run`, loads countries on first boot, then execs the server.
- **`scripts/crates/`** — a separate Cargo workspace that links the main crate as a library:
  - `country_parser` — merges `countries.json` + `countries.geojson` into `merged_countries.json`
  - `country_loader` — loads `merged_countries.json` into the `countries` table (usage: `country_loader <path>`; safe to re-run, exits non-zero only on total failure)
- **`scripts/data/`** — `merged_countries.json` (236 countries) and `seed_test_data*.sql` with sample organisations/connections for manual testing (`psql "$DATABASE_URL" -f scripts/data/seed_test_data.sql`).

> When you change `Country`/`CreateCountry` in `src/data/models.rs`, rebuild `scripts/crates` too — it compiles against the main crate and breaks silently otherwise (it is not covered by `cargo test` at the repo root).

## 🐳 Docker Configuration

### Services

- **landly-server**: Main application container (healthcheck hits `/api/healthcheck`)
- **db**: PostgreSQL 17 database
- **redis**: Redis 7 cache server (healthcheck is an authenticated `PING`)
- **minio** + **minio-init**: local S3-compatible object storage; the init sidecar creates the `landly-images` bucket with anonymous read-only access (requires `minio.license` in the repo root)

`docker/` additionally holds standalone single-service compose files (`docker-compose-postgre.yml`, `-redis.yml`, `-minio.yml`) for quick local experiments; they use development defaults and are not meant for anything shared.

### Production Deployment

For production deployment, consider:

1. **Security**: Use secrets management for sensitive data
2. **Scaling**: Use container orchestration
3. **Monitoring**: Add logging and metrics collection
4. **Backup**: Implement database backup strategies
5. **SSL/TLS**: Configure HTTPS termination

## ✍️ Authorship

Endpoint handlers carry `// [authorship]` comments: the original codebase is human-written, and the design-handoff v2 rework (corridors, people, reviews, saved items, reports, moderation, geo search, profile) plus the 2026 security hardening were written with Claude. The full per-endpoint map is in [docs/authorship.md](docs/authorship.md).

## 🤝 Contributing

1. Clone the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Ensure all tests pass (`cargo test`)
- Add tests for new features
- Update documentation as needed

## 📄 License

This project is licensed under the MIT License.

## 🙏 Acknowledgments

- [Actix Web](https://actix.rs) - Amazing web framework
- [Diesel](https://diesel.rs) - Excellent ORM
- [Rust Community](https://www.rust-lang.org/community) - For the incredible ecosystem

---

Made with ❤️ and 🦀 Rust
