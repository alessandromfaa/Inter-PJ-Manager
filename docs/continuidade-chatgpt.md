# Inter PJ Manager — continuidade para o ChatGPT Web

Atualizado em 23/11/2026. O proprietário quer uma plataforma financeira empresarial em pt-BR, inicialmente para uma empresa, com separação de usuários, contas e empresas. As Fases 1 e 2 foram implementadas e validadas localmente. O próximo marco externo é a homologação da leitura bancária. O trabalho ocorre no repositório local inter, branch main. O ambiente SEFIN está fora do escopo autorizado. Publicação, mudanças em produção e operações financeiras reais dependem de autorização explícita.

## Fundação entregue

Rust estável, Axum, Tokio e SQLx com PostgreSQL no backend. React com TypeScript strict, Vite, Tailwind, Button baseado em shadcn/ui, TanStack Query, React Hook Form, Zod e Recharts no frontend. Docker e Podman Compose, NGINX, health e readiness, migrações, inicialização e backup locais. Interface e instruções em português, dinheiro em centavos e apresentação de horários em America/Fortaleza.

O modo padrão MOCK oferece saldo, entradas, saídas, evolução, agenda, extrato pesquisável e auditoria. As sessões reais da aplicação usam Argon2id, cookie HttpOnly e SameSite, expiração, revogação, verificação de origem e limitação de tentativas. Os perfis são ADMIN, VIEWER, OPERATOR, APPROVER e AUDITOR. A empresa é resolvida pelo backend a partir da sessão. As escritas bancárias permanecem bloqueadas. A definição da licença do código próprio depende do proprietário.

| Commit local da primeira fase | Conteúdo |
|---|---|
| d09f890 | Domínio, persistência, autenticação e autorização |
| 71babc3 | Painel, extrato e auditoria em React |
| 4275fb8 | Compose, execução nativa, CI e backups |
| 25fe097 | Documentação, contratos e aceite |

A Fase 1 passou por build release, formatação, Clippy, três testes Rust, ESLint, dois testes Vitest, build do frontend e Playwright. O navegador validou login, navegação, extrato, tema, largura de 390 px e logout. Docker e Podman subiram em VM descartável local. O ensaio de restauração confirmou uma empresa e as duas migrações originais. Os detalhes estão em docs/validacao-fase-1.md. Para iniciar em MOCK no checkout preparado, execute ./infrastructure/scripts/mock-local.sh e abra http://127.0.0.1:5173. Para Compose, use ./infrastructure/scripts/iniciar.sh e a porta 8080. As senhas geradas ficam exclusivamente nos arquivos privados locais.

## Leitura bancária implementada no checkout

O adaptador Inter usa Reqwest e Rustls. O proprietário seleciona sandbox ou production e habilita explicitamente ENABLE_BANK_READS. Produção exige APP_ENV=production e origem HTTPS. Um vínculo relaciona empresa, identificador local da conta, ambiente e conta corrente. Os arquivos privados ficam separados por ambiente. O cliente usa hosts fixos, certificado e chave de cliente, validação TLS do servidor, cache de token por integração, renovação sob mutex, limitação de frequência, timeout, tratamento de 429 e renovação após 401.

| Uso | Contrato oficial | Permissão ou limite |
|---|---|---|
| OAuth | POST /oauth/v2/token | client_credentials, cinco solicitações por minuto |
| Saldo e componentes | GET /banking/v2/saldo | extrato.read, dez por minuto |
| Extrato enriquecido | GET /banking/v2/extrato/completo | extrato.read, dez por minuto, paginação por pagina e tamanhoPagina |

