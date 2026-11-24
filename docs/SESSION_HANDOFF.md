# Continuidade da sessão

Atualizado em 24/11/2026. Escopo desta retomada: continuar a Fase 3 com isolamento verificável dos testes E2E, sem deploy, commit ou push. O checkout está em `/home/alessandro/codex/inter`, branch `main`, com commit base `1453433`.

## Preferência operacional vigente

Por decisão do proprietário, durante as próximas fases podem ser executados apenas testes unitários. Testes de integração, PostgreSQL, contratos, Playwright e a validação completa serão agrupados ao concluir o conjunto de fases; até lá, registrar verificações pendentes e não declarar aceite final.

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
- A Fase 4.1 passou revisão independente de segurança sem P0/P1; a validação de DTO de criação/revisão foi confirmada. A limitação de troca de sessão entre abas permanece registrada como P2 de MOCK.
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
- Revisores independentes anteriores encontraram e ajudaram a corrigir o compartilhamento acidental da base e imprecisões menores. A revisão final foi solicitada, mas os agentes terminaram por limite de uso antes de devolver resultado.

## Estado atual do Git

O trabalho da Fase 3 ainda não foi commitado. As mudanças listadas acima estavam modificadas ou não rastreadas no início desta consolidação. Não fazer commit ou push sem solicitação do usuário. Arquivos locais de produção, certificados e segredos não devem ser lidos em conteúdo nem incluídos no Git.

## Próximos passos

1. Repetir o conjunto Playwright completo contra `http://127.0.0.1:18080`, confirmando os sete cenários na stack E2E isolada, quando Node/npm estiver disponível.
2. Conferir a instância em `https://127.0.0.1:8443/ready` e confirmar no container `INTER_ENV=production`, `ENABLE_BANK_READS=true`, `ENABLE_BANK_WRITES=false`, `ENABLE_REAL_PAYMENTS=false`. Não executar sincronização de produção sem pedido explícito.
3. Finalizar Clippy/testes após quaisquer correções, `git diff --check` e conferir que segredos/artefatos E2E continuam ignorados.
4. Revisões independentes confirmaram fechamento dos achados P1/P2 específicos; ainda falta executar Playwright no ambiente com Node/npm disponível e então registrar o resultado funcional.
5. Prosseguir com o próximo escopo somente depois de tratar o aceite restante da Fase 3. Pagamentos da Fase 4 exigem MFA, aprovação separada, idempotência, limites e revisão de segurança.

## Fatos, decisões e pendências

- **Fato confirmado:** Compose de produção local estava saudável após restauração, API reportou modo PRODUCTION e sincronização de leitura anterior retornou saldo e extrato sem erro. A URL HTTPS local usa certificado próprio de teste.
- **Decisão do proprietário:** MOCK no desenvolvimento, produção somente leitura, SEFIN fora de escopo, identidade visual da logo apenas no final.
- **Pendente:** sandbox oficial para emissão/consultas/callbacks, limpeza dos dois clientes sintéticos e política sobre sessões cuja origem não pode ser reconstruída, licença do código próprio e revisão final completa.
- **Pendente da Fase 4.1:** validação final da resposta no frontend foi adicionada; permanece como limitação P2 a possibilidade de uma aba antiga tentar retomar um rascunho após troca de sessão. A sessão do backend continua sendo a autoridade de empresa e autor, sem exposição cruzada.
- **Não verificável:** os valores completos usados para estabelecer as credenciais não são necessários à continuidade e foram omitidos. O histórico pode ter informação adicional que não está nos arquivos atuais, portanto o checkout e os documentos oficiais permanecem a fonte para confirmar detalhes.
