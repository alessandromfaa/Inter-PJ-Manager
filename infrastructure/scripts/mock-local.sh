#!/usr/bin/env bash
# Desenvolvimento nativo, com todos os dados e caches dentro do checkout.
set -euo pipefail
cd "$(dirname "$0")/../.."
base=$PWD
state="$base/.tools/mock-local"
umask 077
mkdir -p "$state/socket" "$state/tmp"
exec 9>"$state/execucao.lock"
flock -n 9 || { echo "O ambiente MOCK local já está em execução." >&2; exit 1; }
export TMPDIR="$state/tmp" npm_config_cache="$base/.tools/npm-cache"
export PATH="$base/.tools/node/bin:$base/.tools/cargo/bin:$PATH"
if [[ -d "$base/.tools/rustup" ]]; then
  export RUSTUP_HOME="$base/.tools/rustup" CARGO_HOME="$base/.tools/cargo"
fi
pgbin="$base/.tools/pg/node_modules/@embedded-postgres/linux-x64/native/bin"
if [[ ! -x "$pgbin/initdb" ]]; then pgbin=$(pg_config --bindir); fi
# Falhe antes de iniciar o cliente se outra aplicação estiver usando uma porta.
node - <<'JS'
const net = require('node:net');
for (const port of [8080,5173,55487]) {
 const socket=net.createServer().once('error',()=>{console.error('Porta local ocupada: '+port);process.exitCode=1});
 socket.listen(port,'127.0.0.1',()=>socket.close());
}
JS
export APP_HOST=127.0.0.1 APP_PORT=8080 APP_ORIGIN=http://127.0.0.1:5173
export APP_ENV=development BANK_PROVIDER=inter INTER_ENV=mock ENABLE_BANK_WRITES=false ENABLE_REAL_PAYMENTS=false
export DATABASE_URL=postgresql://manager@127.0.0.1:55487/postgres
export ADMIN_EMAIL=admin@local.test ADMIN_PASSWORD_FILE="$state/admin" DB_PASSWORD_FILE="$state/db"
export MFA_ENCRYPTION_KEY_FILE="$state/mfa_key"
for secret in "$ADMIN_PASSWORD_FILE" "$DB_PASSWORD_FILE" "$MFA_ENCRYPTION_KEY_FILE"; do
  if [[ ! -e "$secret" ]]; then openssl rand -hex 32 > "$secret"; fi
done
if [[ ! -e "$state/pg/PG_VERSION" ]]; then
  "$pgbin/initdb" -D "$state/pg" -U manager --pwfile="$DB_PASSWORD_FILE" --auth=scram-sha-256 --encoding=UTF8 --no-locale > "$state/init.log"
fi
(cd backend && cargo build --locked)
(cd frontend && npm ci --legacy-peer-deps && npm run build)
"$pgbin/pg_ctl" -D "$state/pg" -l "$state/postgres.log" -o "-h 127.0.0.1 -p 55487 -k '$state/socket'" start
api_pid=''
web_pid=''
cleanup(){
  if [[ -n "$web_pid" ]]; then kill "$web_pid" 2>/dev/null || true; fi
  if [[ -n "$api_pid" ]]; then kill "$api_pid" 2>/dev/null || true; fi
  "$pgbin/pg_ctl" -D "$state/pg" -m fast stop
}
trap cleanup EXIT
trap 'exit 0' INT TERM
(cd backend && target/debug/inter-pj-manager bootstrap)
(cd backend && exec target/debug/inter-pj-manager) > "$state/api.log" 2>&1 &
api_pid=$!
(cd frontend && exec node_modules/.bin/vite preview --host 127.0.0.1 --port 5173 --strictPort) &
web_pid=$!
echo "Acesso local em http://127.0.0.1:5173 · admin@local.test"
echo "Arquivo privado da senha: $ADMIN_PASSWORD_FILE"
wait -n "$api_pid" "$web_pid"
