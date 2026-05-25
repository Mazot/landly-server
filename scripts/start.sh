#!/bin/bash
set -e

echo "Waiting for database to be ready..."
while ! pg_isready -h db -p 5432 -U landly-user; do
    sleep 2
done

export HOST=${HOST:-"0.0.0.0"}
export PORT=${PORT:-"8080"}

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
