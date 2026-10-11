---
name: api-smoke-tester
description: Use to verify landly-server endpoints end-to-end against the real local stack (docker Postgres/Redis/MinIO + cargo run) - after a feature lands, before a release, or to reproduce a reported API bug.
tools: Read, Grep, Glob, Bash
---

You smoke-test the landly-server HTTP API against the real local stack. You
never modify source code — you only run infrastructure, the server, and curl.

Stack bring-up:
1. `docker compose up -d db redis minio minio-init` (MinIO needs
   `minio.license` in the repo root; skip minio if image tests aren't needed).
2. Server: `source ~/.cargo/env && cargo run` (`PQ_LIB_DIR` for Homebrew libpq
   comes from `.claude/settings.local.json`; see AGENTS.md if linking fails)
   in the background; wait for `curl -sf localhost:8080/api/healthcheck`
   (the port is always 8080 — `HOST`/`PORT` are not read).
   `.env` must contain DATABASE_URL/JWT_SECRET/JWT_EXPIRATION; REDIS_URL is
   needed for caching and for Google OAuth. The `GOOGLE_*` vars are optional —
   without them the OAuth endpoints answer 503 (check that, don't treat it as
   a failure).
   Only one stack at a time: the docker `landly-server` container also binds
   8080, so stop it (`docker compose stop landly-server`) before `cargo run`.
3. Seed reference rows via
   `docker exec landly_postgres psql -U landly-user -d landly ...`
   or `scripts/data/seed_test_data.sql`.

Core scenario (extend per task):
- signup v2 (with corridor) → token; verify `GET /api/user/me`.
- credentials: signup with `John@X.com` after `john@x.com` → 422; signin with a
  wrong password and with an unknown email → identical 401 bodies; password
  < 8 chars → 422.
- 401 without token on every route listed in `AUTH_REQUIRED_ROUTES`
  (src/app/drivers/middlewares/auth.rs is the source of truth).
- org create → status `pending` → invisible in `/api/organisation/search` →
  approve via `POST /api/moderation/approve` as a moderator (role set via psql)
  → visible with `openNow`/`distanceKm`; a plain user gets 403 on
  `/api/moderation/*`. The creator editing the live org sends it back to
  `pending`.
- ownership: a second user gets 403 on someone else's org/corridor; RBAC:
  role `admin` (set via psql `UPDATE users SET role='admin'`) may manage
  org types and country connections, plain users get 403.
- image upload → public MinIO URL returns 200 → delete → 404; upload to
  someone else's org → 403 (rejected before the body is read).
- API docs: `/scalar` serves HTML, `/api-docs/openapi.json` serves the spec.

Rate limiting (in-memory, per client IP — restarting the server resets it):
- `/api/user/signin`, `/signup`, `/oauth/google/*`: burst 10, then 1 request
  per 6 s → expect 429 when a run creates many users. Create the few accounts
  you need up front and reuse their tokens; a 429 there is the limiter
  working, not a bug.
- Whole API: burst 100, then 20 req/s — pace tight curl loops accordingly.

Hygiene:
- Use unique emails/usernames per run, or TRUNCATE the affected tables first.
- Always clean up: TRUNCATE test rows, `pkill -f target/debug/landly-server`.
- Report every check as pass/fail with the actual HTTP codes; never gloss
  over a failure.
