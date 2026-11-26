# Referência HTTP interna

## Folha demonstrativa em MOCK

As rotas abaixo existem somente em MOCK, exigem sessão ADMIN e aceitam apenas dados fictícios. Não criam pagamentos nem armazenam chaves Pix. A empresa é determinada pela sessão.

- `GET /api/mock/folha/funcionarios`: lista até 200 funcionários fictícios; acima do limite retorna erro, sem resposta truncada.
- `POST /api/mock/folha/funcionarios`: recebe `nome`, `salario_centavos` como string inteira positiva e UUID `idempotencia`.
- `GET /api/mock/folha/lancamentos?funcionario_id=UUID`: lista até 1.000 lançamentos daquele funcionário da empresa; acima do limite retorna erro.
- `POST /api/mock/folha/lancamentos`: recebe `funcionario_id`, `tipo` (`FALTA_INJUSTIFICADA`, `FALTA_JUSTIFICADA`, `ADIANTAMENTO` ou `PRODUCAO`), `data`, `valor_centavos` (string positiva apenas para adiantamento e produção; `null` para faltas) e UUID `idempotencia`.

Repetição com a mesma chave, autor e conteúdo devolve o mesmo registro; dados diferentes com a mesma chave retornam conflito. A criação e a auditoria são transacionais. A prévia no navegador não é fechamento de folha, nem aprovação ou envio Pix. Ainda faltam correção e cancelamento de lançamentos, revisão de segurança, chave Pix fictícia e fechamento.

| Rota | Acesso | Resposta |
|---|---|---|
| GET /health | Sonda interna da API | Texto ok enquanto o processo atende |
| GET /ready | Sonda | 200 após consulta ao PostgreSQL ou 503 |
| GET /metrics | ADMIN autenticado. Métricas operacionais agregadas sem dados financeiros ou identificadores de empresa |
| POST /api/login | Origem autorizada | JSON com email e password; 200 e cookie em caso de sucesso |
| GET /api/session | Autenticado | id, company_id, company, email e roles da sessão |
| POST /api/logout | Cookie e origem autorizada | 200, expiração do cookie e revogação no servidor |
| GET /api/dashboard | Autenticado com pelo menos um perfil | Snapshot MOCK, as_of, saldo inicial, disponível, entradas e saídas em centavos e lançamentos |
| GET /api/auditoria | ADMIN ou AUDITOR | Até 100 ações locais da empresa, em ordem decrescente, com instante UTC |

Os POST exigem a origem configurada em APP_ORIGIN e o cabeçalho `X-PJ-Request: 1`. Para login use JSON e `Content-Type: application/json`. Corpo máximo de 8 KiB. O cookie expira após oito horas. Respostas da API incluem Cache-Control no-store e X-Request-Id.

401 indica credenciais ou sessão inválidas. 403 indica falta de permissão ou origem indevida. 429 indica limitação de tentativas ou concorrência. 503 indica indisponibilidade. O proxy também limita tentativas de login por IP. Na Fase 1, VIEWER, OPERATOR, APPROVER, AUDITOR e ADMIN podem consultar dados demonstrativos. Os dois últimos podem consultar a auditoria local.

## Leitura bancária da Fase 2

| Rota | Permissão e resultado |
|---|---|
| GET /api/ambiente | Pública. Retorna MOCK, SANDBOX ou PRODUCTION para identificação no login. |
| POST /api/sincronizar | ADMIN da empresa vinculada, com proteção de origem e X-PJ-Request. Retorna 204 após o commit. |
| GET /api/dashboard | Sessão da empresa. Fora de MOCK retorna source, certificate_expires_at, reading e last_error. reading é nulo até a primeira sincronização completa. |
| GET /api/extrato | Sessão da empresa. inicio e fim em AAAA-MM-DD, até 90 dias. pagina começa em zero. Retorna entries e has_more. Valores são strings inteiras de centavos. |

409 indica disputa pela sincronização ou recurso indisponível em MOCK. 429 indica limitação pelo Inter, 502 inclui rejeição de contrato ou erro do banco, e 504 indica timeout. Falhas de autorização pelo Inter são distintas da autorização do usuário local.

## Consulta de cobrança imediata

