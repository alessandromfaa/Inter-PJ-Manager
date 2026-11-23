# Plano incremental e critérios de aceite

| Fase | Entrega | Evidência exigida |
|---|---|---|
| 1 · Fundação | Login, RBAC, sessões, painel MOCK, extrato filtrável, auditoria, PostgreSQL e Compose | Inicialização limpa, migrations, login, 401 sem sessão, 403 no papel incorreto e navegação por navegador |
| 2 · Leitura | Cliente OAuth/mTLS em Rustls, cache por integração, saldo e extrato, sincronização incremental e persistência | Contratos e paginação testados em mock HTTP, token renovado uma vez sob concorrência, deduplicação e homologação quando houver credenciais |
| 3 · Recebimentos | Pix cobrança, V3, clientes, webhooks persistidos e conciliação | QR e identificadores conferidos, solicitação aceita acompanha estado posterior, callback autenticado e repetido produz um único efeito |
| 4 · Pagamentos | Preparação, revisão, aprovação com MFA e envio autorizado | Bloqueio de autoaprovação, repetição, excesso de limite e envio por padrão. Resultado indeterminado exige consulta de estado |
| 5 · Automação | Contas a pagar e receber, relatórios, jobs, alertas e Pix Automático elegível | CSV/PDF e totais reconciliados, falhas recuperáveis, elegibilidade confirmada por conta |
| 6 · Produção | Operação HTTPS, auditoria, monitoramento, recuperação e revisão de segurança | Restauração cronometrada, ensaios de falha, revisão de dependências e aceite do proprietário |

O esquema inicial reserva as entidades financeiras. Formulários, regras, workers e endpoints correspondentes entram com os testes da respectiva fase. Cada módulo precisa registrar o contrato vigente, escopos, limites, paginação, erros e suporte no sandbox antes da ativação.

## Prioridade imediata da Fase 2

Implementar leitura de saldo com conversão decimal exata e fixtures contratuais. Em seguida, adicionar extrato paginado com checkpoints persistentes por conta e transações de importação idempotentes. Webhooks e operações de escrita continuam nas fases subsequentes.
