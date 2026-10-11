---
name: add-endpoint
description: Add, change or remove an HTTP endpoint in an EXISTING landly-server feature (route + controller + usecase + OpenAPI + auth + RBAC + tests + docs). Use whenever a task touches a route in src/app/features/*/config.rs — "add an endpoint", "expose X via the API", "new route for Y", "rename/remove /api/...". For a whole new feature module use the feature-scaffolder agent instead.
---

# Add an endpoint to landly-server

A route in this repo is spread over ~10 places; forgetting one is how the docs,
the OpenAPI spec and the auth table drift (or how a route ships unprotected).
Work through the steps in order and tick every box in the final report.

Reference implementations to copy from — read the matching one first:
- authenticated CRUD on own data: `src/app/features/corridor/`
- role gate / author-or-moderator: `src/app/features/review/` (`delete_review`)
- ownership gate: `OrganisationUsecase::ensure_can_manage` (`organisation/usecases.rs`)
- admin gate: `ensure_admin` (`country_connection/usecases.rs`)
- query params + pagination: `review::controllers::list_reviews`

## 0. Decide before writing code

Answer these and state the answers in your first message to the user:

| Question | Consequence |
|---|---|
| Public or authenticated? Mutating routes are authenticated unless the request itself carries the credential (like `/person/claim/{token}`, `/user/oauth/google/exchange`). | step 4 |
| Who may call it: any user / owner (`created_by`, `author_id`) / moderator / admin? | step 3 |
| Does it write data that something caches (`org:*`, `cc:*`, `img:*`, `cor:*`, `per:*`, `common:*`)? | step 3 |
| Credential-like or abuse-prone (login, code exchange, mass creation)? | step 5 |
| Does it create moderated content (org/person)? | step 3 |
| Who wrote the code it touches (for `[authorship]`)? | step 2 |

## 1. Request DTO — `<feature>/requests.rs`

- JSON body: `#[derive(Debug, Deserialize, Serialize, ToSchema)]`.
- Query string: add `IntoParams` too; pagination is `limit`/`offset: Option<i64>`, clamped in the controller (`.unwrap_or(20).clamp(0, 100)`).
- **Never** accept `user_id`/`author_id`/`created_by` from the body — the caller comes from the JWT (step 2).
- Enum-like fields are `String` here and validated with the entity's `TryFrom<&str>` in the usecase (unknown value → 422).
- Free text gets a length cap in the usecase (`validate_max_chars` pattern from `user/entities.rs`).

## 2. Controller — `<feature>/controllers.rs`

```rust
// [authorship] AI-generated (Claude).
#[utoipa::path(
    post,                                   // get | post | put | delete
    path = "/<feature>/<action>/{id}",      // WITHOUT the /api prefix
    context_path = "/api",
    params(("id" = Uuid, Path, description = "...")),   // or params(SomeQueryRequest)
    request_body = SomeRequest,             // only for JSON bodies
    responses(
        (status = 200, description = "...", body = super::presenters::SomeContent),
        (status = 401, description = "Unauthorized", body = AppError),        // authenticated routes
        (status = 403, description = "Forbidden", body = AppError),           // role/ownership gate
        (status = 404, description = "Not found", body = AppError),
        (status = 422, description = "Unprocessable entity", body = AppError),
        (status = 500, description = "Internal server error", body = AppError)
    ),
    tag = "<Feature tag as in mod.rs>"
)]
pub async fn do_something(
    req: HttpRequest,
    state: Data<AppState>,
    id: Path<Uuid>,
    form: Json<SomeRequest>,
) -> Result<HttpResponse, AppError> {
    let caller = caller_user_id(&req)?;     // the module's private helper

    state.di_container.<feature>_usecase.do_something(DoSomethingUsecaseInput {
        caller_user_id: caller,
        target_id: id.into_inner(),
        // ...fields from form
    })
}
```

Rules:
- Controllers stay thin: extract, clamp pagination, build the usecase input, call. No SQL, no role checks, no manual JSON building.
- `[authorship]` comment: brand-new handler → `AI-generated (Claude).`; editing a human-written handler → change it to `Human-written (original codebase); extended by AI (Claude): <what>` (skip for the `images` feature, which is intentionally unmarked).
- List only the status codes the handler can really return.

## 3. Usecase (+ repository, presenter, entities)

- Usecase input struct `<Action>UsecaseInput` next to the usecase.
- Validation first (`TryFrom`, length caps, cross-field rules) → 422 **before** touching the repository.
- RBAC lives HERE, never in the middleware or controller:
  - owner-or-moderator: fetch the row, compare `created_by`/`author_id`, else `repo.fetch_user_role(caller)?.is_moderator()` → `AppError::Forbidden`.
  - role-only: `ensure_admin` / `ensure_moderator` pattern.
  - If the feature repo has no `fetch_user_role` yet, add it to the trait delegating to `User::fetch_role`.
- Domain rules to respect if you touch these areas (see AGENTS.md → "Domain rules"): orgs are created `pending` and list/search show only `live`; a creator editing a `live` org sends it back to `pending`; person contacts only via `PersonContent::from_gated`; review counters recomputed with AVG/COUNT in the same transaction; moderated creates call `ModerationRepository::record_submitted` best-effort.
- Repository: new trait method + impl; SQL in the entity or repo impl as the feature already does. On every write, `invalidate_pattern(&CacheKeys::<feature>_pattern())`; new keys go through `CacheKeys` in `src/utils/cache.rs` (never ad-hoc strings).
- Don't pre-check uniqueness with a SELECT — let the DB constraint fail; `src/error.rs` maps it to 422.
- Presenter: return `HttpResponse` via a `*Content` struct (`#[serde(rename_all = "camelCase")]`, `ToSchema`); nothing private (contacts, tokens, emails of others) in the payload.
- If you add a trait method, every hand-written stub repo in `#[cfg(test)]` modules implementing that trait must get it too — `cargo test` will tell you which.

## 4. Route + auth

1. `<feature>/config.rs`: `.route("/<action>/{id}", web::post().to(do_something))` inside the feature scope. Path params in the route must match the `Path<...>` extractor and the `utoipa` `params`.
2. Authenticated route → add to `AUTH_REQUIRED_ROUTES` in `src/app/drivers/middlewares/auth.rs`:
   ```rust
   AuthRequiredRoute { path: "/api/<feature>/<action>/{id}", method: Method::POST },
   ```
   - full path **with** `/api`; method must match;
   - name the param `{id}` — `test_every_auth_required_route_matches_a_real_path` only substitutes `{id}`;
   - a trailing `/` means prefix match (images style) — don't use it for new routes;
   - bump the array length `[AuthRequiredRoute; N]` and `test_auth_required_routes_count`.
3. Public route: do nothing in `auth.rs` (auth is opt-in; `SKIP_AUTH_ROUTES` is NOT consulted). If it must stay public on purpose, add a guard test like `test_person_claim_routes_are_not_auth_required`.

## 5. Rate limiting (only if step 0 said so)

Wrap the single route with the shared limiter passed from `main`, like `user/config.rs`:
`web::post().to(handler).wrap(Governor::new(auth_limit))`. The feature's config then takes the `RateLimitConfig` as an argument (closure pattern) — never build a limiter inside the feature or the `HttpServer::new` closure.

## 6. OpenAPI — `<feature>/mod.rs`

Add the handler to `paths(...)` and every new request/response type to `components(schemas(...))` of the feature's `ApiDoc`. Never touch the root `#[openapi]` in `main.rs`. Removing/renaming a route → remove/rename it here too.

## 7. Tests (inline `#[cfg(test)]`, no database)

Minimum for a new endpoint:
- happy path through the usecase with the stub repo (assert the write was recorded);
- each gate: wrong owner / insufficient role → `Forbidden`, invalid input → `UnprocessableEntity`, and assert the stub repo recorded **no** write;
- new enum values: round-trip `as_str`/`TryFrom` + unknown value;
- presenter: new `Content` fields map correctly and nothing private leaks.
Copy the `StubRepo` + `Mutex<Vec<_>>` pattern from `review/usecases.rs`.

## 8. Docs

- `README.md`: add the route to its feature's ```http block with a short comment (`# auth; ...`, `# admin`, `# public`); update the "Roles & Permissions" table if a new permission rule appeared.
- `AGENTS.md`: RBAC table (new rule), "Domain rules to preserve" (new invariant), authorship map (new AI-generated endpoint).
- `.github/copilot-instructions.md` only if a convention changed.

## 9. Verify

Run, and paste the real results into the report (the PostToolUse hook already rustfmt-s edited files):

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Cross-check that the new route appears everywhere:

```bash
grep -rn "<action>" src/app/features/<feature>/config.rs src/app/features/<feature>/mod.rs src/app/drivers/middlewares/auth.rs README.md
```

If `Country`/`CreateCountry` or other structs used by `scripts/crates` changed: `cd scripts/crates && cargo build`.
For an end-to-end check against the real stack, offer the `api-smoke-tester` agent.

## Final report

End with this checklist, each line ✓ / ✗ / n/a with a reason for n/a:

```
- request DTO (no caller id in body)
- controller + #[utoipa::path] + [authorship]
- usecase validation + RBAC
- repository / cache invalidation
- presenter (no private data)
- route in config.rs
- AUTH_REQUIRED_ROUTES + count test   (or: public on purpose + guard test)
- rate limit
- ApiDoc paths + schemas
- tests (happy path + every gate)
- README / AGENTS.md
- fmt / clippy / test results
```
