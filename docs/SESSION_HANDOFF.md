# Continuidade da sessão

## Pix Automático MOCK — 25/11/2026

- O proprietário informou que a empresa é elegível. Isso ainda não foi verificado pelo Inter. A matriz registra escopos e endpoints oficiais existentes e as pendências de credenciais/homologação.
- Implementado fluxo local MOCK para solicitar autorização, simular aceite/recusa, criar cobranças por competência idempotente e simular liquidação. Migration `0011_mock_pix_automatico.sql`; módulo backend isolado; tela própria visível apenas em MOCK.
- Todas as rotas resolvem a empresa da sessão, aplicam RBAC e guard de origem; alterações e auditoria transacionárias. Valores são centavos inteiros. Não há CPF/CNPJ, dados bancários, webhook externo ou chamada/transferência bancária neste MVP.
- Validações concluídas: Rust fmt, 19 testes unitários + 1 teste de modos, Clippy com warnings negados e build release; frontend lint, 2 testes unitários e build; `git diff --check`. Novo E2E Playwright completou autorização, aceite, cobrança e liquidação simulada (1/1).
- Migration 0011 aplicada pelo bootstrap à stack MOCK E2E isolada. Deploy local em `http://127.0.0.1:18080` saudável (`/ready=ok`); web presa a loopback. Volume mantido e migration 0007/checksum preservados. Stack de leitura configurada à parte não foi alterada.
- Próximo passo externo: validar scopes e credenciais Pix Automático com Inter e executar homologação oficial de consentimentos, cobranças e callbacks no sandbox. Nenhuma operação bancária real ocorreu; Fase 7 continua excluída.

## Deploy local da branch — 25/11/2026

- Reconstruída a stack `inter-pj-manager-e2e` em MOCK com o código da branch, mantendo o volume PostgreSQL existente. API e frontend saudáveis; `/ready` respondeu `ok` em `http://127.0.0.1:18080`.
- O primeiro bootstrap recusou a migration 0007 porque o volume E2E já tinha o checksum da versão anteriormente aplicada. Comparação SHA-384 confirmou que o checksum registrado corresponde ao arquivo SQL com uma linha vazia final; esse byte foi preservado para não reescrever histórico de migration. Depois, bootstrap iniciou e SQLx confirmou versões 7, 9 e 10 com `success=true`.
- A publicação Compose estava vinculada a `172.17.154.31`; tornou-se configurável por `APP_BIND_ADDRESS` e o frontend foi recriado em `127.0.0.1:18080`. Nenhuma porta foi exposta em interface de rede externa. Stack local configurada para leitura de produção não foi alterada.
- A migration 0007 tem uma linha vazia final intencional para manter o checksum do volume E2E existente; `git diff --check` pode sinalizá-la. Não altere nem reformate a migration após aplicação. Segredos e dados sintéticos seguem apenas em `secrets/e2e/` e no volume E2E ignorado pelo Git.

## Retomada final — MFA e aprovações MOCK — 25/11/2026