`POST /api/pix/cobrancas/consultar` recebe JSON com um único campo `txid`. O POST interno mantém o identificador fora da URL e passa pela validação de origem. O backend consulta o recurso bancário por GET. ADMIN, OPERATOR e VIEWER podem consultar a conta da empresa da sessão. O ambiente MOCK aceita `DEMO000000000000000000000001` (ativa) e `DEMO000000000000000000000002` (concluída).

A resposta traz `source`, `txid`, `status`, `revision` e `original_cents` como string de centavos. O backend registra o ambiente, o usuário e o instante de cada consulta bem-sucedida na auditoria local. Uma cobrança ausente resulta em HTTP 404. A falta de sessão resulta em 401, e um perfil sem acesso, recurso desabilitado ou vínculo de empresa divergente resulta em 403. O servidor rejeita um txid inválido antes de chamar o Inter. O painel externo informa a disponibilidade em `pix_lookup_enabled`.

## Emissão de cobrança fictícia

Em MOCK, `POST /api/mock/pix/cobrancas` aceita uma sessão ADMIN ou OPERATOR, a origem configurada e o cabeçalho `X-PJ-Request: 1`. O corpo contém `intent_id` (UUID da intenção) e `original_cents` (string de 1 a 12 algarismos, positiva e sem zeros à esquerda). A empresa vem da sessão. A chave de idempotência é a combinação da empresa com o txid derivado do UUID. O backend registra a cobrança e sua emissão na auditoria em uma transação PostgreSQL.

| Resultado | HTTP |
|---|---|
| Primeira emissão | 201 com txid, origem MOCK, situação ATIVA, revisão zero e valor |
| Repetição da intenção e do valor | 200 com a mesma cobrança, código e expiração |
| Repetição da intenção com outro valor | 409 |
| Valor ou UUID inválido no JSON | 422 |
| Rota em SANDBOX ou PRODUCTION, com cabeçalhos de origem válidos | 404 |

A resposta inclui `mock_code.copy_paste` e `mock_code.expires_at` (RFC 3339). O mesmo resultado pode ser consultado pelo txid em `POST /api/pix/cobrancas/consultar`. O BR Code de demonstração codifica uma URL dinâmica sob `pix.invalid`. Tem validade de 24 horas e dispensa chave Pix, certificado e acesso à rede bancária. A interface mostra o horário em Fortaleza e oculta o QR Code após expirar. O prazo do código é independente da situação de liquidação.

A tela salva o UUID e o valor fictício da intenção na sessão da aba, com chave por empresa e usuário. Uma falha de rede permite retomar com o mesmo UUID. Após uma emissão confirmada, **Iniciar outra emissão** prepara um novo pedido. Guarde o txid para consultar a cobrança depois de fechar a aba.

## Recebimentos e Cobrança V3 em MOCK

| Rota | Permissão e resultado |
|---|---|
| POST /api/clientes | ADMIN ou OPERATOR. Cadastra um cliente na empresa da sessão; repetição idêntica é aceita. |
| POST /api/mock/recebimentos/pix | ADMIN ou OPERATOR em MOCK. Registra uma única confirmação fictícia por txid e retorna o mesmo `end_to_end_id` em reenvios. |
| POST /api/mock/cobrancas-v3 | ADMIN ou OPERATOR em MOCK. Registra a intenção por UUID e retorna `codigo_solicitacao` e situação. |
| POST /api/mock/cobrancas-v3/atualizar | ADMIN ou OPERATOR em MOCK. Simula liquidação por PIX ou BOLETO, ou cancelamento. |
| GET /api/recebimentos | Perfis de consulta da empresa. Lista clientes, cobranças, eventos ou confirmações com paginação. |
| POST /api/recebimentos/reprocessar | ADMIN. Recoloca um evento esgotado na fila persistente. |

O recebimento fica em processamento até o worker consultar novamente a fonte e gravar a confirmação. A chave única do evento evita repetir o mesmo efeito. Uma confirmação informa `CONCILIADO`, `PENDENTE` ou `DIVERGENTE`; solicitação aceita e liquidação são estados distintos.

## MFA TOTP e aprovação Pix em MOCK — Fase 4

| Rota | Permissão e resultado |
|---|---|
| GET /api/mfa | Usuário autenticado; informa se o próprio usuário habilitou TOTP. |
| POST /api/mfa/iniciar | Usuário autenticado; exige senha atual, retorna URI otpauth para cadastro e inicia janela de 10 minutos. |
| POST /api/mfa/confirmar | Usuário autenticado; confirma o cadastro com código TOTP vigente. |
| POST /api/mfa/desativar | Usuário autenticado; exige senha atual e código TOTP. |

