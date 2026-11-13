# scripts/crates

Command-line tools for landly-server. This is a **separate Cargo workspace**: it
depends on the server as a library (`landly-server = { path = "../.." }`) so the
tools share its models, database pool and S3 client, but `cargo build` /
`cargo test` at the repository root do not build it.

| Binary | Purpose | Touches |
|---|---|---|
| [`country_parser`](#country_parser) | Builds `merged_countries.json` from two public datasets | local files only |
| [`country_loader`](#country_loader) | Loads `merged_countries.json` into the `countries` table | Postgres |
| [`language_loader`](#language_loader) | Loads the countries' languages into `languages` + `countries_to_languages` (idempotent) | Postgres |
| [`orphan_images`](#orphan_images) | Finds (and optionally deletes) image objects no database row points to | Postgres (read) + S3 |

```bash
cd scripts/crates
cargo build                      # all three
cargo run --bin <name> -- <args> # one of them
```

Everything below runs from this directory. The tools read `.env` (via `dotenv`)
and the process environment; every variable is described in
[`docs/environment.md`](../../docs/environment.md).

> **Keep it compiling.** The tools use `Country` / `CreateCountry` from
> `src/data/models.rs` and the S3 / cleanup code in `src/utils/`. After changing
> any of them run `cargo build` here (CI job `scripts-workspace` does too), and
> `cargo audit --file scripts/crates/Cargo.lock` after changing dependencies —
> this workspace has its own `Cargo.lock`.

---

## country_parser

Merges `bin/country_parser/data/countries.json` (names, capitals, flags,
ISO codes) with `bin/country_parser/data/countries.geojson` (borders) into one
list of countries, matching on the ISO 3166-1 alpha-2 or alpha-3 code. Countries
without a border in the GeoJSON are dropped.

```bash
cd bin/country_parser && cargo run
# -> Merged N countries, writes ./data/merged_countries.json
```

It takes no arguments and uses **relative paths**, so run it from
`bin/country_parser/`. The committed result lives in
[`scripts/data/merged_countries.json`](../data/merged_countries.json) — copy the
regenerated file there when the source data changes. Normally you never run the
parser: the loader reads the committed file.

Output shape (`MergedCountry`):

```json
{ "name": "Georgia", "capital": "Tbilisi", "flag": "🇬🇪", "geo_json": { "type": "Polygon", "coordinates": [] } }
```

`country_parser` is also a library: `country_loader` imports `MergedCountry`
from it.

## country_loader

Inserts every entry of a `merged_countries.json` into the `countries` table
through `Country::create`.

```bash
cargo run --bin country_loader -- ../data/merged_countries.json
```

- Needs `DATABASE_URL`; the schema must already be migrated (`diesel migration run`).
- Fields the dataset does not have (`currency`, `phone_code`, `top_cities`,
  `description`) stay empty — fill them through the API or SQL.
- A failing row is printed and counted but does not abort the run.
- Exit codes: `0` finished (even with some failed rows), `1` bad arguments or
  unreadable file, `2` **every** row failed (wrong schema, dead database).

In Docker, `scripts/start.sh` runs it once on first boot, when the `countries`
table is empty (the file ships in the image as `/app/country_data/`).

## language_loader

Loads the languages of a `merged_countries.json` (`country_parser` keeps each country's
official languages from `countries.json`: ISO 639-3 code + English name) into `languages`
(code in `symbol`) and links them in `countries_to_languages`. Idempotent — a language is
matched by its code and existing links are kept — so it is safe to re-run. Countries are
matched by name, so run `country_loader` first. `start.sh` runs it on boot when `languages`
is empty. `GET /api/common/languages` is cached for an hour (`common:languages:*`): after a
manual load on a running stack, delete those keys or wait.

```bash
cargo run --bin language_loader -- ../data/merged_countries.json
```

## orphan_images

Lists the objects under the `images/` prefix of the bucket that no row of the
`images` table points to, and with `--delete` removes them. Such objects appear
when an organisation delete or an upload rolled back and the storage cleanup
failed, and from organisation deletes made before that cleanup existed. The
bucket is publicly readable, so an orphan stays reachable by its old URL.

```bash
cargo run --bin orphan_images                          # dry run: only report
cargo run --bin orphan_images -- --min-age-hours 1     # judge objects older than 1 h
cargo run --bin orphan_images -- --delete              # remove the orphans
```

| Flag | Meaning |
|---|---|
| *(none)* | Dry run — report only. |
| `--delete` | Delete the orphans. |
| `--min-age-hours N` | Ignore objects younger than `N` hours (default **24**). An upload writes the object *before* its row, so a young rowless object may be an upload in flight. |
| `--allow-empty-db` | Allow `--delete` when the database knows no images at all. Off by default: an empty table usually means the wrong `DATABASE_URL`, and every object would look orphaned. |

Needs `DATABASE_URL` and `S3_ENDPOINT_URL`, `S3_ACCESS_KEY_ID`,
`S3_SECRET_ACCESS_KEY`, `S3_BUCKET` (`S3_REGION` optional).

Example output:

```text
bucket landly-images: 120 objects under images/, 117 in use, 1 too new to judge, 2 orphaned
  orphan: images/<org-id>/<uuid>.png
  orphan: images/<org-id>/<uuid>.jpg
dry run — rerun with --delete to remove them
```

Exit codes: `0` ok, `1` S3 not configured / database or bucket unreadable / some
deletes failed, `2` bad arguments.

**Safety.** It only looks at `images/`, never deletes without `--delete`, and
the comparison is against the database named by `DATABASE_URL` — so point it at
the database that owns the bucket. Read one dry-run report before the first
`--delete`.

**On the compose stack** use the `orphan-images` service instead (profile
`maintenance`, same image and environment as the server):

```bash
docker compose run --rm orphan-images             # dry run
docker compose run --rm orphan-images --delete    # delete
```

A weekly cron entry on the host is enough, e.g.
`0 4 * * 0 cd /srv/landly && docker compose run --rm orphan-images --delete`.

The logic (listing, comparison, age and empty-database guards) lives in
[`src/utils/orphan_cleanup.rs`](../../src/utils/orphan_cleanup.rs) and is
unit-tested there; the binary is only argument parsing around `sweep`.

---

## Adding a tool

1. `bin/<name>/Cargo.toml` with `edition.workspace = true` and `publish.workspace = true`;
   add `"bin/<name>"` to `members` in the workspace `Cargo.toml`.
2. Keep logic that is worth testing in the main crate (`src/utils/…`) and make
   the binary thin, like `orphan_images`.
3. Ship it in the image: add a `COPY --from=builder …/release/<name>` and a
   `chmod +x` line to the root `Dockerfile`; for a recurring job add a service
   with `profiles: ["maintenance"]` to `docker-compose.yml`.
4. Document it here and in [`docs/operations.md`](../../docs/operations.md).
