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
