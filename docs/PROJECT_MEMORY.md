# Memória do projeto

## Módulo de pagamentos de funcionários — regras propostas

- Proprietário deseja cadastrar funcionários, salário mensal definido por ele e chave Pix; registrar faltas, adiantamentos e valores de produção informados manualmente. Não calcular INSS, IRPF ou outros encargos.
- Base de cada pagamento: 50% do salário no dia 05 e 50% no dia 20. Faltas e adiantamentos de 06 a 20 pertencem ao pagamento do dia 20; os de 21 a 05 ao pagamento do dia 05. O proprietário confirmou que a falta do dia 05 entra no fechamento do próprio dia. Produção extra compõe o pagamento correspondente.
- Desconto de uma falta injustificada: salário mensal dividido por 30, conforme proprietário. A simulação arredonda a diária ao centavo mais próximo e atribui ao dia 05 o centavo restante de salário ímpar; essas convenções precisam de validação operacional. Pendente definir tratamento de lançamentos após fechamento. Dados reais de pessoal exigem proteção e regras trabalhistas precisam ser validadas pela contabilidade. Integração com pagamentos reais depende de homologação e autorização específica.
- Proprietário deseja vincular a folha a pagamentos Pix agendados, condicionando o crédito à sua aprovação prévia. Distinguir aprovação local, eventual aprovação no banco, agendamento e liquidação; a pessoa que prepara deve ser diferente da que aprova conforme política existente. A capacidade de agendar Pix por chave e as condições bancárias ainda exigem verificação de contrato e homologação; não presumir que uma aprovação local impeça uma instrução já enviada ao Inter de liquidar.
- Em MOCK há cadastro persistente de funcionários e lançamentos fictícios, restrito a ADMIN e isolado por empresa. A migration 0015 e o código local impedem faltas ativas duplicadas por dia e oferecem cancelamento auditado e idempotente; a correção exige cancelar e recriar. O histórico mantém cancelamentos fora da prévia. Antes de aplicar a migration a uma base existente, verificar duplicatas e a autorização aplicável. A migration 0016 acrescenta alterações salariais auditadas para datas de pagamento futuras (05 ou 20), sem modificar o salário inicial; nesta versão não há edição ou cancelamento dessas alterações. A prévia é calculada no navegador com o salário vigente na data do pagamento; ainda não há fechamento, chave Pix ou vínculo com pagamentos. Não usar com dados reais ou como folha definitiva.

Atualizado em 25/11/2026. Este documento registra decisões duradouras confirmadas no checkout. O estado de execução mais recente fica em [SESSION_HANDOFF.md](SESSION_HANDOFF.md). As instruções vigentes continuam em `AGENTS.md` e no wrapper `cx`.

## Estado publicado — 25/11/2026

- O incremento de extrato detalhado, identidade visual Brilhe Móveis e piloto Pix em preparação foi commitado em `1d8b145` (`feat: prepara piloto pix e detalha extrato`) e publicado na branch `main` do GitHub.
- A validação local passou para backend, frontend e build Docker. O arquivo de logo duplicado `brilhe.png` permaneceu fora do commit; a logo usada pela aplicação está em `frontend/public/logo-brilhe-moveis.png`.
- A publicação não inclui `.env`, certificados, chaves, tokens, dumps ou senhas. O remoto Git local usa SSH autenticado.
- O piloto Pix continua limitado a R$ 0,01 e em preparação; flags de escrita bancária e pagamento real permanecem desativadas. A verificação de acesso ao escopo `pagamento-pix.read` em produção retornou HTTP 401, portanto nenhum Pix foi enviado.

## Instalação da outra estação — 25/11/2026

