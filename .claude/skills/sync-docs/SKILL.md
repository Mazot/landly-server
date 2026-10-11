---
name: sync-docs
description: Bring landly-server's documentation back in line with the code — README.md, AGENTS.md, docs/*.md, .github/copilot-instructions.md, .claude/agents/*.md, .claude/skills/*/SKILL.md and the per-feature OpenAPI ApiDocs. Use when asked to "update/sync/check the docs", "обнови MD файлы", after a batch of feature or security work, before a release, or when a doc claim looks wrong.
---

# Sync docs with the code

The code is the source of truth. This skill finds every place where the docs
say something the code no longer does (or never did), and fixes the **docs**.

## 1. Run the mechanical check first

```bash
python3 .claude/skills/sync-docs/check_docs.py
```

It is read-only and exits 1 on any mismatch. It covers:

| Check | Compares |
|---|---|
| Routes | every `.route(...)` in `src/app/features/*/config.rs` (+ healthcheck in `main.rs`) ↔ `METHOD /api/...` mentions in README.md |
| OpenAPI | each route has a `#[utoipa::path]` with the same method+path; each annotated handler is in its feature `ApiDoc` `paths(...)`; each feature `ApiDoc` is merged in `build_openapi()` |
| Auth | every `AUTH_REQUIRED_ROUTES` pattern matches a real route; every non-GET route is protected or listed in `INTENTIONALLY_PUBLIC_MUTATING_ROUTES` (auth.rs tests) |
| Env vars | every `env::var(...)` the non-test code reads ↔ docs/environment.md |
| Features | `src/app/features/*` ↔ "Current features:" in AGENTS.md and copilot-instructions.md |
| Authorship | every `// [authorship] AI-generated` handler ↔ the AI-generated line of docs/authorship.md |

Fix what it reports, rerun until it is clean. If a report is a false positive
caused by the script's parsing (not by the docs), fix the **script** in the same
change and say so — don't paper over it in the docs.

## 2. Then check by reading what the script cannot

Go through `git log --oneline <last docs sync>..HEAD` (if unknown, the last ~30
commits) and the diff of `src/`, `migrations/`, `docker-compose.yml`,
`Dockerfile`, `scripts/`, `.github/workflows/`, `.cargo/`, `Cargo.toml`, then
verify each item below against the code — open the file, don't trust memory:

- **RBAC table** (AGENTS.md + README "Roles & Permissions") ↔ the `ensure_*` / `is_moderator()` / `created_by` checks in usecases.
- **Domain rules** in AGENTS.md ↔ usecases/entities they name (statuses, moderation flow, claim flow, review counters, OAuth flow, credential rules). Every function or test named in the docs must still exist: `grep -rn <name> src`.
- **Request/response shapes** shown in README (`{ id, username, email, token }`, query params, enum values) ↔ `requests.rs` / `presenters.rs` (mind `#[serde(rename_all = "camelCase")]`).
- **Limits & constants**: rate limits (`rate_limit.rs`), upload size/types (`images/usecases.rs`), password/username/text limits (`user/entities.rs`), cache TTLs, `constants::BIND`.
- **Env var semantics**: required vs optional, defaults (`unwrap_or`), compose-only vars, `.env.example` — not just the names the script checks.
- **Docker/CI**: services, healthchecks, ports, `${VAR:?}` credentials, workflow jobs, `.cargo/audit.toml` ignores.
- **Tooling docs**: `.claude/agents/*.md`, `.claude/skills/*/SKILL.md`, `.claude/settings.json` hooks described in AGENTS.md — commands, paths and env they mention must still work.
- **Versions**: Rust MSRV badge/prereq (highest `rust-version` among locked deps, e.g. check `aws-sdk-s3`, `actix-web`), Postgres/Redis images.
- **Duplication**: copilot-instructions.md is a condensed AGENTS.md — every rule it states must agree with AGENTS.md.

## 3. Rules while editing

- Edit docs only. When the code looks wrong rather than the docs (a documented
  env var that is never read, a route missing from `ApiDoc`, an unprotected
  mutating route), **do not silently change behaviour**: document what the code
  actually does, and list it under "Code issues" in the report for the user to
  decide. Exception: adding a missing handler to its feature `ApiDoc`
  `paths(...)` / `schemas(...)` is a docs fix — do it, then run `cargo build`.
- Match each file's existing style and density; don't rewrite sections that are
  still correct.
- README is for API consumers and operators; AGENTS.md for agents changing the
  code (kept lean: rarely needed reference lives in `docs/` — operations,
  environment, authorship — with a pointer in AGENTS.md); copilot-instructions.md
  is a condensed AGENTS.md. Put a fact where its
  reader looks, and keep the copies consistent.
- Keep docs/authorship.md and `// [authorship]` comments consistent;
  never invent authorship — derive it from the comments / commit history.
- If you changed a parser in `check_docs.py`, explain why. Never add a route to
  `INTENTIONALLY_PUBLIC_MUTATING_ROUTES` to silence a report — that is a security decision for the user.
- Routes ↔ OpenAPI and routes ↔ auth are also enforced by `cargo test` (`main.rs` tests, auth.rs tests); the script repeats them so a docs pass sees everything in one run.

## 4. Verify and report

```bash
python3 .claude/skills/sync-docs/check_docs.py   # must be clean
git diff --stat
```

Report to the user, briefly:
1. **Fixed in docs** — grouped by file, one line per wrong/missing claim (what the docs said → what the code does).
2. **Code issues found** (not fixed) — with `file:line` and a suggested fix.
3. **Not verified** — anything you couldn't check (e.g. needs the running stack).

Don't commit unless asked.