- Implementados endpoints TOTP de status, início, confirmação e desativação; segredos ficam cifrados em `user_mfa_credentials`, com expiração do cadastro, confirmação de senha, bloqueio por falhas e rejeição de replay.
- Implementada rota MOCK de aprovação de intenções Pix. Exige perfil elegível, aprovador diferente do autor, intenção em revisão, limite de R$ 100.000, código MFA e UUID idempotente. Consumo MFA, decisão e auditoria compartilham transação; resposta confirma que nenhum envio bancário foi realizado.
- Frontend da tela de pagamentos oferece cadastro por QR, confirmação/desativação TOTP e aprovação. Compose injeta chave dedicada; scripts de início MOCK geram chave se ausente. Documentação de API, desenvolvimento, operação e plano foi atualizada; memória registra o requisito de preservar a chave.
- Validações finais após o incremento: `cargo fmt --all -- --check`; `cargo test --locked --features testes-postgres` (20 testes no binário e 1 de modos); `cargo clippy --locked --all-targets --all-features -- -D warnings`; `cargo build --locked --release`; `cargo audit` (277 dependências, sem avisos); frontend `npm run lint`, `npm test -- --run` (2 testes), `npm run build`, `npm audit --omit=dev` (0 vulnerabilidades) e Playwright E2E (7/7) passaram. O build emite dois avisos Rollup sobre comentários `@__PURE__` do Zod.
- Exercício integrado MFA/aprovação na stack E2E: cadastro TOTP, intenção revisada por ADMIN, aprovação por usuário APPROVER distinto, repetição da mesma chave idempotente e rejeição do replay do mesmo TOTP passaram; a resposta confirma `envio_bancario=false`. Migration 0010 aplicada na stack E2E saudável. O ensaio deixou somente usuários/intents sintéticos na base E2E isolada; nenhuma limpeza destrutiva foi feita.
- O PostgreSQL descartável de teste foi encerrado e removido. A stack E2E permaneceu ativa; não houve commit, push, publicação ou ação na stack local de produção.
- Pendências externas: homologação Inter no sandbox com credenciais autorizadas; scopes Pix Automático; decisão de retenção e canal de alertas/armazenamento cifrado externo; definição de licença. Elegibilidade Pix Automático foi informada pelo proprietário, sem confirmação independente do banco. Fase 7 excluída por solicitação.

## Encerramento da Fase 6 — 25/11/2026

- O proprietário retirou explicitamente a Fase 7 do escopo. Fase 6 executada localmente na stack MOCK E2E isolada; a stack local em modo produção somente leitura não foi alterada nem publicada.
- Validações aprovadas: frontend lint, 2 testes unitários, build, audit npm (0 vulnerabilidades de produção), Playwright 7/7; backend fmt, Clippy todas as features, testes normais e PostgreSQL, build release e `cargo audit`; diff, configuração Compose e sintaxe do script de backup.
- Backup E2E e restauração em bancos novos isolados validados: hash/manifesto válidos, uma empresa, 8 migrations sem falha e tabelas recorrentes existentes. Os artefatos permanecem em `backups/`, protegidos e ignorados pelo Git.
- `/metrics`: 401 sem sessão e métricas agregadas para ADMIN. HTTPS local em loopback respondeu 200 e emitiu HSTS e demais cabeçalhos de segurança.
- A validação manual de recorrências encontrou um SELECT de criação com placeholders desalinhados. Correção em `backend/src/financeiro.rs`, imagem E2E recompilada e cenário repetido com sucesso: primeira geração criou 1 lançamento, segunda criou 0, vencimento ajustado para 29/02/2028.
- Restore cronometrado do dump MOCK E2E em banco descartável: 1 s, com SHA-256 conferido, uma empresa, oito migrations sem falhas e tabelas recorrentes. Isso mede somente `createdb` + `pg_restore` nesse conjunto pequeno, não RTO ponta a ponta.
- `docs/operacao.md` agora recomenda agendamento horário do backup com bloqueio de concorrência e alerta para falha/idade acima de 60 minutos como desenho para buscar RPO financeiro de 1 h. O agendamento não foi instalado; cópia externa cifrada e retenção continuam sem definição, portanto a meta não está garantida.
- Metas propostas para planejamento: dados financeiros RTO 4 h/RPO 1 h; aplicação/configuração RTO 8 h/RPO 24 h. Ainda falta comprovação cronometrada e política de backups/retention alinhada. Também falta configurar canal externo de alertas; publicação externa só mediante autorização específica. Sem homologação bancária ou transação real.

## Bateria final solicitada — 25/11/2026

