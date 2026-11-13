#!/bin/bash
set -euo pipefail

DIR=${1:?usage: apply-migrations.sh <migrations dir>}
DB=${DB_NAME:-zutility}
PSQL=(sudo -u postgres psql -d "$DB" -v ON_ERROR_STOP=1 -tA)

"${PSQL[@]}" -c "CREATE TABLE IF NOT EXISTS schema_migrations (
    filename TEXT PRIMARY KEY,
    sha256 TEXT NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
)"

applied=$("${PSQL[@]}" -c "SELECT count(*) FROM schema_migrations")
has_orders=$("${PSQL[@]}" -c "SELECT to_regclass('public.orders') IS NOT NULL")

if [ "$applied" = "0" ] && [ "$has_orders" = "t" ]; then
  for file in "$DIR"/*.sql; do
    name=$(basename "$file")
    sha=$(sha256sum "$file" | cut -d' ' -f1)
    "${PSQL[@]}" -c "INSERT INTO schema_migrations (filename, sha256) VALUES ('$name', '$sha') ON CONFLICT DO NOTHING"
  done
  echo "bootstrapped the migration ledger with the existing schema"
  exit 0
fi

for file in "$DIR"/*.sql; do
  name=$(basename "$file")
  sha=$(sha256sum "$file" | cut -d' ' -f1)
  recorded=$("${PSQL[@]}" -c "SELECT sha256 FROM schema_migrations WHERE filename = '$name'")

  if [ -n "$recorded" ]; then
    if [ "$recorded" != "$sha" ]; then
      echo "migration $name changed after it was applied" >&2
      exit 1
    fi
    continue
  fi

  echo "applying $name"
  sudo -u postgres psql -d "$DB" -v ON_ERROR_STOP=1 -1 -f "$file"
  "${PSQL[@]}" -c "INSERT INTO schema_migrations (filename, sha256) VALUES ('$name', '$sha')"
done

echo "migrations up to date"
