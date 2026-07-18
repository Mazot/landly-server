#!/bin/bash
# Container entrypoint: waits for Postgres, applies migrations, seeds
# countries on first run, then execs the server binary.
set -euo pipefail

DB_HOST="${DB_HOST:-db}"
DB_PORT="${DB_PORT:-5432}"
DB_USER="${DB_USER:-landly-user}"
DB_WAIT_TIMEOUT="${DB_WAIT_TIMEOUT:-60}"

echo "Waiting for database at ${DB_HOST}:${DB_PORT} (timeout ${DB_WAIT_TIMEOUT}s)..."
elapsed=0
while ! pg_isready -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" > /dev/null 2>&1; do
    sleep 2
    elapsed=$((elapsed + 2))
    if [ "$elapsed" -ge "$DB_WAIT_TIMEOUT" ]; then
        echo "ERROR: database did not become ready within ${DB_WAIT_TIMEOUT}s" >&2
        exit 1
    fi
done

export HOST="${HOST:-0.0.0.0}"
export PORT="${PORT:-8080}"

echo "Running database migrations..."
diesel migration run

echo "Checking if countries need to be loaded..."
COUNTRY_COUNT=$(psql "$DATABASE_URL" -t -A -c "SELECT count(*) FROM countries;" 2>/dev/null || echo "0")
if [ "$COUNTRY_COUNT" -eq "0" ]; then
  echo "Loading countries..."
  ./country_loader ./country_data/merged_countries.json
  echo "Country loader finished."
else
  echo "Countries already loaded ($COUNTRY_COUNT records), skipping."
fi

echo "Starting application..."
exec ./landly-server
