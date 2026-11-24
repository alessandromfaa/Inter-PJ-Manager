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

## Transição da leitura bancária para recebimentos

O aceite da Fase 2 reúne consultas pontuais de saldo e extrato vazio e com lançamentos na conta de produção autorizada. A sincronização com PostgreSQL, a paginação, a concorrência, a reimportação e o rollback foram exercitados com dados sintéticos. O sandbox oficial continua pendente de acesso. A aplicação permite prosseguir com a Fase 3 em MOCK.

| Marco da Fase 3 | Situação e evidência esperada |
|---|---|
| Consulta de cobrança imediata | Implementada por txid no painel. O adaptador valida identificador, situação, revisão, valor e escopo OAuth. A consulta externa depende de habilitação explícita e permissão na conta. |
| Emissão Pix e QR Code | Entregue em MOCK. PostgreSQL preserva txid, valor, código e expiração no reenvio. O teste de navegador perde a primeira resposta e compara o texto decodificado do QR ao Pix Copia e Cola após retomar. Emissão pelo Inter continua pendente. |
| Recebimento Pix em MOCK | Entregue. O recebimento fictício relaciona txid e endToEndId, a fila deduplica o evento e a confirmação só registra liquidação após consultar a fonte MOCK. |
| Clientes e Cobrança V3 | Entregue em MOCK. O pagador pertence à empresa da sessão e a solicitação acompanha codigoSolicitacao, processamento, recebimento ou cancelamento. Emissão externa permanece opt-in de SANDBOX. |
| Webhooks e conciliação | Infraestrutura implementada para Pix e V3, com receptor mTLS em loopback, associação da conta, persistência antes do processamento, deduplicação, retentativas e consulta posterior à fonte. A configuração de callbacks e a homologação externa dependem do ambiente autorizado. |

O ciclo MOCK da Fase 3 está concluído. O aceite automatizado cobre repetição concorrente, estados pendente e recebido, valor exato, canal e conciliação Pix/V3. As constraints e consultas por empresa preservam a segregação, cuja cobertura negativa específica de recebimentos ainda será ampliada. A conclusão externa da fase requer homologar emissão, consulta posterior e callbacks no sandbox oficial; produção permanece somente leitura.
