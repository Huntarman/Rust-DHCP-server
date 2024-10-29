#!/bin/sh
set -e

until pg_isready -h db -p 5432 -U "$POSTGRES_USER"; do
  echo "Waiting for db to be ready..."
  sleep 1
done

echo "db is up - starting application"
exec "$@"