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
