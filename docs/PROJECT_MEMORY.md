# Memória do projeto

Atualizado em 24/11/2026. Este documento registra decisões duradouras confirmadas no checkout. O estado de execução mais recente fica em [SESSION_HANDOFF.md](SESSION_HANDOFF.md). As instruções vigentes continuam em `AGENTS.md` e no wrapper `cx`.

## Arquitetura e decisões confirmadas

- Backend Rust com Axum/Tokio, SQLx/PostgreSQL, domínio e adaptadores bancários separados. Frontend React/TypeScript usa Vite, TanStack Query e interface em pt-BR. Compose mantém PostgreSQL, bootstrap, API e NGINX em serviços separados, com healthchecks e containers restritos.
- A empresa é resolvida da sessão no backend. Vínculos persistidos usam empresa e chaves compostas. Valores monetários trafegam em centavos inteiros ou strings decimais exatas. Datas de armazenamento são UTC e a interface apresenta datas brasileiras.
- MOCK é o padrão de desenvolvimento. O adaptador Inter usa OAuth client credentials, mTLS e Rustls, validação TLS do servidor, token em cache por escopo, renovação concorrente controlada, limitadores de frequência e timeouts. Produção exige configuração explícita de leitura e origem HTTPS. Escritas e pagamentos reais permanecem bloqueados.
- A fundação inclui Argon2id, sessões revogáveis em cookie HttpOnly/SameSite e Secure em produção, validação de origem, RBAC e auditoria. Credenciais, certificados e chaves ficam em arquivos privados locais ignorados pelo Git.
- Recebimentos em MOCK guardam cobranças e recebimentos fictícios em tabelas próprias. Clientes pertencem à empresa e são compartilhados entre ambientes. Eventos persistem antes do processamento, têm chave de deduplicação e confirmação posterior da fonte. Cobrança aceita, em processamento, a receber e liquidada são estados diferentes.
- O adaptador implementa consultas individuais de cobrança Pix e confirmações Pix/V3. Emissão Pix e V3 externas está limitada ao perfil opt-in de SANDBOX. O contrato e os scopes ficam registrados em `docs/inter-api-matrix.md`. Homologação externa continua necessária.
- O Compose E2E usa projeto, volume, empresa e arquivos de segredo próprios. Testes de navegador nunca devem compartilhar a base de clientes/sessões da instância local configurada com produção.
- A configuração Playwright falha fechada fora da origem/conta E2E (`127.0.0.1:18080`, `admin@e2e.local`); o overlay força MOCK e desabilita escritas e pagamentos.
- Segredos E2E são criados com `umask 077`; o fluxo recusa conjunto parcial para não divergir da senha persistida no volume.
- Codex tem perfis especializados em `.codex/agents/` e configuração no checkout em `.codex/config.toml`. A documentação operacional correspondente está em `docs/multiagentes.md`.
- `AGENTS.md` concentra orientações de trabalho no checkout; `docs/DEVLOG.md` registra trabalhos a partir do bootstrap documental, `docs/ROADMAP.md` separa fatos de propostas e `docs/plans/` destina-se a planos futuros.

## Soluções e limitações confirmadas

- O cliente HTTP do frontend aceita corpo vazio para respostas 204 e para chamadas explicitamente marcadas `allowEmpty`. Chamadas tipadas que precisam de JSON falham com 502 de resposta incompleta.
- O webhook usa certificado de cliente validado por CA, conta previamente vinculada e persistência antes do worker. Registro/configuração e homologação de callbacks no Inter ainda dependem de credenciais e ambiente autorizados.
- A Fase 4.1 adiciona intenções de pagamento Pix exclusivamente em MOCK: catálogo sintético, valor exato, idempotência empresa+autor+chave, revisão explícita pelo próprio autor e auditoria transacional. Não há aprovação, MFA ou envio.
- A emissão externa de sandbox registra a intenção antes de enviar. Se o resultado não puder ser conhecido, conserva o estado `INDETERMINADA`, sem reenvio automático.
- A cobertura de segregação negativa específica no processamento de recebimentos ainda pode ser ampliada. Os testes PostgreSQL exigem banco descartável em loopback chamado `inter_pj_test`.
- A licença do código próprio aguarda definição do proprietário. O sandbox oficial ainda não foi homologado nesta sessão.

## Decisões do proprietário

- O trabalho ocorre no checkout local. SEFIN está fora do escopo.
- A identidade visual da empresa e a personalização com a logo devem ficar para a etapa final de implantação.
- Nenhuma operação financeira real foi autorizada. Produção deve continuar somente leitura até nova autorização específica.
- Durante a implementação das próximas fases, somente testes unitários podem ser executados. Integração, PostgreSQL, contratos, Playwright e a suíte completa ficam para o encerramento do conjunto de fases; o aceite permanece pendente até essa validação final.

## Hipóteses ainda não verificadas

- A elegibilidade da empresa para Pix Automático depende de confirmação do Inter e não deve ser presumida.
- A conclusão externa da Fase 3 depende de homologar emissão, consulta de estados e callbacks no sandbox oficial.
