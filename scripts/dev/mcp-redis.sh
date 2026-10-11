#!/usr/bin/env bash
# Launcher for the project's Redis MCP server (.mcp.json → "landly-redis").
# Re-creates the read-only ACL user first (Redis forgets it on restart), then
# starts the official redis-mcp-server as that user. The server has no
# read-only mode of its own — the ACL is what blocks its set/delete/expire
# tools. stdout is reserved for the MCP protocol; diagnostics go to stderr.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

"$ROOT/scripts/dev/setup-readonly-redis.sh" \
    || echo "warning: could not refresh the read-only Redis user; tools will fail until Redis is up and the session restarts" >&2

url="$(grep -E '^MCP_REDIS_URL=' "$ROOT/.env" 2>/dev/null | tail -n 1 | cut -d= -f2- || true)"
if [ -z "$url" ]; then
    echo "MCP_REDIS_URL is not set in .env — start Redis and run scripts/dev/setup-readonly-redis.sh" >&2
    exit 1
fi
case "$url" in
    *@localhost:* | *@127.0.0.1:* | *@\[::1\]:*) ;;
    *) echo "refusing to start: MCP_REDIS_URL must point at a local Redis" >&2; exit 1 ;;
esac

# --url wins over the REDIS_* variables the server would read via dotenv, so
# the admin REDIS_URL in .env is never used. Run from a neutral directory anyway.
cd /
exec uvx --python 3.12 --from 'redis-mcp-server==0.5.1' redis-mcp-server --url "$url"