- Backend em contêiner descartável: `cargo fmt --check`, `cargo test --locked` (15 unitários + 1 teste de modos), `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo test --locked --features testes-postgres` (17 testes com PostgreSQL descartável) e `cargo build --locked --release` passaram.
- Frontend em contêiner descartável: `npm run lint`, `npm test -- --run` (2 testes), `npm run build` e `npm audit --omit=dev` (0 vulnerabilidades) passaram. Build emitiu os dois avisos conhecidos de comentários `@__PURE__` nas dependências Zod.
- Playwright repetido contra a stack E2E reconstruída: 7/7 passaram. Uma tentativa inicial usou imagem sem Chromium e falhou por falta do executável; a repetição na imagem Playwright correta passou integralmente.
- O primeiro teste backend com feature PostgreSQL foi iniciado sem `TEST_DATABASE_URL` e falhou por ausência de variável; repetido com banco descartável em loopback e URL explícita, passou. O banco temporário foi encerrado e removido.
- Revisão do plano identificou que o projeto inteiro ainda não está concluído: MFA e rota de aprovação da Fase 4 não existem; homologação oficial das operações/callbacks da Fase 3 depende de credenciais; Pix Automático depende de elegibilidade; retenção, armazenamento cifrado externo e canal de alertas não foram configurados. Não declarar conclusão integral.

## Continuação em 25/11/2026 — evidência de backup e aceite operacional

- `infrastructure/scripts/backup.sh` prepara manifesto protegido junto ao dump, com horário UTC, tamanho, SHA-256, validação do catálogo e `restauracao_ensaiada=false`. Hash não substitui restauração. Nenhum backup foi criado nesta alteração.
- `docs/operacao.md` descreve conferência do manifesto e mantém retenção manual, sem exclusão automática.
- `docs/implementation-plan.md` ganhou matriz do aceite da Fase 6 separando evidência local de ensaios/decisões pendentes.
- Não executei o script, testes ou ensaio de recuperação. A verificação do script e do manifesto fica para a bateria final em MOCK; produção permanece fora do escopo.

## Continuação em 25/11/2026 — notificações e retenção

- A tela existente de notificações de recebimento agora mostra a contagem de eventos `ESGOTADO` da página, filtra estados da fila e preserva o reprocessamento manual ADMIN. A consulta já atualiza a cada quatro segundos; não há e-mail/webhook de alerta externo configurado.
- Política operacional esclarece que backups permanecem sob retenção manual até decisão do proprietário baseada em exigências aplicáveis, RPO/RTO e capacidade cifrada. A rotina não apaga nem rotaciona backups automaticamente.
- Segurança atualizada para refletir que `/metrics` exige ADMIN e continua restrito à rede operacional privada.
- Não executei testes nem abri serviços. Ficam pendentes validação final de interface/fila, definição externa de canal de alertas e prazo de retenção, além do ensaio de restauração isolado.

## Continuação em 25/11/2026 — recorrências financeiras MOCK

- Migration 0009 agora inclui regras recorrentes financeiras e ocorrências vinculadas ao lançamento, com FKs compostas por empresa.
- Backend oferece listar/criar regras, pausar/reativar e gerar lançamentos para mês/ano solicitado. A execução é manual, transacional, auditada e idempotente por empresa/regra/competência; datas alvo acima do último dia do mês são ajustadas. Não há agendamento nem integração bancária.
- Tela de contas ganhou cadastro/pausa de regras e ação explícita de geração mensal; invalida as consultas afetadas após sucesso.
- Contrato registrado em `docs/api-interna.md`; plano e memória atualizados.
- Não executei testes nem apliquei migrations, conforme a instrução de guardar a bateria para o final. SQL, concorrência, migrations e comportamento de interface permanecem sem validação executável nesta fatia; validar na bateria final.

## Continuação em 25/11/2026 — classificação financeira e operação

