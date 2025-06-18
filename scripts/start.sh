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

echo "Starting application..."
exec ./landly-server