- Docker Engine/Moby 29.9.3 e Docker Compose 5.7.1 foram instalados no Fedora 44 e o serviço Docker foi habilitado.
- Componentes e diretórios locais do motor anterior foram removidos. O projeto e a documentação passaram a mencionar somente Docker.
- A instalação nova foi inicializada em MOCK com empresa e administrador novos. O banco usa volume Docker novo; a senha inicial e os segredos permanecem somente em arquivos locais protegidos.
- A pilha está ativa em `http://127.0.0.1:8080`; `/ready`, página, login inicial e `/api/session` retornaram HTTP 200. Nenhum arquivo da integração Inter foi transferido e nenhuma consulta bancária real foi feita.
- Após autorização em 25/11/2026, a instalação foi preparada para `APP_ENV=production`, `INTER_ENV=production` e leitura bancária habilitada, com escritas/pagamentos falsos. As credenciais mTLS foram transferidas por SSH verificado; o proxy local HTTPS usa `https://127.0.0.1:8443` e a porta HTTP direta foi fechada.
- A verificação autorizada de saldo ao Inter excedeu o timeout; não houve valor retornado nem transação. Login, sessão e prontidão locais por HTTPS passaram.
- Reteste posterior: handshake mTLS e OAuth `extrato.read` passaram; uma sincronização autorizada pelo endpoint da aplicação retornou 204, job `CONCLUIDO` e leitura persistida. O timeout anterior foi transitório.

## Arquitetura e decisões confirmadas

- Backend Rust com Axum/Tokio, SQLx/PostgreSQL, domínio e adaptadores bancários separados. Frontend React/TypeScript usa Vite, TanStack Query e interface em pt-BR. Compose mantém PostgreSQL, bootstrap, API e NGINX em serviços separados, com healthchecks e containers restritos.
- A empresa é resolvida da sessão no backend. Vínculos persistidos usam empresa e chaves compostas. Valores monetários trafegam em centavos inteiros ou strings decimais exatas. Datas de armazenamento são UTC e a interface apresenta datas brasileiras.
- MOCK é o padrão de desenvolvimento. O adaptador Inter usa OAuth client credentials, mTLS e Rustls, validação TLS do servidor, token em cache por escopo, renovação concorrente controlada, limitadores de frequência e timeouts. Produção exige configuração explícita de leitura e origem HTTPS. Escritas e pagamentos reais permanecem bloqueados.
- A fundação inclui Argon2id, sessões revogáveis em cookie HttpOnly/SameSite e Secure em produção, validação de origem, RBAC e auditoria. Credenciais, certificados e chaves ficam em arquivos privados locais ignorados pelo Git.
- Recebimentos em MOCK guardam cobranças e recebimentos fictícios em tabelas próprias. Clientes pertencem à empresa e são compartilhados entre ambientes. Eventos persistem antes do processamento, têm chave de deduplicação e confirmação posterior da fonte. Cobrança aceita, em processamento, a receber e liquidada são estados diferentes.
- O adaptador implementa consultas individuais de cobrança Pix e confirmações Pix/V3, e consulta individual de recorrência Pix Automático restrita a SANDBOX com escopo `rec.read`. Emissão Pix e V3 externas está limitada ao perfil opt-in de SANDBOX. O contrato e os scopes ficam registrados em `docs/inter-api-matrix.md`. Homologação externa continua necessária.
- O Compose E2E usa projeto, volume, empresa e arquivos de segredo próprios. Testes de navegador nunca devem compartilhar a base de clientes/sessões da instância local configurada com produção.
- A configuração Playwright falha fechada fora da origem/conta E2E (`127.0.0.1:18080`, `admin@e2e.local`); o overlay força MOCK e desabilita escritas e pagamentos.
- Segredos E2E são criados com `umask 077`; o fluxo recusa conjunto parcial para não divergir da senha persistida no volume.
- O checkout mantém agentes desabilitados em `.codex/config.toml`; a seleção de modelo é explícita no comando do terminal e não é imposta pelo projeto.
- `AGENTS.md` concentra orientações de trabalho no checkout; `docs/DEVLOG.md` registra trabalhos a partir do bootstrap documental, `docs/ROADMAP.md` separa fatos de propostas e `docs/plans/` destina-se a planos futuros.

## Soluções e limitações confirmadas