- A tela MOCK de contas agora oferece cadastro de categoria ou centro de custo; após criação, o seletor correspondente é atualizado. Backend já tinha as rotas e mantém os registros por empresa, com auditoria e rejeição de duplicatas.
- O endpoint `/metrics` agora exige sessão autenticada ADMIN, pois é encaminhado pelo NGINX junto às rotas acessíveis pelo navegador.
- `docs/operacao.md` registra responsáveis/canais de alerta, monitoramento da idade do backup, ensaio periódico em banco isolado e necessidade de definir retenção; o script não faz rotação automática.
- `docs/implementation-plan.md` atualiza o estado das fases 5 e 6. A Fase 4 continua pendente de MFA verificável e fluxo de aprovação, com envio externo desabilitado.
- Ainda não executei a bateria final de testes/refinamentos, conforme orientação do proprietário. Próximas etapas: revisar lacunas de implementação restantes no plano e preparar, ao final, validação de migrations/API, testes completos, auditoria, build e E2E isolado.

Atualizado em 24/11/2026. Escopo desta retomada: continuar a Fase 3 com isolamento verificável dos testes E2E, sem deploy, commit ou push. O checkout está em `/home/alessandro/codex/inter`, branch `main`, com commit base `1453433`.

## Preferência operacional vigente

Direcionamento atualizado em 25/11/2026: implementar as fases do projeto e deixar testes de integração, PostgreSQL, contratos, Playwright e refinamentos para a bateria final. Não declarar aceite final até a validação agrupada.

## Bootstrap documental posterior

- Criados `docs/DEVLOG.md`, `docs/ROADMAP.md` e `docs/plans/.gitkeep`; `AGENTS.md` foi complementado com arquitetura e comandos identificados. As alterações anteriores do checkout foram preservadas.
- As verificações e pendências da Fase 3 abaixo são registros anteriores, não foram reexecutadas nesta tarefa documental. Consulte o estado Git atual antes de prosseguir.

## Concluído nesta linha de trabalho

- Fase 4.1 implementada em MOCK: tela e API para criar/listar rascunhos de pagamento Pix, confirmar destinatário/valor e encaminhar para `EM_REVISAO`; nenhuma operação bancária é chamada.

- Fases 1 e 2 constam como implementadas. A leitura real de saldo/extrato foi validada anteriormente com a conta de produção autorizada.
- Fase 3 em MOCK recebeu clientes, solicitação V3, simulação de recebimento Pix, fila persistente, deduplicação e conciliação.
- Corrigido o cliente HTTP para aceitar corpos vazios apenas onde esperado. Respostas sem JSON em operações tipadas agora são falha de contrato.
- Ampliados testes do adapter local para `cob.write`, `boleto-cobranca.write`, `pix.read` e `boleto-cobranca.read`.
- Criada stack Playwright separada por projeto, volume, empresa e segredos E2E. Instruções estão em `docs/desenvolvimento.md`.
- Fechado o caminho de fallback dos testes: Playwright agora exige `E2E_ORIGIN=http://127.0.0.1:18080`, `admin@e2e.local` e senha explícita; o overlay fixa MOCK e bloqueia escritas/pagamentos.
- O cliente HTTP não aceita mais `204` vazio em operações que exigem representação; foi adicionado cenário E2E para esse contrato.
- Sincronização bancária, cujo contrato retorna `204`, foi marcada explicitamente com `allowEmpty`; comandos e documentação usam o projeto Compose fixo `inter-pj-manager-e2e`.
- Provisionamento E2E agora aplica `umask 077` antes da criação e recusa conjunto parcial de segredos.
- A Fase 4.1 passou revisão de segurança sem P0/P1; a validação de DTO de criação/revisão foi confirmada. A limitação de troca de sessão entre abas permanece registrada como P2 de MOCK.
- Atualizados plano, matriz do Inter, API interna, README e documento de continuidade.
- A instância local HTTPS com configuração de produção somente leitura foi restaurada após um E2E ter reutilizado inadvertidamente a base da stack original.

## Ocorrência operacional importante

