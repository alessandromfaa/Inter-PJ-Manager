# Validação da Fase 1

Ensaios locais de 23/11/2026. Ambiente bancário MOCK em todas as execuções.

| Verificação | Resultado observado |
|---|---|
| Rust estável 1.100 | cargo fmt --check, cargo clippy com todos os alvos e features e warnings como erros, cargo build --locked --release aprovados |
| Testes Rust | Três testes aprovados com testes-postgres. Cobrem fechamento em centavos, sessão, expiração, logout, tentativas concorrentes, limitação de login, RBAC, segregação de auditoria e rejeição das configurações de banco real |
| PostgreSQL | Duas migrations aplicadas em cluster nativo descartável. Compose com PostgreSQL 17 iniciou com as duas migrations e bootstrap repetível |
| Dependências | cargo audit e npm audit concluídos sem vulnerabilidades identificadas nas bases consultadas |
| Frontend | ESLint, TypeScript strict, build Vite e dois testes Vitest aprovados |
| Docker Compose | Build das imagens e inicialização em VM local. Bootstrap terminou com código 0. PostgreSQL, API e web saudáveis. API com UID 65532 e web com UID 101 |
| Playwright | Fluxo pela origem servida pelo NGINX aprovado. Incluiu validação do formulário, login, painel MOCK, alternância de tema, largura de 390 px, pesquisa no extrato, auditoria, logout e negativa após revogação |
| Backup e restauração | Dump gerado pelo script, restaurado com pg_restore --single-transaction em banco novo. Conferidas uma empresa e duas migrations com sucesso |
| Administração local | Provisionamento de usuário VIEWER pelo comando criar-usuario em contêiner da instância de teste |

O teste revelou um identificador reservado na migration inicial e uma legenda que excedia a largura do navegador em tela pequena. Ambos foram corrigidos antes do aceite. A atualização do SQLx e a seleção exclusiva de Core e driver PostgreSQL resolveram avisos da auditoria inicial sem exceções na lista de vulnerabilidades.

A validação externa de OAuth/mTLS e dos endpoints depende da Fase 2 e de credenciais de homologação. A validação de produção, pagamentos, MFA financeiro e webhooks pertence às fases correspondentes. A execução de CI em serviço hospedado ainda depende de repositório remoto.
