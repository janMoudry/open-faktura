#!/bin/sh
set -eu
# Compose file secrets retain host permissions. Read as root only at startup,
# then drop privileges before starting the server. Never print secret values.
if [ -n "${DATABASE_URL_FILE:-}" ]; then
  DATABASE_URL=$(cat "$DATABASE_URL_FILE")
  export DATABASE_URL
  unset DATABASE_URL_FILE
fi
if [ -n "${BOOTSTRAP_SECRET_FILE:-}" ]; then
  install -d -m 0700 -o 10001 -g 10001 /run/faktura
  install -m 0400 -o 10001 -g 10001 "$BOOTSTRAP_SECRET_FILE" /run/faktura/bootstrap
  export BOOTSTRAP_SECRET_FILE=/run/faktura/bootstrap
fi
exec setpriv --reuid=10001 --regid=10001 --init-groups --no-new-privs faktura-server "$@"