- Em 25/11/2026 as migrations 6, 7, 9, 10 e 11 foram aplicadas, mediante autorização específica, à base local de leitura de produção após backup validado; versões 1–7 e 9–11 sem falha. A API antiga permaneceu em execução. A instalação local não tinha `secrets/mfa_key` no momento; preparar chave estável e protegida antes de eventual implantação da nova API, após avaliar credenciais MFA existentes.
- O proxy HTTPS local usa `.tools/local-tls/nginx.conf` e encaminha ao frontend existente em `172.17.154.31:8080`; esse endereço é específico da máquina e deve ser reconferido se a publicação mudar.

- Em 25/11/2026, migrations 6, 7, 9, 10 e 11 foram aplicadas e repetidas com sucesso pelo migrador SQLx sobre cópia do backup local de produção em PostgreSQL temporário sem rede, em tmpfs. Imagem de ensaio tinha migrations idênticas ao checkout. Isso não valida permissões de produção, desempenho, implantação nem homologação bancária; nenhuma migration foi aplicada à base de produção.

- Em 25/11/2026, dump da instância local de leitura de produção foi restaurado em PostgreSQL temporário sem rede, com dados em tmpfs; uma empresa e migrations 1–5 sem falhas. A restauração usou `--no-owner --no-privileges` e não valida permissões, recuperação ponta a ponta ou RTO. A cópia temporária foi descartada. Checkout contém migrations posteriores 6, 7, 9, 10 e 11, que exigem autorização antes de aplicação na base de produção.

- Modelo de backup diário usa timer systemd às 03:00 do fuso do host, com atraso aleatório até 15 minutos; wrapper exige projeto e arquivos Compose explícitos e usa flock. Na máquina local, o timer está instalado para o projeto `inter-pj-manager` de leitura de produção; cópia externa cifrada, alertas e política de retenção ainda são pendências. O serviço root executa scripts do checkout, cujo acesso deve ser restrito. Backup diário não satisfaz a meta financeira proposta de RPO 1 h.
- Teste axe-core cobre telas principais, erros de login e formulários abertos em desktop/celular e tema escuro; verificação automatizada não substitui avaliação com tecnologias assistivas e usuários.

- Compose usa loopback por padrão; CI E2E provisiona também a chave MFA. Origem HTTPS de produção é validada antes de conectar ou aplicar migrations. Inicialização ainda aplica migrations, que exigem autorização específica em produção.

- Relatório CSV financeiro MOCK neutraliza prefixos de fórmula em planilhas e limita cada exportação a 10.000 linhas; solicitação maior retorna 413. O limite não substitui controle de volume de dados no banco.
- Login de conta bloqueada usa resposta genérica e verificação Argon2 fictícia; NGINX limita a frequência por endereço. O bloqueio por conta ainda pode causar indisponibilidade sob ataque distribuído e requer monitoramento.

