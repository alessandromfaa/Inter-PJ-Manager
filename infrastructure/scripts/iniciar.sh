#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
umask 077
mkdir -p secrets
chmod 700 secrets
for name in db_root db_app admin mfa_key; do
 if [ ! -e "secrets/$name" ]; then
  tmp=$(mktemp "secrets/$name.XXXXXX")
  openssl rand -hex 32 > "$tmp"
  chmod 444 "$tmp"
  ln "$tmp" "secrets/$name" || test -s "secrets/$name"
  unlink "$tmp"
 fi
done
exec "${ENGINE:-docker}" compose up -d --build