Referências primárias: [OAuth](https://developers.inter.co/redocusaurus/swagger-token-yaml.yaml) e [Banking](https://developers.inter.co/redocusaurus/swagger-banking-yaml.yaml).

A primeira sincronização consulta 30 dias. As seguintes retomam da última data importada com dois dias de sobreposição e avançam até 90 dias por sincronização. A paginação tradicional divide intervalos acima de 10 mil itens. Um único dia acima desse limite gera erro explícito. As páginas são verificadas antes do avanço do checkpoint. A gravação agrupa saldo, transações e checkpoint numa transação PostgreSQL. O identificador bancário idTransacao evita duplicatas por conta. A sincronização concorrente da mesma conta é bloqueada. Falhas preservam a última leitura concluída. O painel identifica o ambiente, apresenta saldo, bloqueios, limite, validade do certificado e data da leitura. O extrato local tem filtros de período e paginação.

Pontos de entrada: backend/src/infrastructure/banking/inter/{client,contract,store,mod,tests}.rs, backend/migrations/0003_leituras_inter.sql, backend/src/api/mod.rs, frontend/src/pages/banking.tsx e compose.inter.yml. As rotas novas são GET /api/ambiente, POST /api/sincronizar para ADMIN e GET /api/extrato para consulta autorizada. GET /api/dashboard preserva o contrato MOCK e oferece uma resposta própria para leitura externa.

## Próximas fases e regras de continuidade

A Fase 3 trata Pix recebimento, Cobrança V3, clientes, callbacks autenticados, persistência e conciliação. A Fase 4 trata pagamentos, MFA, limites, revisão e aprovação por pessoas distintas. A Fase 5 cobre Pix Automático condicionado à elegibilidade, contas internas, relatórios, jobs e alertas. A Fase 6 cobre hardening, testes de falhas, recuperação e aceite de produção. Consultar docs/inter-api-matrix.md antes de integrar cada família. Conferir versões, escopos, limites, sandbox e contratos no Inter. Distinguir solicitações aceitas, estados indeterminados e liquidação efetiva.

No ChatGPT Web, use este arquivo como contexto. O código-fonte e as alterações de trabalho residem no checkout local. Para revisar ou modificar código, use o repositório correspondente ou forneça os arquivos relevantes. Credenciais, certificados, chaves, tokens, dados bancários e backups devem permanecer sob controle local.

## Operação e próximos passos para a pessoa que assumir

1. Para conferir o projeto no computador autorizado, leia README.md, AGENTS.md, docs/architecture.md e docs/implementation-plan.md. As instruções de instalação, configuração, atualização, backup e restauração estão em docs/operacao.md. A API interna está em docs/api-interna.md.
2. Para homologar a leitura, siga docs/inter.md. Mantenha o ambiente em MOCK até receber credenciais de sandbox do proprietário. O material necessário é Client ID, Client Secret, certificado, chave privada e conta autorizada. O proprietário os configura localmente. A nova conversa precisa apenas dos resultados sanitizados.
3. A configuração explícita usa INTER_ENV, ENABLE_BANK_READS, INTER_COMPANY_ID, INTER_ACCOUNT_ID, INTER_ACCOUNT_NUMBER e INTER_CREDENTIALS_ROOT. A subpasta sandbox ou production contém os quatro arquivos client_id, client_secret, cert.pem e key.pem. O perfil Compose de leitura combina compose.yml e compose.inter.yml. O administrador inicia a consulta pelo botão **Sincronizar leitura**.
4. No sandbox, confira a correspondência entre conta, saldo, sinais de débito e crédito, datas e páginas. Sincronize novamente e verifique o histórico local e a ausência de duplicatas. Guarde a data e o resultado do ensaio sem expor identificadores bancários.
5. Para evolução funcional, a próxima entrega é a Fase 3. Comece por uma cobrança Pix imediata em MOCK com contrato verificado, depois trate consultas e estados. Em Cobranças V3 acompanhe codigoSolicitacao separadamente da emissão e da liquidação. Para webhooks, valide a autenticação documentada pelo Inter, persista antes de processar e deduplique por conta. A elegibilidade de Pix Automático é uma etapa posterior.

**Texto para iniciar a conversa no ChatGPT Web**

> Use este documento como contexto do Inter PJ Manager. Responda em português brasileiro. Comece pelo estado de validação ao final do arquivo. Preserve a restrição de não alterar o ambiente SEFIN, o padrão MOCK, a segregação por empresa e o bloqueio de escritas bancárias. Para homologação, peça somente resultados sanitizados. Para implementar alterações, verifique o estado do repositório correspondente. Retome a partir das pendências registradas.

## Estado de validação na entrega da Fase 2

O commit af788aa acrescentou o adaptador, a terceira migração, as rotas de leitura, a interface e o manifesto opcional de integração. A implementação foi validada em MOCK e com serviços de teste restritos a loopback. Nenhuma operação financeira real foi executada. A auditoria inicial identificou um aviso de manutenção na biblioteca chrono. A versão final usa a biblioteca time e o suporte correspondente do driver PostgreSQL.

| Verificação executada | Evidência |
|---|---|
| Backend | Rustfmt, Clippy com todos os alvos e features e warnings como erros, build release com lockfile, nove testes Rust aprovados |
| Contratos, cache e falhas | OAuth concorrente com reutilização, renovação por expires_in, HTTP 401, 429, 503, timeout, paginação, IDs repetidos e conversão exata em centavos |
| mTLS local | Certificados temporários gerados no teste. Conexão recusada sem certificado de cliente ou com emissor de servidor desconhecido. Identidade válida e CA confiável permitiram ler saldo sintético |
| PostgreSQL | Teste de segregação por empresa, vínculo de conta, concorrência, reimportação idempotente e preservação do checkpoint após falha na segunda página |
| Frontend | ESLint, TypeScript strict e build Vite aprovados. Dois testes Vitest e dois fluxos Playwright aprovados. O fluxo de leitura usa respostas SANDBOX simuladas no navegador |
| Dependências | cargo audit e npm audit concluídos sem vulnerabilidades reportadas na execução final |
| Contêineres | Docker Compose na VM local reutilizada. API, web e PostgreSQL saudáveis. Banco com versões de migração 1, 2 e 3 aplicadas com sucesso. Inicialização adicional do perfil SANDBOX com credenciais sintéticas, sessão local e painel vazio antes da primeira importação |
| Homologação do banco | Pendente de credenciais exclusivas de homologação e validação pelos servidores oficiais do Inter |

**Ponto de retomada.** O operador pode iniciar o produto localmente em MOCK pelo README. Com as credenciais de sandbox configuradas pelo proprietário, o primeiro exercício externo é sincronizar saldo e extrato, repetir a consulta e conferir valores, datas e deduplicação. O desenvolvimento da Fase 3 pode começar com mocks e contratos oficiais enquanto aguarda essa homologação. Os testes locais, os contêineres e a documentação foram preparados dentro do escopo do projeto, sem implantação na SEFIN.
