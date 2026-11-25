# Segurança e limites operacionais

A Fase 1 destina-se ao uso local com dados bancários sintéticos. O banco persiste dados de autenticação e auditoria reais da aplicação. O acesso ao socket do motor de contêineres e aos arquivos de provisionamento é administrativo.

- Senhas usam Argon2id com salt aleatório e parâmetros padrão da dependência fixada (19 MiB, duas iterações, paralelismo 1). A API limita a quatro verificações concorrentes e bloqueia novas tentativas após cinco falhas em uma janela de 15 minutos.
- Cookies HttpOnly, SameSite=Strict e Secure em produção. Os POST exigem origem exata e cabeçalho personalizado. CORS permanece fechado. O proxy define CSP, proteção contra framing, no-sniff e política de referência.
- Tokens de sessão completos só trafegam pelo cookie. O PostgreSQL mantém digest e expiração. A troca de perfil ou desativação de usuário tem efeito nas consultas seguintes.
- Logs estruturados incluem identificador de requisição, duração e status. Mantenha RUST_LOG em info para operação normal. Segredos bancários ficam fora do frontend e do repositório.
- `/metrics` não contém identificadores nem saldos, exige sessão ADMIN e deve permanecer acessível somente na rede operacional privada; colete externamente com controle de acesso e retenção.
- O backend e o proxy executam como usuários sem privilégios e com sistema de arquivos somente leitura. O PostgreSQL usa uma role de aplicação distinta do superusuário. Proteger o volume e os backups com criptografia de disco é responsabilidade da implantação.
- Permissões e consultas são escopadas à empresa autenticada. A auditoria exibe ações e horários, sem credenciais, senhas ou dados bancários completos.
- Para incidentes, isole o acesso, preserve a auditoria e revogue sessões do usuário por via administrativa no banco. Limite o acesso ao backup e ao cofre.

## Condições para dados reais

Antes de habilitar um adaptador externo, concluir testes de contrato, mTLS, isolamento entre contas e ambientes e política de rotação de certificados. Antes de qualquer escrita bancária, concluir MFA, separação de funções, aprovação vinculada ao valor e destinatário, limites e reconciliação após falhas. A Fase 1 recusa ambas as flags de escrita.

Definir política de retenção por classe de dado, base legal, responsáveis e processo para solicitações de titulares. Estabelecer expurgo controlado, segregação mais forte dos papéis PostgreSQL de migration e runtime, auditoria resistente à adulteração, cópias externas cifradas e alertas. Testes de carga, auditoria ASVS, validação de TLS do ambiente e recuperação de desastre integram a revisão de produção.


## Revisão local de 25/11/2026

A revisão estática confirmou guard de origem em POST, cookies protegidos, escopo por empresa, limite de concorrência Argon2, limitador de login no NGINX e exportação CSV com neutralização de fórmulas. As suítes locais são evidência de comportamento em MOCK, não substituem teste de invasão, auditoria ASVS nem homologação bancária.

Antes de operação com dados reais permanecem: MFA efetivo para autenticação e ações autorizadas fora de MOCK; segregação das permissões de migration e execução no PostgreSQL; auditoria resistente à adulteração; monitoramento e resposta para bloqueios de conta ou tentativas distribuídas; backups cifrados fora do host e recuperação ensaiada; e revisão das regras do proxy quando houver outro balanceador à frente, pois a limitação de login usa o endereço observado pelo NGINX. A configuração de proxy e endereços confiáveis deve ser validada no destino, sem aceitar cabeçalhos de origem do cliente indiscriminadamente.
