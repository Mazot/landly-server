# Environment variables

Moved out of AGENTS.md. Read before adding/renaming an `env::var`, changing `.env.example`, compose `environment:` or `scripts/start.sh`. Every variable the code reads must be listed here (`sync-docs` checks it).


| Variable | Required | Description |
|---|---|---|
| `DATABASE_URL` | yes | PostgreSQL connection string |
| `REDIS_URL` | no | Redis connection string; without it the repository cache is a no-op and `GET /common/*` is served uncached (Google OAuth needs it) |
| `JWT_SECRET` | yes | JWT signing secret |
| `JWT_EXPIRATION` | yes | JWT expiry in seconds |
| `GOOGLE_CLIENT_ID` | no | OAuth Google client ID |
| `GOOGLE_CLIENT_SECRET` | no | OAuth Google client secret |
| `OAUTH_GOOGLE_REDIRECT_URL` | no | Google OAuth callback URL |
| `S3_ENDPOINT_URL` | no | S3/R2 endpoint; storage disabled if absent |
| `S3_ACCESS_KEY_ID` | no | S3/R2 access key |
| `S3_SECRET_ACCESS_KEY` | no | S3/R2 secret key |
| `S3_BUCKET` | no | S3/R2 bucket name |
| `S3_REGION` | no | Region (defaults to `"auto"`) |
| `S3_PUBLIC_URL` | no | Public base URL for object access |
| `FRONTEND_ORIGIN` | no | Allowed CORS origin (default `*`) and the OAuth redirect target (default `http://localhost:5173`) |
| `TRUST_PROXY_HEADERS` | no | `true` = rate limiter keys clients by `Forwarded`/`X-Forwarded-For` (only behind a proxy that overwrites them); default: TCP peer IP |
| `DB_POOL_MAX_SIZE`, `DB_POOL_MIN_IDLE`, `DB_POOL_MAX_LIFETIME_SECS`, `DB_POOL_IDLE_TIMEOUT_SECS`, `DB_POOL_CONNECTION_TIMEOUT_SECS` | no | r2d2 Postgres pool tuning (`utils/db.rs`) |
| `REDIS_POOL_*` (same five suffixes) | no | r2d2 Redis pool tuning (`utils/redis.rs`) |
| `RUST_LOG` | no | `env_logger` filter (compose default `info`) |
| `TEST_DATABASE_URL` | no | Database for the DB-backed tests (`src/test_support.rs`); its name must contain `test`. Read from the env or that single key in `.env`; tests skip without it |
| `REDIS_TEST_URL` | no | Enables the real-Redis cache tests in `utils/cache.rs` (use a scratch DB index such as `/15`); skipped without it |
| `HOST`, `PORT` | — | Set by compose/`start.sh` but **not read** by the binary — it binds `constants::BIND` (`0.0.0.0:8080`) |

Tooling-only (not read by the binary): `MCP_DATABASE_URL` / `MCP_REDIS_URL` — read-only connection strings for the `landly-postgres` / `landly-redis` MCP servers, written to `.env` by `scripts/dev/setup-readonly-db.sh` / `scripts/dev/setup-readonly-redis.sh` (see docs/operations.md).

Compose-only (consumed by `docker-compose.yml`, not by the binary): `POSTGRES_USER`/`POSTGRES_PASSWORD`/`POSTGRES_DB`, `REDIS_USER`/`REDIS_PASSWORD`, `MINIO_ROOT_USER`/`MINIO_ROOT_PASSWORD`, `SERVICES_BIND_ADDRESS`, `BIND_ADDRESS`, `EXTERNAL_PORT`, `*_PORT` overrides, `MINIO_LICENSE_PATH`. `start.sh` additionally reads `DB_HOST`/`DB_PORT`/`DB_USER`/`DB_WAIT_TIMEOUT`.
