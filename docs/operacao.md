# Operação local e implantação

## Configuração

| Variável | Uso e padrão |
|---|---|
| APP_ENV | development para HTTP local, production para cookie Secure e validação da origem HTTPS. Independente do ambiente bancário. |
| APP_HOST | Bind da API nativa. Padrão 127.0.0.1. O Compose usa 0.0.0.0 dentro da rede privada. |
| APP_PORT | Porta nativa e porta externa da interface no Compose, por padrão 8080. |
| APP_ORIGIN | Origem exata vista pelo navegador, por padrão http://127.0.0.1:8080. Ao mudar a porta pública, ajuste também a origem. |
| APP_BIND_ADDRESS | Interface IPv4 em que o NGINX publica a porta Compose; para desenvolvimento local, use `127.0.0.1`. |
| BANK_PROVIDER, INTER_ENV | inter e mock por padrão. sandbox e production exigem ENABLE_BANK_READS=true. |
| ENABLE_REAL_PAYMENTS, ENABLE_BANK_WRITES | Somente false é aceito nesta fase. |
| DATABASE_URL, DB_PASSWORD_FILE | Endereço PostgreSQL e arquivo de senha. O Compose define os valores internos. |
| ADMIN_EMAIL, ADMIN_PASSWORD_FILE | Identidade e arquivo da senha do bootstrap. No Compose, o arquivo é montado a partir de secrets/admin. |
| MFA_ENCRYPTION_KEY_FILE | Arquivo com chave hexadecimal de 32 bytes para cifrar segredos TOTP. O Compose usa `secrets/mfa_key`; preserve-a em backup protegido separado do banco. |
| COMPANY_NAME | Nome da empresa inicial. Padrão Empresa de demonstração. |
| USER_EMAIL, USER_ROLE, USER_PASSWORD_FILE, COMPANY_ID | Usados exclusivamente pelo comando criar-usuario. |
| RUST_LOG | Filtro de tracing, padrão info. |
| APP_TIMEZONE | Valor previsto America/Fortaleza. O frontend da primeira fase utiliza esse fuso fixo. |
| INTER_CREDENTIALS_ROOT, INTER_COMPANY_ID, INTER_ACCOUNT_ID, INTER_ACCOUNT_NUMBER | Diretório de arquivos por ambiente, UUID da empresa, UUID local estável da conta e número da conta. Ver docs/inter.md. |
| ENGINE | Motor dos scripts locais, docker por padrão, ou podman. |
| TEST_DATABASE_URL | Banco descartável de testes de integração, nome inter_pj_test em 127.0.0.1. |
| E2E_ORIGIN, E2E_EMAIL, E2E_PASSWORD, CHROME_PATH | Configuração exclusiva dos testes de navegador. |

Sessões usam identificadores opacos aleatórios e digest no servidor. A chave MFA cifra segredos TOTP com XChaCha20-Poly1305. Se a chave for perdida, os usuários precisarão recuperar o acesso por procedimento administrativo antes de reconfigurar MFA; não substitua a chave enquanto existirem credenciais cifradas com a anterior.

## Provisionar usuário

Um operador com acesso administrativo ao host prepara o arquivo de senha em secrets, protegido pelo diretório 0700 e legível pelo UID do contêiner. Obtenha COMPANY_ID no campo company_id de /api/session em sessão autorizada. Na raiz, exporte COMPANY_ID, USER_EMAIL e USER_ROLE com o perfil desejado.

```sh
docker compose run --rm --no-deps \
  -e COMPANY_ID -e USER_EMAIL -e USER_ROLE \
  -e USER_PASSWORD_FILE=/run/user_password \
  -v "$PWD/secrets/novo_usuario:/run/user_password:ro" api criar-usuario
```

O comando transaciona a criação do usuário, seu perfil e o evento de provisionamento. E-mails são únicos. O vínculo da empresa precisa existir.

## Backup e restauração

```sh
./infrastructure/scripts/backup.sh
```

O script usa pg_dump em formato custom, exige arquivo não vazio, verifica o catálogo com pg_restore e confirma que a tabela central `companies` está incluída antes de publicar o arquivo final. Grava dump e manifesto em `backups` sob umask 077. O manifesto registra horário UTC, tamanho, SHA-256, validação do catálogo e indica que a restauração não foi ensaiada. O hash permite conferir integridade do arquivo, mas não prova que ele pode ser restaurado. Dumps contêm dados pessoais e hashes de senha. Proteja ambos com armazenamento criptografado e acesso restrito. Metas propostas pelo proprietário para planejamento: RTO de 4 horas e RPO de 1 hora para dados financeiros; RTO de 8 horas e RPO de 24 horas para aplicação/configuração. São objetivos ainda não comprovados por ensaios cronometrados nem por uma política de backup compatível. `/metrics` publica somente prontidão e indicador do modo MOCK, sem valores, dados pessoais ou rótulos empresariais.

