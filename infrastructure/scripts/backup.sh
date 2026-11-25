#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
umask 077
mkdir -p backups
tmp=$(mktemp backups/manager.XXXXXXXX)
manifest_tmp="$tmp.manifest"
: > "$manifest_tmp"
trap 'unlink "$tmp"; test -z "$manifest_tmp" || unlink "$manifest_tmp"' EXIT
"${ENGINE:-docker}" compose exec -T db pg_dump -U postgres -d manager -Fc > "$tmp"
test -s "$tmp"
"${ENGINE:-docker}" compose exec -T db pg_restore -l < "$tmp" >/dev/null
"${ENGINE:-docker}" compose exec -T db pg_restore -l < "$tmp" | grep -Eq 'TABLE DATA public companies '
dump="$tmp.dump"
manifest="$dump.manifest"
{
  printf 'criado_em_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'tamanho_bytes=%s\n' "$(wc -c < "$tmp" | tr -d ' ')"
  printf 'sha256=%s\n' "$(sha256sum "$tmp" | awk '{print $1}')"
  printf 'catalogo_validado=true\n'
  printf 'restauracao_ensaiada=false\n'
} > "$manifest_tmp"
ln "$tmp" "$dump"
mv "$manifest_tmp" "$manifest"
manifest_tmp=""
echo "Arquivo criado: $dump"
echo "Manifesto criado: $manifest"
