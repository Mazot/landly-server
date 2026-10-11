#!/usr/bin/env bash
# End-to-end smoke test against a RUNNING landly-server + Postgres.
#
#   scripts/smoke.sh                 # core scenario
#   SMOKE_IMAGES=1 scripts/smoke.sh  # + image upload/delete (needs MinIO/S3)
#
# Needs: curl, jq, docker (role changes + cleanup go through `docker exec psql`).
# Creates a few users/orgs with a unique run prefix and deletes them at the end.
# Uses 6 credential requests, so it stays under the auth rate limit (burst 10);
# don't run it more than once per minute against the same server.
#
# Env: BASE_URL (http://localhost:8080), PG_CONTAINER (landly_postgres),
#      PG_USER (landly-user), PG_DB (landly), SMOKE_IMAGES (0).
set -uo pipefail

BASE_URL="${BASE_URL:-http://localhost:8080}"
PG_CONTAINER="${PG_CONTAINER:-landly_postgres}"
PG_USER="${PG_USER:-landly-user}"
PG_DB="${PG_DB:-landly}"
SMOKE_IMAGES="${SMOKE_IMAGES:-0}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

RUN="smoke$(date +%s)$RANDOM"
PASSWORD="Smoke-pass-123"
TMP="$(mktemp -d)"
trap 'cleanup; rm -rf "$TMP"' EXIT

PASS=0
FAIL=0
CODE=""
BODY=""

for bin in curl jq docker; do
    command -v "$bin" >/dev/null || { echo "missing dependency: $bin" >&2; exit 2; }
done

# ------------------------------------------------------------------ helpers

# req METHOD PATH [TOKEN] [JSON]  -> sets CODE and BODY
req() {
    local method="$1" path="$2" token="${3:-}" json="${4:-}"
    local args=(-s -o "$TMP/body" -w '%{http_code}' -X "$method" "$BASE_URL$path")
    [ -n "$token" ] && args+=(-H "Authorization: Bearer $token")
    [ -n "$json" ] && args+=(-H 'Content-Type: application/json' -d "$json")
    CODE="$(curl "${args[@]}")"
    BODY="$(cat "$TMP/body")"
}

# check NAME EXPECTED_CODE  (against the last req)
check() {
    if [ "$CODE" = "$2" ]; then
        PASS=$((PASS + 1))
        printf '  PASS  %-62s %s\n' "$1" "$CODE"
    else
        FAIL=$((FAIL + 1))
        printf '  FAIL  %-62s got %s, want %s\n' "$1" "$CODE" "$2"
        printf '        body: %.300s\n' "$BODY"
    fi
}

# assert NAME CONDITION_RESULT(0/1) [DETAIL]
assert() {
    if [ "$2" = "0" ]; then
        PASS=$((PASS + 1))
        printf '  PASS  %s\n' "$1"
    else
        FAIL=$((FAIL + 1))
        printf '  FAIL  %s %s\n' "$1" "${3:-}"
    fi
}

psql_exec() {
    docker exec -i "$PG_CONTAINER" psql -U "$PG_USER" -d "$PG_DB" -v ON_ERROR_STOP=1 -tAq -c "$1"
}

cleanup() {
    [ -n "${CLEANED:-}" ] && return
    CLEANED=1
    echo
    echo "== cleanup ($RUN)"
    psql_exec "
        DELETE FROM moderation_events WHERE target_id IN (
            SELECT o.id FROM organisations o JOIN users u ON u.id = o.created_by
            WHERE u.email LIKE '$RUN-%');
        DELETE FROM organisations WHERE created_by IN (SELECT id FROM users WHERE email LIKE '$RUN-%');
        DELETE FROM users WHERE email LIKE '$RUN-%';" \
        && echo "  removed test users/orgs" \
        || echo "  WARN: cleanup failed — remove rows with email LIKE '$RUN-%' by hand"
}

section() { echo; echo "== $1"; }

# -------------------------------------------------------------- scenario

section "docs & health"
req GET /api/healthcheck; check "GET /api/healthcheck" 200
req GET /scalar; check "GET /scalar" 200
req GET /api-docs/openapi.json; check "GET /api-docs/openapi.json" 200
paths=$(jq '.paths | length' <<<"$BODY" 2>/dev/null || echo 0)
assert "openapi spec has paths ($paths)" "$([ "$paths" -gt 40 ] && echo 0 || echo 1)"

section "401 without token on every AUTH_REQUIRED_ROUTES entry"
uuid="00000000-0000-4000-8000-000000000000"
# path/method pairs straight from the source of truth
awk '/const AUTH_REQUIRED_ROUTES/,/^\];/' "$ROOT/src/app/drivers/middlewares/auth.rs" \
    | grep -oE 'path: "[^"]+"|Method::[A-Z]+' | paste - - \
    | sed -E 's/path: "([^"]+)"\tMethod::([A-Z]+)/\2 \1/' > "$TMP/auth_routes"
while read -r method pattern; do
    path="${pattern//\{id\}/$uuid}"
    case "$path" in */) path="$path$uuid" ;; esac
    req "$method" "$path"
    check "$method $pattern (no token)" 401
done < "$TMP/auth_routes"

