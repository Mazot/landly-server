#!/usr/bin/env bash
# Creates/updates the landly_readonly role in the LOCAL docker Postgres and
# stores its connection string as MCP_DATABASE_URL in .env (generated once,
# never committed). Safe to re-run, e.g. after new migrations.
#
# Env: PG_CONTAINER (landly_postgres), PG_USER (landly-user), PG_DB (landly),
#      PG_HOST_PORT (5432) — the port the host reaches Postgres on.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ENV_FILE="$ROOT/.env"
PG_CONTAINER="${PG_CONTAINER:-landly_postgres}"
PG_USER="${PG_USER:-landly-user}"
PG_DB="${PG_DB:-landly}"
PG_HOST_PORT="${PG_HOST_PORT:-5432}"

docker inspect -f '{{.State.Running}}' "$PG_CONTAINER" 2>/dev/null | grep -q true \
    || { echo "container $PG_CONTAINER is not running (docker start $PG_CONTAINER)" >&2; exit 1; }

touch "$ENV_FILE"
existing="$(grep -E '^MCP_DATABASE_URL=' "$ENV_FILE" | tail -n 1 | cut -d= -f2- || true)"
if [ -n "$existing" ]; then
    # reuse the password already stored in .env
    password="$(sed -E 's#^[^:]+://[^:]+:([^@]+)@.*#\1#' <<<"$existing")"
else
    password="$(openssl rand -hex 24)"
    printf '\n# Read-only role for the Postgres MCP server (scripts/dev/setup-readonly-db.sh)\nMCP_DATABASE_URL=postgresql://landly_readonly:%s@localhost:%s/%s\n' \
        "$password" "$PG_HOST_PORT" "$PG_DB" >> "$ENV_FILE"
    echo "added MCP_DATABASE_URL to .env"
fi

docker exec -i "$PG_CONTAINER" psql -q -U "$PG_USER" -d "$PG_DB" -v ro_password="$password" \
    < "$ROOT/scripts/dev/create-readonly-role.sql"
echo "landly_readonly is ready (SELECT only, no password hashes / claim tokens)"
