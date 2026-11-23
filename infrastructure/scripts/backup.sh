#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
umask 077
mkdir -p backups
tmp=$(mktemp backups/manager.XXXXXXXX)
trap 'unlink "$tmp"' EXIT
"${ENGINE:-docker}" compose exec -T db pg_dump -U postgres -d manager -Fc > "$tmp"
"${ENGINE:-docker}" compose exec -T db pg_restore -l < "$tmp" >/dev/null
ln "$tmp" "$tmp.dump"
echo "Arquivo criado: $tmp.dump"
