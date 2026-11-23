#!/bin/sh
set -eu
# O arquivo local contém senha hexadecimal gerada por iniciar.sh.
app_pass=$(cat /run/secrets/db_app)
case "$app_pass" in ''|*[!a-fA-F0-9]*) exit 1;; esac
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<SQL
CREATE ROLE manager LOGIN PASSWORD '$app_pass';
ALTER DATABASE manager OWNER TO manager;
ALTER SCHEMA public OWNER TO manager;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
SQL
