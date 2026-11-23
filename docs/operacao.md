# Operação local e implantação

## Configuração

| Variável | Uso e padrão |
|---|---|
| APP_ENV | development para HTTP local, production para cookie Secure e validação da origem HTTPS. Independente do ambiente bancário. |
| APP_HOST | Bind da API nativa. Padrão 127.0.0.1. O Compose usa 0.0.0.0 dentro da rede privada. |
| APP_PORT | Porta nativa e porta externa da interface no Compose, por padrão 8080. |
| APP_ORIGIN | Origem exata vista pelo navegador, por padrão http://127.0.0.1:8080. Ao mudar a porta pública, ajuste também a origem. |
| BANK_PROVIDER, INTER_ENV | inter e mock. Outros valores encerram a inicialização. |
| ENABLE_REAL_PAYMENTS, ENABLE_BANK_WRITES | Somente false é aceito nesta fase. |
| DATABASE_URL, DB_PASSWORD_FILE | Endereço PostgreSQL e arquivo de senha. O Compose define os valores internos. |
| ADMIN_EMAIL, ADMIN_PASSWORD_FILE | Identidade e arquivo da senha do bootstrap. No Compose, o arquivo é montado a partir de secrets/admin. |
| COMPANY_NAME | Nome da empresa inicial. Padrão Empresa de demonstração. |
| USER_EMAIL, USER_ROLE, USER_PASSWORD_FILE, COMPANY_ID | Usados exclusivamente pelo comando criar-usuario. |
| RUST_LOG | Filtro de tracing, padrão info. |
| APP_TIMEZONE | Valor previsto America/Fortaleza. O frontend da primeira fase utiliza esse fuso fixo. |
| INTER_CLIENT_ID, INTER_CLIENT_SECRET_FILE, INTER_CERT_PATH, INTER_KEY_PATH, INTER_ACCOUNT_NUMBER | Reservados à próxima fase. A aplicação MOCK não carrega material bancário. |
| ENGINE | Motor dos scripts locais, docker por padrão, ou podman. |
| TEST_DATABASE_URL | Banco descartável de testes de integração, nome inter_pj_test em 127.0.0.1. |
| E2E_ORIGIN, E2E_EMAIL, E2E_PASSWORD, CHROME_PATH | Configuração exclusiva dos testes de navegador. |

Sessões usam identificadores opacos aleatórios e digest no servidor. A versão atual dispensa segredo global de assinatura e chave de criptografia de payload, pois o recebimento de payloads bancários ainda está desabilitado.

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

O script usa pg_dump em formato custom, verifica o catálogo do dump com pg_restore e grava em backups sob umask 077. Dumps contêm dados pessoais e hashes de senha. Proteja-os com armazenamento criptografado, acesso restrito e política de retenção definida pelo proprietário. Defina RPO e RTO antes de operar dados reais.

Para exercitar a recuperação, escolha um nome novo em uma instância local isolada. O exemplo falha na criação se o banco de ensaio já existir. Execute a restauração somente após a criação bem-sucedida.

```sh
docker compose exec -T db createdb -U postgres -O manager ensaio_restauracao &&
docker compose exec -T db pg_restore -U postgres -d ensaio_restauracao --single-transaction < "$ARQUIVO_DUMP"
docker compose exec -T db psql -U postgres -d ensaio_restauracao -c 'SELECT count(*) FROM companies'
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
| Troca de ambiente impede a inicialização | A primeira fase exige inter/mock e as duas flags de escrita em false. |
| Porta ocupada | Configure APP_PORT e APP_ORIGIN juntos e recrie os serviços do projeto. |

Correlacione erros pelo X-Request-Id e pelas mensagens estruturadas do backend. Evite compartilhar logs, capturas de rede ou dumps com dados identificáveis.