- O cliente HTTP do frontend aceita corpo vazio para respostas 204 e para chamadas explicitamente marcadas `allowEmpty`. Chamadas tipadas que precisam de JSON falham com 502 de resposta incompleta.
- O webhook usa certificado de cliente validado por CA, conta previamente vinculada e persistência antes do worker. Registro/configuração e homologação de callbacks no Inter ainda dependem de credenciais e ambiente autorizados.
- Pix Automático demonstrativo só existe em MOCK: estados de solicitação/consentimento são distintos; aceite, cancelamento, cobrança por competência e liquidação toleram reenvios idênticos sem duplicar auditoria ou efeitos. Competências únicas por empresa/autorização impedem duplicação e liquidação simulada gera apenas identificador sintético. Não coletar dados reais de pagadores nesta tela; integração oficial exige adapter próprio, consentimento e homologação.
- MFA TOTP está disponível somente em MOCK. O segredo é cifrado com XChaCha20-Poly1305 sob chave hex de 32 bytes em `MFA_ENCRYPTION_KEY_FILE`; manter a chave protegida e recuperável junto à estratégia de backup. Cadastro expira em 10 minutos, falhas bloqueiam por 15 minutos após cinco tentativas e o contador persistido impede reutilizar o mesmo passo TOTP.
- Aprovação de intenção Pix MOCK exige autor diferente, papel `APPROVER`/`ADMIN`, revisão, MFA e valor até R$ 100.000. Decisão, consumo TOTP, auditoria e chave idempotente são transacionais. O resultado `APROVADO_MOCK` declara `envio_bancario=false`; nenhum envio externo é conectado.
- A Fase 4 MOCK inclui intenções Pix e aprovação independente por TOTP, com autor distinto, papel `APPROVER`/`ADMIN`, revisão e limite de R$ 100.000. Cadastro MFA cifra segredo com XChaCha20-Poly1305, expira em 10 minutos, bloqueia após cinco falhas e persiste contador anti-replay. Decisão, auditoria, consumo TOTP e chave idempotente são transacionais; resultado é `APROVADO_MOCK` e `envio_bancario=false`.
- A Fase 5 possui itens financeiros empresariais em MOCK, estados auditados, totais exatos, exportação CSV e lembretes locais idempotentes consultáveis. Em 25/11/2026 o proprietário informou elegibilidade para Pix Automático; isso não foi verificado independentemente pelo Inter. O fluxo demonstrativo não movimenta dinheiro e não coleta CPF/CNPJ/conta. Contrato Inter divide `rec` (recorrência), `solicrec` (consentimento) e `cobr` (cobrança posterior ao aceite), além dos respectivos status/webhooks; IDs locais UUID não são IDs oficiais de 29 caracteres. Integração externa depende de scopes/credenciais e homologação e de definir armazenamento/consentimento dos dados pessoais exigidos.
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

O piloto de pagamento Pix por chave limita cada intenção a R$ 0,01. Preparação e aprovação local podem ficar disponíveis antes de habilitar transmissão. O autor e o aprovador são pessoas distintas da mesma empresa. A implantação local está em preparação, com a migration 0013 aplicada. Pendências da integração e verificação OAuth constam do handoff.

## Extrato detalhado — 25/11/2026

- O adapter do Inter agora preserva id, data de inclusão, operação, título, descrição e documento do extrato completo; o campo `detalhes` não é importado para evitar dados de terceiros.
- A API interna de extrato passou a devolver resumo exato do período e a tela mostra entradas, saídas, resultado líquido e metadados dos lançamentos.
- A migration `0012_extrato_detalhado.sql` é aditiva e foi validada no build Docker.
- Após autorização, a migration foi renumerada para `0012_extrato_detalhado.sql` e aplicada na produção local. A consulta Pix de leitura foi habilitada (`ENABLE_PIX_CHARGE_READS=true`); pagamentos e escritas bancárias continuam bloqueados.

- A outra estação terá banco e instalação novos e usará os arquivos da integração bancária existente por transferência privada. Usuários, histórico, sessões e UUIDs locais são novos; credenciais e chaves nunca entram no Git. A ativação de leitura de produção depende de HTTPS e autorização no destino.

- O trabalho ocorre no checkout local. SEFIN está fora do escopo.
- A identidade visual aplicada ao frontend usa a logo e a paleta da Brilhe Móveis; o nome exibido do produto é `PJ Manager`. A personalização pode ser refinada sem alterar o bloqueio de operações reais.
- Nenhuma operação financeira real foi autorizada. Produção deve continuar somente leitura até nova autorização específica.
- Direcionamento atualizado em 25/11/2026: implementar as fases do projeto antes da bateria final de testes e refinamentos. Testes de integração, PostgreSQL, contratos, Playwright e validação completa devem ficar agrupados ao encerramento; até então, não declarar aceite final.
- Nenhuma operação financeira real foi autorizada. Aprovação MOCK pode ser construída com controles apropriados; envio externo permanece bloqueado sem autorização específica e homologação.

## Hipóteses ainda não verificadas

- Habilitação dos scopes Pix Automático, credenciais e funcionamento da empresa no sandbox Inter continuam sem confirmação; a elegibilidade foi declarada pelo proprietário, não verificada independentemente.
- A conclusão externa da Fase 3 depende de homologar emissão, consulta de estados e callbacks no sandbox oficial.
