# Inter PJ Manager

- Ao iniciar uma sessão, leia `docs/PROJECT_MEMORY.md` e `docs/SESSION_HANDOFF.md` se existirem. Trate-os como contexto auxiliar e confirme o estado atual no checkout.
- Ao concluir trabalho relevante, atualize esses documentos de forma concisa, sem segredos, respeitando o escopo e as restrições do modo `cx` ativo. Consulte as regras globais do Codex e os prompts do wrapper para limites de gravação, Git e infraestrutura.

- Escreva interface, mensagens operacionais e documentação em português brasileiro.
- Desenvolva no checkout local. O ambiente SEFIN está fora do escopo. Serviços de desenvolvimento escutam em loopback.
- Use MOCK no desenvolvimento e nos testes comuns. Bloqueie inicialização com ambiente desconhecido ou escrita bancária habilitada enquanto a capacidade estiver indisponível.
- Verifique o contrato publicado pelo Inter e atualize docs/inter-api-matrix.md antes de integrar uma operação. Registre incertezas e teste o adapter sem movimentação real.
- Resolva a empresa a partir da sessão no backend. Use SQL parametrizado e chaves compostas para vínculos multiempresa.
- Faça cálculos financeiros com representação exata. Distinga solicitação, aprovação, processamento, liquidação e resultado indeterminado.
- Preserve Argon2id, validação de origem, RBAC, cookies HttpOnly e Secure em produção. Escritas sensíveis exigem MFA, dupla autorização e idempotência.
- Mantenha certificados, chaves, dumps, senhas e tokens fora de Git, logs e frontend.
- Execute formatação, lint, auditorias, testes e build pertinentes. Relate separadamente validações documentais, mocks e homologação bancária.
- Solicite autorização para operações destrutivas, publicação, mudanças em produção ou transação real.

## Projeto e execução

O Inter PJ Manager é uma aplicação de gestão financeira empresarial. O frontend React acessa a API Axum pela mesma origem; o backend autentica e resolve a empresa, persiste no PostgreSQL e usa adaptadores MOCK ou Inter. Consulte `docs/architecture.md`, `docs/api-interna.md` e `docs/inter-api-matrix.md` para os limites e contratos; confirme o código quando a documentação estiver desatualizada.

- `backend/`: Rust edição 2024, Axum 0.8, Tokio e SQLx 0.9.1; `src/main.rs` registra rotas, e `src/api`, `src/auth.rs`, `src/recebimentos` e `src/infrastructure/banking` contêm handlers, autenticação, recebimentos e adaptadores. As migrations SQL estão em `backend/migrations/` e os testes em `backend/tests/`.
- `frontend/`: React 19, TypeScript 5.9, Vite 7, Tailwind 4 e TanStack Query 5; telas em `src/pages/`, cliente HTTP em `src/services/`, testes Vitest em `tests/` e Playwright em `e2e/`.
- `infrastructure/`: Dockerfile do backend e scripts locais; `compose.yml` integra PostgreSQL 17, bootstrap, API e NGINX. `compose.inter.yml` configura integração externa e `compose.e2e.yml` isola a pilha de testes. `.env.example` documenta as variáveis sem credenciais.
- `docs/` reúne arquitetura, operação, segurança, contrato HTTP, matriz Inter e plano de implementação. `.github/workflows/qualidade.yml` executa verificações em MOCK. Não foi identificado Makefile ou pipeline de deploy.

### Comandos verificados na documentação e configuração

Na raiz, em um clone novo sem `.env`, crie-o com `cp .env.example .env`; para iniciar a instalação local MOCK, use `./infrastructure/scripts/iniciar.sh`. Inspecione com `docker compose ps -a` e `curl -f http://127.0.0.1:8080/ready`. O script herda o ambiente: confira previamente que a configuração efetiva é MOCK. `docker compose down` encerra a pilha preservando o volume. O script `./infrastructure/scripts/mock-local.sh` oferece execução nativa; Ctrl+C encerra seus processos. Leia `README.md` e `docs/desenvolvimento.md` antes de iniciar serviços ou alterar configuração.

Em `backend/`: `cargo fmt --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo test --locked` e `cargo build --locked --release`. A suíte `cargo test --locked --features testes-postgres` requer `TEST_DATABASE_URL` para banco descartável em `127.0.0.1` chamado `inter_pj_test`; ela aplica migrations e grava dados sintéticos. A inicialização do backend também aplica migrations; o bootstrap `cargo run --locked -- bootstrap` cria os dados iniciais. Confira o destino do banco e as permissões antes de executar.

