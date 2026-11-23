# Referência HTTP da Fase 1

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
