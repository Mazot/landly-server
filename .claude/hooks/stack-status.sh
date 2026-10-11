#!/bin/bash
# SessionStart: report the local stack state so the agent knows it up front.
# Read-only — never starts, stops or migrates anything. Always exits 0.
# Output: JSON with the full summary as additionalContext (for the agent) and
# a one-line systemMessage (for the user).
root="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$root" || exit 0

lines=()
problems=()
add() { lines+=("$1"); }

# ---------------------------------------------------------------- containers
# Health is only meaningful while running (a stopped container keeps its last one).
container_state() { docker inspect -f '{{.State.Status}}{{if and .State.Health (eq .State.Status "running")}}/{{.State.Health.Status}}{{end}}' "$1" 2>/dev/null || echo missing; }

if ! docker info >/dev/null 2>&1; then
    add "- Docker: NOT running — Postgres/Redis/MinIO unavailable."
    problems+=("docker down")
    pg_up=false
else
    pg_up=false
    for c in landly_postgres landly_redis_cache landly_minio; do
        state="$(container_state "$c")"
        add "- $c: $state"
        case "$c:$state" in
            landly_postgres:running*) pg_up=true ;;
            landly_postgres:*) problems+=("postgres $state") ;;
            landly_redis_cache:running*) ;;
            landly_redis_cache:*) problems+=("redis $state") ;;
        esac
    done
    server_state="$(container_state landly_server)"
    if [[ "$server_state" == running* ]]; then
        add "- landly_server (docker): running — it holds port 8080, stop it before \`cargo run\`."
    fi
fi

# ---------------------------------------------------------------- migrations
if $pg_up && command -v diesel >/dev/null 2>&1; then
    if list="$(diesel migration list 2>/dev/null)"; then
        pending="$(grep -F '[ ]' <<<"$list" | sed -E 's/^[[:space:]]*\[ \][[:space:]]*//')"
        if [ -n "$pending" ]; then
            count="$(wc -l <<<"$pending" | tr -d ' ')"
            add "- Pending migrations ($count): $(paste -sd, - <<<"$pending" | sed 's/,/, /g') — apply with \`diesel migration run\` (local DB) before relying on the schema."
            problems+=("$count pending migration(s)")
        else
            add "- Migrations: all applied."
        fi
    else
        add "- Migrations: could not query (check DATABASE_URL in .env)."
    fi
else
    add "- Migrations: not checked (Postgres not running)."
fi

# ---------------------------------------------------------------- server
if lsof -nP -iTCP:8080 -sTCP:LISTEN >/dev/null 2>&1; then
    add "- Port 8080: in use (a landly server is probably running)."
else
    add "- Port 8080: free (server not running; \`cargo run\` to start)."
fi

# ---------------------------------------------------------------- MCP servers
env_has() { grep -qE "^$1=" "$root/.env" 2>/dev/null; }
mcp_note=""
env_has MCP_DATABASE_URL || mcp_note+=" landly-postgres needs scripts/dev/setup-readonly-db.sh;"
env_has MCP_REDIS_URL || mcp_note+=" landly-redis needs scripts/dev/setup-readonly-redis.sh;"
add "- MCP (landly-postgres / landly-redis) connected at session start: their tools only work if the containers above were running then; after starting them, the user must restart the session.${mcp_note}"

# ---------------------------------------------------------------- output
if [ ${#problems[@]} -gt 0 ]; then
    add "To start the stack: \`docker start landly_postgres landly_redis_cache landly_minio\` — ask the user before starting or migrating anything."
    summary="landly stack: $(printf '%s, ' "${problems[@]}" | sed 's/, $//')"
else
    summary="landly stack: postgres + redis up, migrations applied"
fi

context="Local landly stack at session start (read-only check by .claude/hooks/stack-status.sh):"
for l in "${lines[@]}"; do context+=$'\n'"$l"; done

jq -n --arg ctx "$context" --arg msg "$summary" '{
    systemMessage: $msg,
    hookSpecificOutput: { hookEventName: "SessionStart", additionalContext: $ctx }
}'
exit 0