Em `frontend/`: `npm ci --legacy-peer-deps`, `npm run lint`, `npm test`, `npm run build` e `npm run dev -- --host 127.0.0.1 --port 5173 --strictPort`. A documentação requer Node 22.12 ou posterior da série 22 e Rust estável com rustfmt e Clippy. `cargo audit` e `npm audit` são as auditorias documentadas. Para Playwright, siga a configuração isolada de `docs/desenvolvimento.md`, confirme MOCK no ambiente efetivo e, com a pilha isolada na porta 18080 e as credenciais E2E configuradas, execute em `frontend/`: `E2E_ORIGIN=http://127.0.0.1:18080 npm run e2e`. Não use uma instância com dados de produção.

Antes de alterar, confira Git, instruções locais e efeitos em dados e contratos; preserve alterações existentes. Depois, execute as verificações pertinentes e informe limitações, distinguindo testes locais de homologação bancária. Não execute autonomamente exclusão de volumes persistentes ou backups, restauração sobre produção ou outras operações destrutivas críticas. Migrations em produção, publicação, transações reais e exposição de serviços exigem autorização específica aplicável. Nunca registre segredos ou conteúdo de arquivos privados.

## Coordenação multiagente

O coordenador usa o modelo e o esforço de raciocínio definidos em `.codex/config.toml`. Os perfis em `.codex/agents/` vinculam cada especialidade ao modelo e ao esforço acordados.

Se o cliente avisar que ignorou a configuração do projeto, omitir o perfil solicitado ou registrar um modelo ou esforço divergente, suspenda a delegação afetada. Informe a divergência e siga o guia de ativação e diagnóstico. [Operação multiagente](docs/multiagentes.md)

1. Ao receber uma tarefa, classifique sua natureza, complexidade (rotineira, intermediária ou arquitetural) e risco (baixo, moderado, alto ou crítico). Identifique arquivos, limites de confiança e critério de aceite. Consultas simples exigem apenas uma classificação breve.
2. Delegue o trabalho do projeto automaticamente ao perfil correspondente. Use `backend` para Rust/API, `frontend` para React/UX, `database` para SQL/migrations, `tester` para verificação e `documentation` para documentação. Em tarefas mistas, separe as responsabilidades e indique quem integra. Use o nome do perfil na chamada de delegação.
3. Acione `architect` antes de decisões arquiteturais relevantes, especialmente quando alteram limites entre módulos, contratos públicos, persistência ou recuperação de falhas.
4. Atribua integrações com o Banco Inter a `banking`. Exija a fonte oficial e evidências distintas para MOCK, sandbox e produção.
5. Toda mudança de funcionalidade financeira exige revisão de `security`, por agente diferente do autor, inclusive mudanças de autenticação, exposição de dados, callbacks e operações idempotentes.
6. Use paralelismo apenas quando entradas, dependências e critérios de aceite permitirem trabalho independente. O limite é de quatro subagentes concorrentes. O coordenador organiza as filas e integra resultados dependentes em sequência.
7. Atribua um responsável por arquivo durante cada rodada. Se surgir sobreposição de edição, serialize as alterações ou redistribua a propriedade antes de escrever. Preserve as alterações locais do usuário. Revisores trabalham em leitura.
8. Antes de declarar a implementação concluída, execute os testes pertinentes ao diff e registre comando e resultado. Para configuração, exercite o carregamento pelo consumidor real. Uma verificação pendente deve aparecer no aceite.
9. Cada entrega relevante passa por `reviewer` independente depois da implementação. O autor corrige os achados e o revisor confirma a resolução. A revisão financeira de `security` também é obrigatória.
10. P0 compreende impacto crítico imediato, como exposição ativa de segredo ou movimentação indevida. P1 compreende falha grave plausível, como quebra de isolamento ou duplicação de operação financeira. Com P0/P1 aberto, bloqueie o aceite, a publicação e o avanço de fase até correção e nova revisão.
11. No relatório, registre por tarefa os agentes acionados, modelo e esforço observados, evidência de execução e resultado. Distinga configuração solicitada e metadados efetivos. Se o runtime omitir um dado, identifique-o como indisponível.
12. Se faltar um perfil, modelo ou nível de raciocínio, interrompa a parte dependente, informe o item indisponível e solicite decisão humana para uma substituição. Não escolha silenciosamente outro modelo ou agente genérico.
13. Envie a cada agente o menor contexto suficiente, com objetivo, arquivos, restrições e saída esperada. Evite análises duplicadas e criação de agentes sem trabalho definido. Encerre agentes concluídos para liberar capacidade.
14. Solicite aprovação humana antes de operações destrutivas, exposição à internet, alteração em produção, publicação externa, transação financeira real ou exceção à política de modelos. Considere a aprovação já concedida para o escopo exato. SEFIN permanece fora do escopo.