Na primeira execução do novo E2E, duas linhas de cliente sintéticas foram gravadas na tabela `customers` da base local usada pela instância de leitura de produção. A contagem foi verificada por nome sintético. Não houve exclusão. Sessões criadas pelos testes também podem existir, mas a tabela `user_sessions` não registra data/origem para atribuição segura. Não revogar sessões nem remover clientes sem inspeção e autorização específicas. A instância HTTPS foi restaurada sem nova sincronização e respondeu 200. Escritas e pagamentos bancários permaneceram desativados.

## Arquivos desta alteração de Fase 3

- Backend: `backend/tests/postgres/seguranca.rs`, `backend/src/infrastructure/banking/inter/tests.rs`.
- Frontend: `frontend/src/services/api.ts`, `frontend/src/main.tsx`, `frontend/src/pages/recebimentos.tsx`, `frontend/e2e/recebimentos.spec.ts`.
- Execução: `compose.e2e.yml`.
- Documentação: `README.md`, `docs/api-interna.md`, `docs/continuidade-chatgpt.md`, `docs/desenvolvimento.md`, `docs/implementation-plan.md`, `docs/inter-api-matrix.md`.
- Também alterado por esta tarefa de memória: `AGENTS.md`, `docs/PROJECT_MEMORY.md`, `docs/SESSION_HANDOFF.md`.

## Verificações registradas

- Rust: `cargo fmt --check` e `cargo clippy --locked --all-targets --features testes-postgres -- -D warnings` passaram. Testes PostgreSQL com banco descartável passaram: 13 testes do binário e 1 teste de integração. Testes sem feature passaram: 11 e 1.
- Frontend: Vitest 2 passou, ESLint passou e `npm run build` passou. O build reporta avisos de comentários `@__PURE__` em dependências Zod.
- Playwright: conjunto isolado anterior passou 6 testes; após as correções de isolamento e contrato 204, o conjunto completo precisa ser repetido. O ambiente atual não possui `npm` disponível, portanto essa repetição não foi executada nesta retomada.
- Compose: `docker compose -p inter-pj-manager-e2e -f compose.yml -f compose.e2e.yml config` passou; `git diff --check` passou. `npm run lint/test/build` não pôde ser executado porque o comando `npm` não está instalado neste shell.
- `git diff --check` passou antes da atualização destes arquivos de memória.
- Revisões anteriores encontraram e ajudaram a corrigir o compartilhamento acidental da base e imprecisões menores. A revisão final permanece pendente.

## Estado atual do Git

O trabalho da Fase 3 ainda não foi commitado. As mudanças listadas acima estavam modificadas ou não rastreadas no início desta consolidação. Não fazer commit ou push sem solicitação do usuário. Arquivos locais de produção, certificados e segredos não devem ser lidos em conteúdo nem incluídos no Git.

## Próximos passos

0. Política preparatória da Fase 4.2 adicionada em `backend/src/pagamentos.rs` com teste unitário; a rota de aprovação continua bloqueada até implementação de MFA verificável, persistência/idempotência e revisão de segurança.
1. Repetir o conjunto Playwright completo contra `http://127.0.0.1:18080`, confirmando os sete cenários na stack E2E isolada, quando Node/npm estiver disponível.
2. Conferir a instância em `https://127.0.0.1:8443/ready` e confirmar no container `INTER_ENV=production`, `ENABLE_BANK_READS=true`, `ENABLE_BANK_WRITES=false`, `ENABLE_REAL_PAYMENTS=false`. Não executar sincronização de produção sem pedido explícito.
3. Finalizar Clippy/testes após quaisquer correções, `git diff --check` e conferir que segredos/artefatos E2E continuam ignorados.
4. Revisões independentes confirmaram fechamento dos achados P1/P2 específicos; ainda falta executar Playwright no ambiente com Node/npm disponível e então registrar o resultado funcional.
5. Prosseguir com o próximo escopo somente depois de tratar o aceite restante da Fase 3. Pagamentos da Fase 4 exigem MFA, aprovação separada, idempotência, limites e revisão de segurança.

## Retomada de 24/11/2026

