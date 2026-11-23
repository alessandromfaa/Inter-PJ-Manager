# Desenvolvimento e testes

O Compose é o caminho de instalação integrado. Para desenvolvimento nativo, use Rust estável com rustfmt e Clippy, Node.js 22.12 ou posterior da série 22 e um PostgreSQL local dedicado. Execute o binário Rust a partir de backend, onde se encontra migrations. O processo nativo lê o ambiente exportado pelo shell. O arquivo .env na raiz fornece a interpolação ao Compose.

## Backend

Exporte DATABASE_URL apontando a um banco exclusivo, DB_PASSWORD_FILE apontando ao arquivo privado da senha, ADMIN_EMAIL e ADMIN_PASSWORD_FILE. A senha inicial deve ter entre 16 e 256 bytes. Use um diretório privado de segredos. Para desenvolvimento com Vite na porta 5173 defina APP_ORIGIN=http://127.0.0.1:5173.

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

Com a pilha MOCK iniciada localmente, configure E2E_ORIGIN para a URL em loopback, E2E_EMAIL para o administrador de teste e E2E_PASSWORD para sua senha. Para o Compose padrão, na raiz do projeto:

```sh
export E2E_PASSWORD="$(cat secrets/admin)"
cd frontend
npx playwright install chromium
npm run e2e
unset E2E_PASSWORD
```

CHROME_PATH permite usar um Chromium compatível já instalado. Playwright valida o formulário, autenticação, navegação, filtro do extrato, consulta de auditoria e revogação. A CI executa verificações em MOCK e publica somente o resultado dos testes.
