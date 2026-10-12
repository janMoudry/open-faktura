#!/bin/sh
# Only for a fresh production Compose project; refuses a database with user tables.
set -eu
compose() { docker compose --env-file "${PRODUCTION_ENV_FILE:-infra/production.env}" -f infra/compose.production.yaml "$@"; }
backup_dir=${1:?Usage: sh scripts/restore.sh BACKUP_DIRECTORY}
test -s "$backup_dir/database.dump"
test -f "$backup_dir/files.tar"
compose up -d --wait postgres
count=$(compose exec -T postgres psql -U faktura -d faktura -Atc "SELECT count(*) FROM pg_tables WHERE schemaname='public'")
[ "$count" = 0 ] || { echo 'Refusing restore: target database is not empty.' >&2; exit 1; }
compose stop app
compose exec -T postgres pg_restore -U faktura -d faktura --exit-on-error --single-transaction < "$backup_dir/database.dump"
# A backup must never resurrect browser sessions.
compose exec -T postgres psql -U faktura -d faktura -v ON_ERROR_STOP=1 -c 'DELETE FROM auth_session;'
compose run --rm -T --no-deps --entrypoint tar app -C /data/files -xf - < "$backup_dir/files.tar"
printf 'Restore complete. Start the matching app version with compose up -d.\n'