- O checkout estava limpo em `729f83f`; o handoff anterior mencionava alterações não commitadas que já estão incorporadas nesse commit.
- Alterados `backend/src/pagamentos.rs` e `docs/api-interna.md` para registrar a política preparatória de aprovação MOCK.
- `cargo fmt --check` e `cargo test --locked --lib` não foram executados porque `cargo` não está disponível neste shell. Nenhum teste de integração, PostgreSQL, contrato ou Playwright foi executado.

## Retomada de 25/11/2026

- Removidos os perfis locais de `.codex/agents/` e a documentação de multiagentes.
- `.codex/config.toml` agora mantém agentes desabilitados e não impõe modelo; a seleção fica no comando explícito do terminal.
- Atualizados `AGENTS.md` e `docs/PROJECT_MEMORY.md` para remover delegação automática e referências ao modelo anterior.
- Validados `git diff --check` e a leitura da configuração TOML; não foram executados testes de aplicação porque a alteração é somente de configuração/documentação.

## Validação unitária de 25/11/2026

- Com as toolchains locais do checkout, `backend`: `cargo test --locked --bin inter-pj-manager` passou com 14 testes unitários.
- `frontend`: `npm test -- --run` passou com 2 testes em 1 arquivo.
- Não foram executados PostgreSQL, integração, contratos, Playwright, lint ou build; a decisão vigente mantém somente testes unitários durante as próximas fases.

## Retomada adicional de 25/11/2026

- Reavaliada a pendência de cobertura de segregação multiempresa em `recebimentos::worker`: o processamento já seleciona eventos pelo escopo de empresa/conta e consulta/atualiza as tabelas MOCK usando `company_id`; a confirmação externa também consulta cobranças pela empresa e conta da sessão vinculada.
- A evidência negativa correspondente está no teste PostgreSQL `backend/tests/postgres/seguranca.rs`, portanto não foi executado nesta fase por restrição explícita a testes unitários. Nenhuma alteração de lógica foi feita sem poder validar esse fluxo permitido.
- Próximo trabalho de código pode seguir em pagamentos MOCK sem ativar aprovação até haver mecanismo MFA verificável; a pendência de teste multiempresa fica para a validação agrupada final.

## Incremento de pagamentos MOCK — 25/11/2026

- Criada migration `backend/migrations/0007_mock_payment_approvals.sql` para persistir aprovador, instante da decisão e chave idempotente, com FK multiempresa, unicidade da chave por empresa/aprovador, proibição de autoaprovação e exigência de revisão anterior.
- A rota de aprovação permanece fechada porque não há MFA provisionado/verificável no sistema; nenhum envio bancário foi implementado.
- Verificação pendente: somente testes unitários são permitidos durante esta fase, mas a migration exige validação PostgreSQL/integração e será incluída na bateria final. Não foi aplicada a nenhum banco.

## Continuação da Fase 4.2 — 25/11/2026

- A migration registra decisão na própria intenção, com chave idempotente, unicidade por empresa/aprovador/chave, vínculo multiempresa e constraints de autor/revisão.
- `pagamentos.rs` agora testa unitariamente que a política de registro exige revisão anterior, ausência de aprovação prévia, perfil/limite válidos e aprovador distinto.
- A rota continua não registrada no Axum enquanto não houver MFA verificável. Testes unitários executados após a alteração ainda precisam ser registrados abaixo.

## Novo direcionamento — 25/11/2026

- Proprietário orientou executar todas as etapas de implementação do projeto e agrupar testes completos e refinamentos ao final. Isso substitui a restrição anterior de executar somente testes unitários durante as fases.
- Permanecem os limites de não operar produção, não movimentar dinheiro real e não publicar/deployar sem autorização específica.

## Fases 5 e 6 — implementação local em andamento