O segredo TOTP é cifrado com XChaCha20-Poly1305 pela chave de 32 bytes em `MFA_ENCRYPTION_KEY_FILE`. Códigos têm janela de 30 segundos, tolerância de um passo, bloqueio após cinco falhas por 15 minutos e contador persistido contra replay. Preserve a chave em backup protegido separado do banco. No piloto Pix ela também protege a derivação das chaves de cifragem de destinatários. O cadastro MFA fica disponível em MOCK e para operadores e aprovadores da conta vinculada quando a preparação Pix está habilitada.

## Preparação de pagamentos Pix em MOCK — Fase 4.1

Estas rotas existem somente quando `INTER_ENV=mock` e não enviam dinheiro:

| Rota | Permissão e resultado |
|---|---|
| GET /api/mock/pagamentos/pix?page=N | ADMIN, OPERATOR, VIEWER, APPROVER ou AUDITOR da empresa. Lista intenções fictícias e catálogo sintético de destinatários. |
| POST /api/mock/pagamentos/pix | ADMIN ou OPERATOR. Cria um rascunho idempotente por empresa, autor e `intent_id`; valor em centavos exatos. |
| POST /api/mock/pagamentos/pix/revisar | ADMIN ou OPERATOR, somente o autor. Confirma destinatário e valor persistidos e muda `RASCUNHO` para `EM_REVISAO`. |

Repetições idênticas retornam a mesma intenção; divergências resultam em conflito. Auditoria e mudança de estado são gravadas na mesma transação.

A rota POST `/api/mock/pagamentos/pix/aprovar` exige usuário distinto do autor, perfil `APPROVER` ou `ADMIN`, intenção revisada, valor de 1 centavo a R$ 100.000,00, código TOTP e chave idempotente UUID. O estado `APROVADO_MOCK` não dispara operação bancária. Repetição com mesmo aprovador e chave retorna sucesso sem consumir outro TOTP; chave reutilizada ou decisão divergente conflita. A decisão, consumo do código e auditoria são transacionais.

## Contas demonstrativas em MOCK — Fase 5

| Rota | Permissão e resultado |
|---|---|
| GET /api/mock/financeiro | Lista até 200 contas da empresa da sessão. |
| GET /api/mock/financeiro/relatorio.csv?inicio=AAAA-MM-DD&fim=AAAA-MM-DD | Exporta contas do período, até 366 dias, sempre limitadas à empresa da sessão. |
| POST /api/mock/financeiro | ADMIN ou OPERATOR. Cria conta a pagar/receber com valor em centavos, parte, descrição e vencimento. |
| POST /api/mock/financeiro/alterar | ADMIN ou OPERATOR. Liquida ou cancela somente item aberto da empresa. |
| GET /api/mock/financeiro/lembretes | Lista lembretes da empresa da sessão. |
| POST /api/mock/financeiro/lembretes | ADMIN ou OPERATOR. Registra, de forma idempotente, lembretes locais para itens em aberto com vencimento de hoje até os próximos sete dias. |
| GET /api/mock/financeiro/recorrencias | Lista regras recorrentes MOCK da empresa da sessão. |
| POST /api/mock/financeiro/recorrencias | ADMIN ou OPERATOR. Cria regra recorrente local de pagar/receber com dia do mês e classificação opcional. |
| POST /api/mock/financeiro/recorrencias/gerar | ADMIN ou OPERATOR. Gera lançamentos de um mês/ano escolhido, ajusta dia 29–31 ao último dia do mês e é idempotente por regra e competência. A geração é manual e auditada. |
| POST /api/mock/financeiro/recorrencias/alterar | ADMIN ou OPERATOR. Ativa ou pausa uma regra recorrente da empresa; não altera ocorrências já geradas. |
| GET/POST /api/mock/categorias | Lista ou cria categoria MOCK da empresa. |
| GET/POST /api/mock/centros-custo | Lista ou cria centro de custo MOCK da empresa. |

Os registros são demonstrativos e não alteram saldo bancário nem enviam pagamentos. Cada alteração é auditada na mesma transação.

