# Estado e pendências do projeto

O escopo aprovado no plano incremental compreende seis fases. A Fase 7 está fora do escopo por decisão do proprietário. Este documento resume o estado verificado no checkout; o aceite detalhado está em `docs/implementation-plan.md` e a continuidade operacional em `docs/SESSION_HANDOFF.md`.

## Implementado localmente

- Fases 1 e 2: fundação, autenticação/RBAC, auditoria, consulta bancária conforme configuração e persistência.
- Fase 3 MOCK: clientes, cobranças Pix/V3, callbacks persistidos, deduplicação e conciliação. Homologação oficial continua pendente.
- Fase 4 MOCK: criação e revisão de intenções Pix; MFA TOTP cifrado e aprovação independente/idempotente com limite, anti-replay e sem envio bancário. Validada por teste integrado na stack E2E.
- Fase 5 MOCK: contas a pagar/receber, classificação, CSV, lembretes, regras recorrentes manuais/idempotentes e fila de recebimentos.
- Fase 6 local: métricas ADMIN, backup verificável, restore isolado, HTTPS em loopback e validações locais.

## Pendências para conclusão integral

- Homologar emissão, consulta e callbacks com credenciais oficiais de sandbox Inter.
- Proprietário informou elegibilidade Pix Automático em 25/11/2026. Próximo passo externo: validar habilitação dos scopes e credenciais com Inter e homologar consentimentos, recorrências, cobranças e callbacks no sandbox; a implementação atual limita-se a MOCK.
- Definir retenção, provisionar armazenamento cifrado externo e instalar/monitorar frequência de backup compatível com RPO financeiro proposto de 1 hora. Nenhum agendamento foi configurado.
- Configurar canal e responsáveis para alertas operacionais.
- Medir recuperação ponta a ponta contra as metas propostas de RTO/RPO. O restore de 1 segundo cobriu apenas o banco E2E pequeno e não prova o RTO completo.
- Revisão final de segurança/aceite operacional e publicação externa, se autorizada. Nenhum deploy foi realizado.

## Validação final local

Em 25/11/2026, após MFA/aprovação MOCK, backend formatado, 20 testes do binário (com PostgreSQL), 1 teste de modos e Clippy all-features passaram. Frontend lint, 2 testes unitários, build, audit de produção e Playwright (7/7) passaram. O fluxo MFA/aprovação também passou exercício integrado na stack MOCK: habilitação, aprovação por segundo usuário, repetição idempotente e rejeição de replay. Ver detalhes em `docs/SESSION_HANDOFF.md`.
