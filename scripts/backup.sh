#!/bin/sh
# Run from repository root. Stop writers for a consistent DB + files snapshot.
set -eu
umask 077
compose() { docker compose --env-file "${PRODUCTION_ENV_FILE:-infra/production.env}" -f infra/compose.production.yaml "$@"; }
backup_dir=${1:?Usage: sh scripts/backup.sh NEW_BACKUP_DIRECTORY}
mkdir "$backup_dir"
compose stop app
trap 'compose start app >/dev/null' EXIT HUP INT TERM
compose exec -T postgres pg_dump -U faktura -d faktura --format=custom > "$backup_dir/database.dump"
compose run --rm --no-deps --entrypoint tar app -C /data/files -cf - . > "$backup_dir/files.tar"
cp "${PRODUCTION_ENV_FILE:-infra/production.env}" "$backup_dir/production.env"
# Keep operator secrets separately from source control, together with encrypted backups.
cp -R .secrets "$backup_dir/secrets"
printf 'Backup complete. Protect and encrypt this directory before off-site storage.\n'