## Pix Automático demonstrativo em MOCK

Em SANDBOX, `POST /api/sandbox/pix-automatico/consultar` recebe `id_rec` oficial e devolve apenas identificador e status. Exige sessão ADMIN, OPERATOR ou VIEWER da empresa vinculada e scope `rec.read`. A consulta é auditada; credenciais e homologação Inter continuam pendentes.

Estas rotas só são registradas quando `INTER_ENV=mock`; usam a empresa da sessão, validação comum de origem, valores em centavos exatos e auditoria transacional. Não coletam CPF/CNPJ ou dados bancários e não enviam cobranças nem pagamentos.

| Rota | Permissão e resultado |
|---|---|
| GET /api/mock/pix-automatico/recorrencias | Perfis de consulta; lista autorizações fictícias da empresa. |
| POST /api/mock/pix-automatico/recorrencias | ADMIN ou OPERATOR; cria autorização e solicitação de consentimento fictícia em estado `AGUARDANDO_CONSENTIMENTO`. |
| POST /api/mock/pix-automatico/recorrencias/decidir | ADMIN ou OPERATOR; simula aceite/recusa. Repetição da mesma decisão retorna o estado final; decisão oposta conflita. Aceite muda para `ATIVA`; não representa consentimento real. |
| POST /api/mock/pix-automatico/recorrencias/cancelar | ADMIN ou OPERATOR; cancela solicitação pendente ou autorização ativa; repetição após sucesso retorna o estado final sem novo evento de auditoria. |
| GET /api/mock/pix-automatico/cobrancas | Perfis de consulta; lista cobranças fictícias da empresa. |
| POST /api/mock/pix-automatico/cobrancas | ADMIN ou OPERATOR; cria cobrança para autorização ativa, valor fixado e competência ainda não usada. Repetir a competência retorna a mesma cobrança (200), sem nova auditoria. |
| POST /api/mock/pix-automatico/cobrancas/liquidar | ADMIN ou OPERATOR; marca uma cobrança como liquidada em simulação e gera identificador sintético; repetição retorna o mesmo resultado final. |

Escopo funcional do MOCK: um valor fixo por recorrência; competências são lançadas manualmente e a aplicação não envia notificações ao pagador. Não converte dados MOCK em chamadas externas: o contrato do Inter separa `POST /pix/v2/rec` (cadastro da recorrência), `POST /pix/v2/solicrec` (solicitação de consentimento direcionada à conta do pagador) e `POST /pix/v2/cobr` (cobranças após aceite). Requer scopes `rec.write`, `solicrec.write`, `cobr.write` e leitura/callbacks correspondentes. A API exige identificadores oficiais e dados de CPF/CNPJ/conta que a tela MOCK intencionalmente não guarda. Implementação externa depende de definir tratamento protegido e consentimento para esses dados, scopes/credenciais Inter e homologação no sandbox. A elegibilidade foi informada pelo proprietário, sem confirmação independente do banco.


O relatório financeiro CSV MOCK aceita até 10.000 linhas por exportação; se o limite for excedido, retorna 413 e o período deve ser reduzido. Campos com prefixos interpretáveis como fórmulas por planilhas recebem apóstrofo preventivo.

## Pagamentos Pix por chave no piloto

Em SANDBOX ou PRODUCTION, `ENABLE_PIX_PAYMENT_PREPARATION=true` habilita a preparação para usuários da empresa e conta vinculadas. `GET /api/pagamentos/pix` lista as últimas 100 intenções. O POST nessa rota recebe `idempotencia` (UUID), `chave` e `valor_centavos` (string `1`) do operador ou administrador. O destinatário fica cifrado no banco.

O segundo usuário, ADMIN ou APPROVER, revisa chave e valor e confirma em `POST /api/pagamentos/pix/aprovar` com `id`, `chave`, `valor_centavos` e `codigo_mfa`. A aprovação local antecede o envio. A inclusão bancária exige o autor original, nova confirmação TOTP e a rota `POST /api/pagamentos/pix/enviar`, registrada somente com preparação, escrita bancária e pagamentos habilitados explicitamente. O POST em `/api/pagamentos/pix/consultar` recebe o id local e consulta a referência associada à intenção daquela empresa e conta. O comando `manager verificar-acesso-pix` verifica OAuth para os dois escopos.