Antes de habilitar operação contínua, atribua responsáveis e canais para alertas de indisponibilidade, falha de backup, espaço em disco e expiração de certificados. Monitore a idade do último backup e valide periodicamente restaurações em banco isolado. As metas de RTO/RPO acima orientam o planejamento, mas precisam ser aprovadas operacionalmente, medidas em ensaio e sustentadas por frequência de backup/recuperação compatível. Até definir obrigações legais, prazo de retenção e capacidade cifrada, mantenha os backups sob controle manual e não automatize descarte; o script atual não exclui nem rotaciona arquivos. Registre data, operador, resultado do catálogo e resultado do ensaio de restauração em controle operacional sem dados pessoais. Não inclua métricas com dados bancários, identificadores pessoais ou nomes de empresas.

Para buscar o RPO financeiro proposto de 1 hora, agende `./infrastructure/scripts/backup.sh` a cada hora em um host confiável, com `flock` para impedir execuções simultâneas. Use o mesmo diretório de backups protegido, configure alerta se a execução falhar ou se o backup válido mais recente ultrapassar 60 minutos e acompanhe espaço livre. O agendador deve carregar o ambiente/seleção Compose correto, registrar saída sem segredos e não apagar backups. Essa frequência é uma recomendação de desenho, não está instalada pelo repositório nem garante RPO por si só. Até existir armazenamento cifrado fora do host e política aprovada de retenção, mantenha operação controlada e não considere a meta cumprida. Para configuração/aplicação, preserve também os artefatos versionados e inventário de configuração; o backup PostgreSQL isolado não cobre esses componentes.

Para exercitar a recuperação, confira o SHA-256 do dump contra o manifesto e escolha um nome novo em uma instância local isolada. O `createdb` falha se o banco já existir. Execute a restauração somente após a criação bem-sucedida, confira migrations e tabelas e compare totais não sensíveis. Não aponte o `pg_restore` para `manager` nem para qualquer banco com dados que devam ser preservados.

```sh
docker compose exec -T db createdb -U postgres -O manager ensaio_restauracao &&
docker compose exec -T db pg_restore -U postgres -d ensaio_restauracao --single-transaction < "$ARQUIVO_DUMP" &&
docker compose exec -T db psql -U postgres -d ensaio_restauracao -c 'SELECT count(*) AS empresas FROM companies' &&
docker compose exec -T db psql -U postgres -d ensaio_restauracao -c 'SELECT count(*) AS migrations FROM _sqlx_migrations'
```

Confirme as versões em _sqlx_migrations, totais e vínculos por empresa. Revogue todas as sessões recuperadas antes de permitir acesso ao sistema restaurado. Guarde o material de integração em cofre separado. Uma recuperação em produção exige autorização, plano de corte e verificação do estado das ordens junto ao banco.

## Atualização e HTTPS

Faça backup e ensaie a restauração. Aplique a nova versão em ambiente isolado e verifique migrations com `./infrastructure/scripts/migrar.sh`. Inicie a pilha, aguarde /ready e teste login, dashboard e auditoria. Para voltar a uma versão com esquema incompatível, prepare outra instância a partir do backup validado e planeje a troca.

A publicação em produção depende da Fase 6. Para ensaio de HTTPS, use proxy reverso com certificado confiável, APP_ENV=production e APP_ORIGIN correspondente ao domínio HTTPS exato. Mantenha API e PostgreSQL privados. Configure TLS, HSTS, monitoramento e armazenamento cifrado no ambiente de destino. Verifique os atributos do cookie no navegador. Exposição de um serviço depende de autorização específica.

## Diagnóstico

| Sintoma | Verificação |
|---|---|
| Bootstrap encerra com erro | Confira as permissões e o comprimento dos arquivos de segredo, a saúde do PostgreSQL e o log do serviço bootstrap. |
| Login recebe 403 | Compare protocolo, nome e porta do navegador com APP_ORIGIN. Utilize a URL do README. |
| Login recebe 429 | Espere a janela de bloqueio de 15 minutos após cinco falhas por conta ou o intervalo do limitador no proxy. |
| Usuário sem acesso à auditoria | Confira ADMIN ou AUDITOR na sessão. Os papéis são lidos no servidor. |
| /ready falha | Confira conexões, espaço em disco e migrations. Use docker compose ps -a e logs dos serviços locais. |
| Alteração da senha em secrets/admin sem efeito | O bootstrap cria o primeiro administrador somente em banco vazio. Alterar o arquivo após a criação não altera o hash persistido. |
| Troca de ambiente impede a inicialização | Confira o vínculo, ambiente, permissões dos arquivos e as duas flags de escrita em false. |
| Porta ocupada | Configure APP_PORT e APP_ORIGIN juntos e recrie os serviços do projeto. |

Correlacione erros pelo X-Request-Id e pelas mensagens estruturadas do backend. Evite compartilhar logs, capturas de rede ou dumps com dados identificáveis.
