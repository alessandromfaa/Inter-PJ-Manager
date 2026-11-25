# Plano incremental e critérios de aceite

| Fase | Entrega | Evidência exigida |
|---|---|---|
| 1 · Fundação | Login, RBAC, sessões, painel MOCK, extrato filtrável, auditoria, PostgreSQL e Compose | Inicialização limpa, migrations, login, 401 sem sessão, 403 no papel incorreto e navegação por navegador |
| 2 · Leitura | Cliente OAuth/mTLS em Rustls, cache por integração, saldo e extrato, sincronização incremental e persistência | Contratos e paginação testados em mock HTTP, token renovado uma vez sob concorrência, deduplicação e homologação quando houver credenciais |
| 3 · Recebimentos | Pix cobrança, V3, clientes, webhooks persistidos e conciliação | QR e identificadores conferidos, solicitação aceita acompanha estado posterior, callback autenticado e repetido produz um único efeito |
| 4 · Pagamentos | Preparação, revisão, aprovação com MFA e envio autorizado | Bloqueio de autoaprovação, repetição, excesso de limite e envio por padrão. Resultado indeterminado exige consulta de estado |
| 5 · Automação | Contas, relatórios e fluxos MOCK; Pix Automático demonstrativo | Totais reconciliados, idempotência por competência; integração bancária validada separadamente no sandbox |
| 6 · Produção | Operação HTTPS, auditoria, monitoramento, recuperação e revisão de segurança | Restauração cronometrada, ensaios de falha, revisão de dependências e aceite do proprietário |

## Estado da execução

- Direcionamento do proprietário em 25/11/2026: concluir as implementações viáveis das fases antes de executar a bateria de testes e refinamentos finais.
- Fase 4 local em MOCK implementa cadastro TOTP com segredo cifrado, expiração, bloqueio por falhas, proteção contra replay e aprovação idempotente por segundo usuário com limite de R$ 100.000,00. Testes unitários, integração HTTP manual em ambiente E2E isolado e Playwright passaram. Nenhuma aprovação MOCK envia ordem bancária; envio Inter permanece desabilitado e homologação externa continua pendente.
- Fase 5 implementada em MOCK na camada local: contas demonstrativas a pagar/receber, baixa/cancelamento auditados, resumo exato, filtro, CSV, lembretes idempotentes, cadastro de categorias/centros de custo, regras recorrentes com geração mensal manual/idempotente, notificações de recebimento e fluxo Pix Automático demonstrativo separado. O proprietário informou elegibilidade em 25/11/2026; scopes, credenciais e homologação Inter continuam pendentes.
- Fase 6 executada no ambiente local isolado: `/metrics` validado com 401 sem sessão e dados agregados para ADMIN; backup MOCK com manifesto e ensaio de restauração em banco separado; TLS local validado com HSTS; testes de aplicação, lint, build, auditorias de dependências e E2E passaram. Metas propostas para planejamento: dados financeiros RTO 4 h/RPO 1 h; aplicação e configuração RTO 8 h/RPO 24 h. Ainda não comprovadas em ensaio nem sustentadas por política de backup final. Nada foi publicado nem implantado em produção.
- Bateria de 25/11/2026: `cargo fmt --check`, 15 unitários + 1 teste de modos, Clippy com todas as features, 17 testes PostgreSQL, build release; frontend lint, 2 unitários, build, `npm audit --omit=dev` (zero vulnerabilidades) e Playwright (7/7) passaram. O build frontend mantém dois avisos Rollup em anotações `@__PURE__` de Zod.
- A validação local da Fase 6 foi realizada em MOCK isolado, incluindo restauração e cenário manual de recorrência. Ela não representa homologação externa, backup fora do host ou aceite de produção.
- O projeto completo ainda não está concluído: homologação Inter das operações da Fase 3 e Pix Automático depende de credenciais e sandbox autorizado; canal externo de alertas, retenção e storage cifrado externo dependem de decisão/configuração operacional. Publicação depende de autorização específica. Fase 7 permanece fora do escopo por decisão do proprietário.
- Por decisão explícita do proprietário, a Fase 7 está fora do escopo desta execução.

### Aceite operacional da Fase 6

| Critério | Estado local | Pendência de aceite |
|---|---|---|
| Métricas agregadas | Validado: sem autenticação retorna 401; ADMIN recebe somente prontidão e modo MOCK | Coleta contínua depende de monitor externo |
| Backup verificável | Dois backups MOCK E2E com manifesto, SHA-256 e permissões restritas | Definir retenção conforme obrigações e armazenamento; compatibilizar frequência com RPO financeiro de 1 h |
| Restauração | Ensaio cronometrado do banco isolado: 1 s para restaurar o dump de teste; uma empresa, 8 migrations sem falha e tabelas recorrentes presentes | A medição não inclui detecção, provisionamento, aplicação, configuração ou validação funcional e não comprova RTO financeiro de 4 h nem da aplicação de 8 h; testar também RPOs de 1 h/24 h |
| Retenção | Sem descarte automático; controle manual documentado | Proprietário definir prazo, obrigações e capacidade cifrada |
| Alertas de operação | Eventos de recebimento têm fila, filtros, atualização periódica e reprocessamento ADMIN | Definir canal externo para indisponibilidade, backup, disco e certificados |
| HTTPS e publicação | TLS local em loopback validado; HSTS e cabeçalhos de segurança presentes | Publicação, domínio e destino externo continuam fora do escopo sem autorização específica |

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
