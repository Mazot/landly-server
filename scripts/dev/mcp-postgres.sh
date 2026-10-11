#!/usr/bin/env bash
# Launcher for the project's Postgres MCP server (.mcp.json → "landly-postgres").
# Connects as the read-only role from MCP_DATABASE_URL in .env, so no
# credentials live in .mcp.json. Pinned version + restricted mode
# (read-only transactions, SQL checked before execution). Python 3.12: its
# pglast dependency has no wheels for newer interpreters. `mcp<2`: 0.3.0 does
# not cap it, and mcp 2.x renamed FastMCP.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
url="$(grep -E '^MCP_DATABASE_URL=' "$ROOT/.env" 2>/dev/null | tail -n 1 | cut -d= -f2- || true)"

if [ -z "$url" ]; then
    echo "MCP_DATABASE_URL is not set in .env — run scripts/dev/setup-readonly-db.sh first" >&2
    exit 1
fi
case "$url" in
    *@localhost:* | *@127.0.0.1:* | *@\[::1\]:*) ;;
    *) echo "refusing to start: MCP_DATABASE_URL must point at a local database" >&2; exit 1 ;;
esac

export DATABASE_URI="$url"
exec uvx --python 3.12 --from 'postgres-mcp==0.3.0' --with 'mcp<2' postgres-mcp --access-mode=restricted
