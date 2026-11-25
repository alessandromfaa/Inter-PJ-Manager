#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
: "${COMPOSE_PROJECT_NAME:?Defina explicitamente COMPOSE_PROJECT_NAME para o ambiente autorizado}"
: "${COMPOSE_FILE:?Defina explicitamente COMPOSE_FILE para o ambiente autorizado}"
case "$COMPOSE_PROJECT_NAME" in
  *[!a-z0-9_-]*|'') echo "Nome de projeto Compose inválido." >&2; exit 1 ;;
esac
umask 077
mkdir -p backups
exec flock -n "backups/.backup-${COMPOSE_PROJECT_NAME}.lock" ./infrastructure/scripts/backup.sh
