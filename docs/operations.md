# Operations: CI, scripts workspace, Docker

Moved out of AGENTS.md. Read before touching `.github/workflows/`, `scripts/`, `docker-compose.yml`, `Dockerfile`, `docker/` or `.cargo/audit.toml`.

## CI (.github/workflows)

- `rust.yml` — fmt check, build, tests, **plus a separate `scripts-workspace` job** (the scripts crates are not covered by the root build); coverage via tarpaulin → Codecov. All jobs use `Swatinem/rust-cache`.
- `clippy.yml` — `cargo clippy --all-targets --all-features -- -D warnings`.
- `audit.yml` — `cargo audit` for both lockfiles (root + scripts/crates), on dependency changes and weekly. Ignored advisories live in `.cargo/audit.toml`, each with a reason and an exit condition (currently `RUSTSEC-2026-0258`, h2 0.3 via actix-http — unreachable because the server is HTTP/1.x only; drop the ignore if HTTP/2 is ever enabled).
- `dependencies.yml` — weekly `cargo update` PR for both workspaces.
- `release.yml` — builds and attaches the linux binary on `v*` tags.

## Scripts workspace

`scripts/crates` is a **separate Cargo workspace** (`country_parser`, `country_loader`) that depends on the main crate as a library (`landly-server = { path = "../.." }`). It is NOT built by `cargo build`/`cargo test` at the repo root — after changing structs it consumes (notably `Country`/`CreateCountry` in `src/data/models.rs`), verify it still compiles: `cd scripts/crates && cargo build`. The Docker image builds it and `start.sh` runs `country_loader` on first boot when the `countries` table is empty. Seed SQL for manual testing lives in `scripts/data/seed_test_data*.sql`.

## Docker

- The server container healthcheck must target `/api/healthcheck` (there is no `/health` route); `curl` is installed in the runtime image for this. It is a liveness probe only — it does not touch Postgres or Redis.
- The server listens on the hardcoded `constants::BIND` (`0.0.0.0:8080`); `HOST`/`PORT` are exported by `start.sh` and compose but not read by the binary, so leave `PORT` at 8080 (compose maps `${BIND_ADDRESS:-127.0.0.1}:${EXTERNAL_PORT:-8080}` → `${PORT}`).
- `JWT_EXPIRATION` is required by `utils/token.rs` and is passed through docker-compose (default 3600). The old docs mentioned a nonexistent `JWT_EXP_SECS`.
- Service credentials (`POSTGRES_PASSWORD`, `REDIS_PASSWORD`, `MINIO_ROOT_USER`/`MINIO_ROOT_PASSWORD`, `JWT_SECRET`) are required via `${VAR:?}` — never reintroduce hardcoded defaults. db/redis/minio ports bind to `SERVICES_BIND_ADDRESS` (default `127.0.0.1`), and the bucket is `mc anonymous set download` (read-only; `public` would allow anonymous uploads).
- The redis healthcheck uses an authenticated `PING` as `REDIS_USER` — plain `redis-cli incr ping` always exits 0 even on NOAUTH and mutates a key. Redis auth comes from the ACL file only (`--requirepass` is ignored with `--aclfile`), which must keep `user default off`.
- Shell snippets in compose (`redis` command, `minio-init` entrypoint) read credentials from the container environment as quoted `"$$VAR"` — never interpolate `${VAR}` secrets into shell text. Postgres/Redis passwords must be URL-safe (they are embedded in `DATABASE_URL`/`REDIS_URL`).
- `docker/docker-compose-{postgre,redis,minio}.yml` are standalone single-service files for local experiments (the MinIO one uses the `minioadmin` defaults); the root `docker-compose.yml` is the real stack and the one the rules above apply to.

## Local data inspection (MCP servers)

### Postgres — `landly-postgres`

`.mcp.json` registers the project MCP servers (Claude Code asks once to approve them). **`landly-postgres`** It is [Postgres MCP Pro](https://github.com/crystaldba/postgres-mcp) started by `scripts/dev/mcp-postgres.sh`:

- pinned `postgres-mcp==0.3.0` via `uvx` on Python 3.12 with `mcp<2` (0.3.0 breaks on Python 3.14 builds of `pglast` and on mcp 2.x);
- `--access-mode=restricted`: queries run in read-only transactions and are validated before execution;
- connects as the **`landly_readonly`** role through `MCP_DATABASE_URL` in `.env`, and refuses non-local hosts. The role (created by `scripts/dev/setup-readonly-db.sh` → `scripts/dev/create-readonly-role.sql`) has `SELECT` only, `default_transaction_read_only = on`, a 30 s statement timeout, and **no access to `users.password_hash` or `person_claim_tokens.token`**.

Setup on a new machine: `docker start landly_postgres` (or `docker compose up -d db`), then `scripts/dev/setup-readonly-db.sh` — it generates the role password once and appends `MCP_DATABASE_URL` to `.env`. Re-run it after a migration adds a table or column holding secrets (new tables are readable automatically via default privileges; hide secret columns in the SQL file). Tools: `list_schemas`, `list_objects`, `get_object_details`, `explain_query`, `analyze_query_indexes`, `analyze_workload_indexes`, `analyze_db_health`, `get_top_queries`, `execute_sql`.

### Redis — `landly-redis`

The official [redis/mcp-redis](https://github.com/redis/mcp-redis) server, started by `scripts/dev/mcp-redis.sh`: pinned `redis-mcp-server==0.5.1` via `uvx` on Python 3.12, connecting with `--url` from `MCP_REDIS_URL` (`--url` wins over the `REDIS_*` variables the server would read through dotenv, so the admin `REDIS_URL` is never used; non-local hosts are refused).

The server has **no read-only mode** — its `set`/`delete`/`expire`/`hset`/`publish`… tools are blocked by the ACL user **`landly_readonly`** that `scripts/dev/setup-readonly-redis.sh` creates: `-@all +@read -keys` plus `info`/`ping`/`hello`/`auth`/`select` and the harmless `client` subcommands, keys limited to the cache namespaces `org:* cc:* img:* cor:* per:* common:*`, no pub/sub channels. `oauth:*` (PKCE verifiers, single-use login codes) is not readable; key **names** are still visible through `SCAN`, so an in-flight local OAuth login code (60 s) can show up by name.

The compose Redis regenerates its ACL file on every container start, so the user is lost on restart: the launcher re-runs the setup script before every start (needs the container running; otherwise the server starts and its tools fail until Redis is up and the session restarts). When a new `CacheKeys` namespace appears, add it to `CACHE_NAMESPACES` in the setup script — `sync-docs` checks this.

### MinIO / S3 — no MCP on purpose

The official MinIO MCP server (`minio/mcp-server-aistor`) was archived in May 2025, and the remaining MinIO/S3 MCP servers are small unmaintained projects — not worth handing storage credentials to. Inspect objects with the MinIO console (`http://localhost:9001`) or `mc` from the `minio/mc` image instead; `scripts/smoke.sh` with `SMOKE_IMAGES=1` covers upload → public read → delete.

