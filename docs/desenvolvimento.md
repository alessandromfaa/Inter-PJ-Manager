# Desenvolvimento e testes

O Compose é o caminho de instalação integrado. Para desenvolvimento nativo, use Rust estável com rustfmt e Clippy, Node.js 22.12 ou posterior da série 22 e um PostgreSQL local dedicado. Execute o binário Rust a partir de backend, onde se encontra migrations. O processo nativo lê o ambiente exportado pelo shell. O arquivo .env na raiz fornece a interpolação ao Compose.

## Backend

Exporte DATABASE_URL apontando a um banco exclusivo, DB_PASSWORD_FILE apontando ao arquivo privado da senha, ADMIN_EMAIL e ADMIN_PASSWORD_FILE. A senha inicial deve ter entre 16 e 256 bytes. Configure `MFA_ENCRYPTION_KEY_FILE` com o caminho de um arquivo contendo 64 caracteres hexadecimais de uma chave aleatória de 32 bytes; mantenha a chave estável e protegida. Use um diretório privado de segredos. Para desenvolvimento com Vite na porta 5173 defina APP_ORIGIN=http://127.0.0.1:5173.

```sh
cd backend
cargo run --locked -- bootstrap
cargo run --locked
```

A API escuta por padrão em 127.0.0.1:8080. O bootstrap pode ser repetido. Para verificar o backend:

```sh
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --locked --release
cargo install cargo-audit --locked
cargo audit
```

## Teste de integração com PostgreSQL

Crie previamente um banco descartável chamado inter_pj_test em 127.0.0.1. Exporte TEST_DATABASE_URL com acesso a esse banco. A suíte verifica host e nome antes de aplicar migrations e criar empresas e usuários sintéticos com identificadores aleatórios. Os dados do ensaio permanecem exclusivamente nesse banco.

```sh
cargo test --locked --features testes-postgres
```

A suíte verifica origem, cookie, consultas sem autenticação, troca de permissões na sessão existente, segregação dos registros de auditoria, logout e limitação de tentativas inválidas. O teste unitário verifica o saldo a partir dos lançamentos assinados.

## Frontend

```sh
cd frontend
npm ci --legacy-peer-deps
npm run lint
npm test
npm run build
npm audit
npm run dev -- --host 127.0.0.1 --port 5173 --strictPort
```

Vite encaminha /api ao backend local. A interface usa React, TanStack Query, React Hook Form, Zod, Recharts, Tailwind e a composição Button do shadcn/ui com Radix Slot. Os testes de apresentação usam Vitest. [Referência do componente Button](https://ui.shadcn.com/docs/components/button)

## Teste de navegador

Use uma stack própria para o navegador. O arquivo `compose.e2e.yml` define outro projeto Compose, volume PostgreSQL, empresa e segredos. A porta permanece em loopback. Não execute Playwright contra a instância configurada com dados de produção.

```sh
umask 077
mkdir -p secrets/e2e
existing=$(find secrets/e2e -maxdepth 1 -type f | wc -l)
if [ "$existing" -ne 0 ] && [ "$existing" -ne 3 ] && [ "$existing" -ne 4 ]; then
  echo "Conjunto parcial de segredos E2E; restaure os arquivos necessários antes de continuar." >&2
  exit 1
fi
for secret in db_root db_app admin mfa_key; do
  test -e "secrets/e2e/$secret" || openssl rand -out "secrets/e2e/$secret" -hex 32
done
chmod 700 secrets/e2e
chmod 444 secrets/e2e/*
APP_BIND_ADDRESS=127.0.0.1 APP_PORT=18080 APP_ORIGIN=http://127.0.0.1:18080 \
  docker compose -p inter-pj-manager-e2e -f compose.yml -f compose.e2e.yml up -d --build
E2E_ORIGIN=http://127.0.0.1:18080 \
E2E_EMAIL=admin@e2e.local \
E2E_PASSWORD="$(cat secrets/e2e/admin)" \
  npm --prefix frontend exec playwright test
```

O Playwright recusa qualquer origem, conta ou senha que não pertença à pilha isolada acima. Não execute a suíte contra o Compose padrão ou contra uma base que contenha dados de produção. `CHROME_PATH` permite usar um Chromium compatível já instalado. Playwright valida o formulário, autenticação, navegação, filtro do extrato, consulta de auditoria, revogação e os fluxos de recebimentos. A CI executa verificações em MOCK e publica somente o resultado dos testes. A chave `secrets/e2e/mfa_key` é exclusiva dessa stack e deve permanecer disponível enquanto houver credenciais MFA sintéticas no volume E2E.