section "signup / signin / credentials"
signup() { # signup NAME -> echoes token, sets USER_ID_<name>
    req POST /api/user/signup "" "{\"username\":\"$RUN$1\",\"email\":\"$RUN-$1@example.test\",\"password\":\"$PASSWORD\"}"
}
signup a; check "signup user A" 200; TOKEN_A=$(jq -r '.token // empty' <<<"$BODY")
signup b; check "signup user B" 200; TOKEN_B=$(jq -r '.token // empty' <<<"$BODY")
signup m; check "signup user M (future moderator)" 200; TOKEN_M=$(jq -r '.token // empty' <<<"$BODY")
assert "signup returns tokens" "$([ -n "$TOKEN_A" ] && [ -n "$TOKEN_B" ] && [ -n "$TOKEN_M" ] && echo 0 || echo 1)"

req POST /api/user/signup "" "{\"username\":\"${RUN}dup\",\"email\":\"$(echo "$RUN-A" | tr a-z A-Z)@EXAMPLE.test\",\"password\":\"$PASSWORD\"}"
check "signup with case-only duplicate email -> 422" 422

req POST /api/user/signin "" "{\"email\":\"$RUN-a@example.test\",\"password\":\"wrong-password-1\"}"
check "signin wrong password -> 401" 401; WRONG_PW_BODY="$BODY"
req POST /api/user/signin "" "{\"email\":\"$RUN-nobody@example.test\",\"password\":\"wrong-password-1\"}"
check "signin unknown email -> 401" 401
assert "wrong password and unknown email answer identically" "$([ "$WRONG_PW_BODY" = "$BODY" ] && echo 0 || echo 1)"

req GET /api/user/me "$TOKEN_A"; check "GET /api/user/me with token" 200

section "moderation flow & RBAC"
psql_exec "UPDATE users SET role = 'moderator' WHERE email = '$RUN-m@example.test';" >/dev/null
lat=52.52$((RANDOM % 90 + 10)); lng=13.40$((RANDOM % 90 + 10))
req POST /api/organisation/create "$TOKEN_A" "{\"name\":\"$RUN org\",\"latitude\":$lat,\"longitude\":$lng}"
check "A creates organisation" 200
ORG_ID=$(jq -r '.id // empty' <<<"$BODY")
assert "new organisation is pending" "$([ "$(jq -r .status <<<"$BODY")" = pending ] && echo 0 || echo 1)" "(status=$(jq -r .status <<<"$BODY"))"

search_has_org() {
    req GET "/api/organisation/search?lat=$lat&lng=$lng&radius_km=2"
    jq -e --arg id "$ORG_ID" '.items | any(.id == $id)' <<<"$BODY" >/dev/null && echo 0 || echo 1
}
assert "pending org is hidden from search" "$([ "$(search_has_org)" = 1 ] && echo 0 || echo 1)"

decision="{\"kind\":\"org\",\"target_id\":\"$ORG_ID\"}"
req POST /api/moderation/approve "$TOKEN_B" "$decision"; check "plain user B approves -> 403" 403
req POST /api/moderation/approve "$TOKEN_M" "$decision"; check "moderator M approves" 200
assert "approved org is visible in search" "$(search_has_org)"

req PUT "/api/organisation/update/$ORG_ID" "$TOKEN_B" '{"description":"hijack"}'
check "B edits A's organisation -> 403" 403
req PUT "/api/organisation/update/$ORG_ID" "$TOKEN_M" '{"description":"moderator edit"}'
check "moderator edits it" 200
assert "moderator edit keeps it live" "$([ "$(jq -r .status <<<"$BODY")" = live ] && echo 0 || echo 1)" "(status=$(jq -r .status <<<"$BODY"))"
req PUT "/api/organisation/update/$ORG_ID" "$TOKEN_A" '{"description":"creator edit"}'
check "creator A edits live org" 200
assert "creator edit sends it back to pending" "$([ "$(jq -r .status <<<"$BODY")" = pending ] && echo 0 || echo 1)" "(status=$(jq -r .status <<<"$BODY"))"

req POST /api/common/org_types "$TOKEN_M" '{"org_type":"smoke","color":"#000000","title":"Smoke"}'
check "moderator creates org type (admin only) -> 403" 403

if [ "$SMOKE_IMAGES" = "1" ]; then
    section "images (SMOKE_IMAGES=1)"
    printf '\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\1\0\0\0\1\x08\x06\0\0\0\x1f\x15\xc4\x89\0\0\0\rIDATx\x9cc\xf8\x0f\0\0\1\1\0\x05\x18\xd8N\0\0\0\0IEND\xaeB`\x82' > "$TMP/pixel.png"
    upload() {
        CODE=$(curl -s -o "$TMP/body" -w '%{http_code}' -X POST "$BASE_URL/api/images/upload/$ORG_ID" \
            -H "Authorization: Bearer $1" -F "file=@$TMP/pixel.png;type=image/png")
        BODY="$(cat "$TMP/body")"
    }
    upload "$TOKEN_B"; check "B uploads to A's organisation -> 403" 403
    upload "$TOKEN_A"; check "A uploads an image" 200
    IMG_ID=$(jq -r '.id // empty' <<<"$BODY"); IMG_URL=$(jq -r '.url // empty' <<<"$BODY")
    if [ -n "$IMG_URL" ]; then
        CODE=$(curl -s -o /dev/null -w '%{http_code}' "$IMG_URL"); BODY=""
        check "public image URL is readable" 200
    fi
    req DELETE "/api/images/delete/$IMG_ID" "$TOKEN_A"; check "A deletes the image" 200
fi

section "observed, not asserted"
req GET "/api/organisation/fetch/$ORG_ID"
echo "  INFO  public GET /api/organisation/fetch/{id} of a pending org -> $CODE (status=$(jq -r '.status // "-"' <<<"$BODY"))"

section "summary"
echo "  $PASS passed, $FAIL failed (run $RUN)"
[ "$FAIL" -eq 0 ]