- Fase 5: criadas migration e rotas MOCK para contas a pagar/receber, baixa/cancelamento com auditoria, resumo por tipo, filtro, CSV e geração idempotente/listagem de lembretes locais para vencimentos de hoje até sete dias.
- Pix Automático não foi implementado: a documentação oficial do Inter condiciona a oferta a CNPJ com pelo menos seis meses e a memória exige confirmar a elegibilidade da empresa.
- Fase 6: `/metrics` retorna prontidão e indicador MOCK, sem dados de negócio; documentação operacional descreve o limite da métrica.
- Fase 4: migration de decisões e invariantes em código/testes unitários existem, mas MFA verificável ainda não foi implementado. A rota de aprovação e qualquer envio permanecem ausentes.
- Verificações direcionadas do incremento: `cargo fmt --check`, `cargo check --locked --offline`, `cargo test --locked --bin inter-pj-manager` (15 passaram), `cargo clippy --locked --offline --bin inter-pj-manager -- -D warnings`, `npm run build`, `npm run lint` e `npm test -- --run` (2 passaram). Build frontend conserva dois avisos Rollup em comentários `@__PURE__` de Zod.
- Pendentes para o encerramento: validar migrations e rotas em PostgreSQL, cenários de API, contratos, Playwright, suíte completa e revisão de segurança. Nenhuma migration foi aplicada a uma base nesta etapa.

## Continuação da Fase 5 — relatório e classificação MOCK

- `mock_financial_items` recebeu vínculos compostos com tabelas específicas de categorias e centros de custo MOCK por empresa.
- Criadas rotas de consulta/criação de categorias e centros, e seletores no formulário. Referências são verificadas pela empresa do usuário.
- O relatório CSV agora é gerado no backend por intervalo de datas de até 366 dias, com filtragem pela empresa da sessão e escape de campos; o download mantém a sessão autenticada.
- Validações direcionadas após o incremento: `cargo fmt --check`, `cargo check --locked --offline`, `cargo clippy --locked --offline --bin inter-pj-manager -- -D warnings`, `cargo test --locked --bin inter-pj-manager` (15 passaram), `npm run build`, `npm run lint` e `npm test -- --run` (2 passaram). Os avisos Rollup de comentários `@__PURE__` em Zod permanecem.
- A validação PostgreSQL das migrations 0007 e 0009 e os testes de API seguem para a bateria final; nenhuma migration foi aplicada.

## Fase 6 — reforço de backup e recuperação

- `infrastructure/scripts/backup.sh` agora falha para dump vazio e verifica catálogo e inclusão da tabela `companies` antes de criar o nome final do backup.
- `docs/operacao.md` reforça que restauração só deve ocorrer em banco novo e isolado e pede confirmação das tabelas `companies` e `_sqlx_migrations`.
- Nenhum backup foi criado e nenhum banco foi criado/restaurado nesta alteração.
- Verificação de shell e teste de backup permanecem pendentes para a validação final; os comandos envolveriam o serviço de banco local, que não foi iniciado.

## Fatos, decisões e pendências

- **Fato confirmado:** Compose de produção local estava saudável após restauração, API reportou modo PRODUCTION e sincronização de leitura anterior retornou saldo e extrato sem erro. A URL HTTPS local usa certificado próprio de teste.
- **Decisão do proprietário:** MOCK no desenvolvimento, produção somente leitura, SEFIN fora de escopo, identidade visual da logo apenas no final.
- **Pendente:** sandbox oficial para emissão/consultas/callbacks, limpeza dos dois clientes sintéticos e política sobre sessões cuja origem não pode ser reconstruída, licença do código próprio e revisão final completa.
- **Pendente da Fase 4.1:** validação final da resposta no frontend foi adicionada; permanece como limitação P2 a possibilidade de uma aba antiga tentar retomar um rascunho após troca de sessão. A sessão do backend continua sendo a autoridade de empresa e autor, sem exposição cruzada.
- **Não verificável:** os valores completos usados para estabelecer as credenciais não são necessários à continuidade e foram omitidos. O histórico pode ter informação adicional que não está nos arquivos atuais, portanto o checkout e os documentos oficiais permanecem a fonte para confirmar detalhes.
