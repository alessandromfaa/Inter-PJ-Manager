# Memória do projeto

Atualizado em 25/11/2026. Este documento registra decisões duradouras confirmadas no checkout. O estado de execução mais recente fica em [SESSION_HANDOFF.md](SESSION_HANDOFF.md). As instruções vigentes continuam em `AGENTS.md` e no wrapper `cx`.

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
- O checkout mantém agentes desabilitados em `.codex/config.toml`; a seleção de modelo é explícita no comando do terminal e não é imposta pelo projeto.
- `AGENTS.md` concentra orientações de trabalho no checkout; `docs/DEVLOG.md` registra trabalhos a partir do bootstrap documental, `docs/ROADMAP.md` separa fatos de propostas e `docs/plans/` destina-se a planos futuros.

## Soluções e limitações confirmadas

- O cliente HTTP do frontend aceita corpo vazio para respostas 204 e para chamadas explicitamente marcadas `allowEmpty`. Chamadas tipadas que precisam de JSON falham com 502 de resposta incompleta.
- O webhook usa certificado de cliente validado por CA, conta previamente vinculada e persistência antes do worker. Registro/configuração e homologação de callbacks no Inter ainda dependem de credenciais e ambiente autorizados.
- Pix Automático demonstrativo só existe em MOCK: estados de solicitação/consentimento são distintos; aceite, cancelamento, cobrança por competência e liquidação toleram reenvios idênticos sem duplicar auditoria ou efeitos. Competências únicas por empresa/autorização impedem duplicação e liquidação simulada gera apenas identificador sintético. Não coletar dados reais de pagadores nesta tela; integração oficial exige adapter próprio, consentimento e homologação.
- MFA TOTP está disponível somente em MOCK. O segredo é cifrado com XChaCha20-Poly1305 sob chave hex de 32 bytes em `MFA_ENCRYPTION_KEY_FILE`; manter a chave protegida e recuperável junto à estratégia de backup. Cadastro expira em 10 minutos, falhas bloqueiam por 15 minutos após cinco tentativas e o contador persistido impede reutilizar o mesmo passo TOTP.
- Aprovação de intenção Pix MOCK exige autor diferente, papel `APPROVER`/`ADMIN`, revisão, MFA e valor até R$ 100.000. Decisão, consumo TOTP, auditoria e chave idempotente são transacionais. O resultado `APROVADO_MOCK` declara `envio_bancario=false`; nenhum envio externo é conectado.
- A Fase 4 MOCK inclui intenções Pix e aprovação independente por TOTP, com autor distinto, papel `APPROVER`/`ADMIN`, revisão e limite de R$ 100.000. Cadastro MFA cifra segredo com XChaCha20-Poly1305, expira em 10 minutos, bloqueia após cinco falhas e persiste contador anti-replay. Decisão, auditoria, consumo TOTP e chave idempotente são transacionais; resultado é `APROVADO_MOCK` e `envio_bancario=false`.
- A Fase 5 possui itens financeiros empresariais em MOCK, estados auditados, totais exatos, exportação CSV e lembretes locais idempotentes consultáveis. Em 25/11/2026 o proprietário informou elegibilidade para Pix Automático; isso não foi verificado independentemente pelo Inter. O fluxo demonstrativo local mantém consentimento, recorrência e cobrança separados e não movimenta dinheiro. Integração externa depende de habilitação de scopes/credenciais e homologação oficial.
- Itens financeiros MOCK aceitam categoria e centro de custo em tabelas isoladas por empresa; relatórios CSV por período são gerados no servidor a partir da sessão autenticada e limitados a 366 dias.
- Regras recorrentes MOCK geram lançamentos somente após ação manual para uma competência escolhida; advisory lock por empresa/período serializa gerações e chave composta empresa+regra+período impede duplicação. Vencimentos 29–31 ajustam ao fim do mês; criação, pausa/reativação e cada lançamento são auditados. A regra pode ser pausada sem alterar lançamentos existentes.
- `/metrics` expõe somente prontidão e indicador de modo, sem métricas financeiras ou identificadores empresariais; exige sessão autenticada com perfil ADMIN.
- O script de backup rejeita dump vazio, valida catálogo e inclusão de dados de `companies`, e cria manifesto com data UTC, tamanho, SHA-256 e estado de restauração não ensaiada. O hash verifica integridade, não recuperabilidade; não há rotação automática.
- A tela financeira MOCK permite cadastrar categorias e centros de custo; os cadastros permanecem isolados por empresa. A operação contínua ainda depende de definir alertas, retenção e rotina comprovada de recuperação.
- A aba Notificações de recebimentos consulta fila persistida em atualização periódica, permite filtrar estados na página atual e expõe reprocessamento manual de eventos esgotados somente a ADMIN. Não há canal externo de alertas configurado.
- Backups não são rotacionados automaticamente. Prazo de retenção fica pendente de decisão do proprietário baseada em obrigações, RPO/RTO e armazenamento; não descartar por padrão.
- Metas propostas pelo proprietário para planejamento: dados financeiros RTO 4 h/RPO 1 h; aplicação/configuração RTO 8 h/RPO 24 h. Ainda precisam de aprovação operacional, ensaios cronometrados e frequência de backup compatível; não são SLOs comprovados.
- Em 25/11/2026, o backup MOCK E2E foi restaurado com sucesso em bancos isolados: uma empresa, oito migrations sem falha e tabelas recorrentes presentes. Manifesto registra hash, horário, tamanho e validação da restauração.
- O ensaio de 25/11/2026 mediu 1 s para `createdb` + `pg_restore` do dump E2E pequeno; não representa RTO ponta a ponta. A operação documenta backup horário com `flock` e alerta de atraso como recomendação para buscar RPO de 1 h, ainda não implantada.
- O proxy HTTPS local em loopback usa TLS 1.2/1.3, HSTS de um ano e cabeçalhos de segurança; certificados locais são de teste e não representam publicação externa.
- A emissão externa de sandbox registra a intenção antes de enviar. Se o resultado não puder ser conhecido, conserva o estado `INDETERMINADA`, sem reenvio automático.
- A cobertura de segregação negativa específica no processamento de recebimentos ainda pode ser ampliada. Os testes PostgreSQL exigem banco descartável em loopback chamado `inter_pj_test`.
- A licença do código próprio aguarda definição do proprietário. O sandbox oficial ainda não foi homologado nesta sessão.

## Decisões do proprietário

- O trabalho ocorre no checkout local. SEFIN está fora do escopo.
- A identidade visual da empresa e a personalização com a logo devem ficar para a etapa final de implantação.
- Nenhuma operação financeira real foi autorizada. Produção deve continuar somente leitura até nova autorização específica.
- Direcionamento atualizado em 25/11/2026: implementar as fases do projeto antes da bateria final de testes e refinamentos. Testes de integração, PostgreSQL, contratos, Playwright e validação completa devem ficar agrupados ao encerramento; até então, não declarar aceite final.
- Nenhuma operação financeira real foi autorizada. Aprovação MOCK pode ser construída com controles apropriados; envio externo permanece bloqueado sem autorização específica e homologação.

## Hipóteses ainda não verificadas

- Habilitação dos scopes Pix Automático, credenciais e funcionamento da empresa no sandbox Inter continuam sem confirmação; a elegibilidade foi declarada pelo proprietário, não verificada independentemente.
- A conclusão externa da Fase 3 depende de homologar emissão, consulta de estados e callbacks no sandbox oficial.
