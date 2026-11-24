# Referência HTTP interna

| Rota | Acesso | Resposta |
|---|---|---|
| GET /health | Sonda interna da API | Texto ok enquanto o processo atende |
| GET /ready | Sonda | 200 após consulta ao PostgreSQL ou 503 |
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

## Preparação de pagamentos Pix em MOCK — Fase 4.1

Estas rotas existem somente quando `INTER_ENV=mock` e não enviam dinheiro:

| Rota | Permissão e resultado |
|---|---|
| GET /api/mock/pagamentos/pix?page=N | ADMIN, OPERATOR, VIEWER, APPROVER ou AUDITOR da empresa. Lista intenções fictícias e catálogo sintético de destinatários. |
| POST /api/mock/pagamentos/pix | ADMIN ou OPERATOR. Cria um rascunho idempotente por empresa, autor e `intent_id`; valor em centavos exatos. |
| POST /api/mock/pagamentos/pix/revisar | ADMIN ou OPERATOR, somente o autor. Confirma destinatário e valor persistidos e muda `RASCUNHO` para `EM_REVISAO`. |

Não há rota de aprovação, autorização MFA ou execução bancária nesta fatia. Repetições idênticas retornam a mesma intenção; divergências resultam em conflito. Auditoria e mudança de estado são gravadas na mesma transação.
