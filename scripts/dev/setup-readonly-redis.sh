#!/usr/bin/env bash
# Creates/refreshes the `landly_readonly` ACL user in the LOCAL docker Redis and
# stores its URL as MCP_REDIS_URL in .env (password generated once, never
# committed). The compose stack regenerates its ACL file on every container
# start, so this user disappears on restart — scripts/dev/mcp-redis.sh calls
# this script before every launch. All output goes to stderr (the MCP launcher
# must keep stdout clean for the protocol).
#
# The user may only READ the cache namespaces (CacheKeys in src/utils/cache.rs);
# `oauth:*` (PKCE verifiers, single-use login codes = sessions) is off limits.
# Key NAMES stay visible through SCAN — a 60 s login code would show up by name
# while a local OAuth login is in flight.
#
# Env: REDIS_CONTAINER (landly_redis_cache), REDIS_HOST_PORT (6379).
set -euo pipefail
exec 1>&2

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ENV_FILE="$ROOT/.env"
REDIS_CONTAINER="${REDIS_CONTAINER:-landly_redis_cache}"
REDIS_HOST_PORT="${REDIS_HOST_PORT:-6379}"
# Keep in sync with the CacheKeys namespaces (test_feature_patterns_are_distinct).
CACHE_NAMESPACES=('~org:*' '~cc:*' '~img:*' '~cor:*' '~per:*' '~common:*')

docker inspect -f '{{.State.Running}}' "$REDIS_CONTAINER" 2>/dev/null | grep -q true \
    || { echo "container $REDIS_CONTAINER is not running (docker start $REDIS_CONTAINER)"; exit 1; }

env_value() { grep -E "^$1=" "$ENV_FILE" 2>/dev/null | tail -n 1 | cut -d= -f2- || true; }

admin_url="$(env_value REDIS_URL)"
[ -n "$admin_url" ] || { echo "REDIS_URL (admin) is not set in .env"; exit 1; }
admin_user="$(sed -E 's#^rediss?://([^:@/]*):.*#\1#' <<<"$admin_url")"
admin_pass="$(sed -E 's#^rediss?://[^:@/]*:([^@]*)@.*#\1#' <<<"$admin_url")"

ro_url="$(env_value MCP_REDIS_URL)"
if [ -n "$ro_url" ]; then
    ro_pass="$(sed -E 's#^rediss?://[^:@/]*:([^@]*)@.*#\1#' <<<"$ro_url")"
else
    ro_pass="$(openssl rand -hex 24)"
    printf '\n# Read-only Redis user for the Redis MCP server (scripts/dev/setup-readonly-redis.sh)\nMCP_REDIS_URL=redis://landly_readonly:%s@localhost:%s/0\n' \
        "$ro_pass" "$REDIS_HOST_PORT" >> "$ENV_FILE"
    echo "added MCP_REDIS_URL to .env"
fi

docker exec -e REDISCLI_AUTH="$admin_pass" "$REDIS_CONTAINER" \
    redis-cli --no-auth-warning --user "$admin_user" \
    ACL SETUSER landly_readonly reset on ">$ro_pass" "${CACHE_NAMESPACES[@]}" resetchannels \
    -@all +@read -keys +info +ping +hello +auth +select \
    '+client|setname' '+client|getname' '+client|setinfo' '+client|id' \
    | grep -qx OK || { echo "ACL SETUSER failed"; exit 1; }

echo "redis user landly_readonly is ready (read-only, cache namespaces only)"
