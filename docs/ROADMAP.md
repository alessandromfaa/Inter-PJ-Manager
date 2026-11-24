# Roadmap inicial

Este documento descreve o estado identificado no checkout e possibilidades para trabalho futuro. Recomendações e sugestões não são requisitos aprovados. Para critérios por fase, consultar também `implementation-plan.md`; para pendências operacionais recentes, consultar `SESSION_HANDOFF.md`.

## Estado atual

- **Fato observado:** aplicação web de gestão financeira empresarial, com backend Rust/Axum, frontend React/TypeScript, PostgreSQL e execução local via Docker Compose ou scripts nativos documentados.
- **Fato observado:** o ambiente MOCK existe para desenvolvimento e testes. A leitura bancária Inter está implementada com configuração explícita; o estado documentado mantém escritas e pagamentos reais bloqueados.
- **Fato observado:** o plano incremental descreve seis fases. As fases 1 e 2 estão documentadas como implementadas; o ciclo MOCK da fase 3 está concluído, com homologação externa ainda pendente.
- **Fato observado:** `docs/SESSION_HANDOFF.md` registra testes recentes aprovados para backend e frontend, além de uma execução Playwright parcial que ainda requer repetição completa. Esses resultados são registros existentes, não foram reexecutados durante este bootstrap.

## Funcionalidades existentes identificadas

- Autenticação com Argon2id, sessões persistidas, controle de origem, RBAC e auditoria por empresa.
- Painel e extrato, com dados de demonstração em MOCK; sincronização explícita de saldo e extrato Inter conforme configuração documentada.
- Clientes, cobranças Pix e Cobrança V3 em MOCK, persistência de eventos, deduplicação e conciliação.
- Infraestrutura de recepção de callbacks com validação de certificado e consulta posterior à fonte, cuja configuração/homologação externa depende de ambiente autorizado.
- Fase 4.1 iniciada em MOCK: preparação e revisão de intenções Pix com catálogo sintético, idempotência, auditoria e isolamento por empresa; não existe aprovação ou envio.
- Compose local, scripts de inicialização e desenvolvimento, testes automatizados e workflow de CI em MOCK.

## Melhorias recomendadas

- **Recomendação:** repetir o conjunto Playwright completo na stack E2E isolada e registrar resultado, conforme pendência explícita no handoff.
- **Recomendação:** ampliar a cobertura negativa de segregação multiempresa no processamento de recebimentos, indicada na memória do projeto.
- **Recomendação:** manter a matriz `docs/inter-api-matrix.md` atualizada e validar contratos em mock antes de integrar operações bancárias.
- **Recomendação:** explicitar e conferir o modo MOCK e a origem isolada antes de executar testes de navegador: a sobreposição E2E herda variáveis do ambiente e o Playwright usa a porta 8080 por padrão.

## Dívidas técnicas

- **Fato documentado:** licença do código próprio ainda aguarda definição do proprietário.
- **Fato documentado:** cobertura negativa específica de segregação de recebimentos pode ser ampliada.
- **Fato documentado:** há avisos de comentários `@__PURE__` de dependências Zod no build frontend, conforme handoff.

## Problemas conhecidos e pendências

- **Pendente:** repetir o conjunto Playwright completo após uma execução anterior parcialmente aprovada; conferir a evidência do handoff antes de considerar a validação encerrada.
- **Pendente:** homologar no sandbox oficial do Inter emissão, consulta de estado e callbacks para concluir a parte externa da fase 3.
- **Pendente:** revisar a ocorrência de dados sintéticos e sessões relatada em `SESSION_HANDOFF.md`; o documento registra expressamente que não se deve remover registros nem revogar sessões sem inspeção e autorização específicas.
- **Pendente:** concluir revisão final da entrega da fase 3, segundo o handoff.
- **Pendente:** confirmar a elegibilidade da empresa para Pix Automático; a memória afirma que não deve ser presumida.

## Funcionalidades futuras sugeridas

As fases abaixo aparecem no plano incremental existente. Sua presença neste roadmap não aprova nem prioriza sua implementação.

- **Próxima fatia:** pagamentos com aprovação independente, MFA, limites, idempotência final e tratamento de resultado indeterminado; a intenção Pix MOCK deve permanecer sem envio até esses controles.
- **Sugestão:** automação de contas a pagar/receber, relatórios, tarefas agendadas e alertas; avaliar Pix Automático somente após confirmação de elegibilidade.
- **Sugestão:** preparação operacional de produção com monitoramento, recuperação, restauração ensaiada e revisão de segurança.

## Prioridades sugeridas

1. **Prioridade sugerida:** fechar verificações locais já pendentes da fase 3 e concluir a revisão registrada no handoff.
2. **Prioridade sugerida:** tratar homologação externa da fase 3 quando houver ambiente e credenciais autorizados.
3. **Prioridade sugerida:** decidir licença do código próprio e resolver outras pendências de governança.
4. **Prioridade sugerida:** discutir escopo e critérios de aceite das fases 4 a 6 antes de aprovar qualquer implementação.
