# Configuração do Inter Empresas

O painel inicial funciona com `INTER_ENV=mock`. A Fase 2 oferece sincronização de leitura por administrador.

## Credenciais e ambientes

1. O administrador entra no Internet Banking PJ pelo fluxo de autenticação do banco.
2. Em **Soluções para sua empresa**, seleciona **Nova Integração**, identifica a empresa e o ambiente e solicita a permissão de extrato e saldo para a primeira leitura.
3. Na lista de integrações, segue o fluxo de ativação e baixa certificado e chave. Registra a conta autorizada e a validade do certificado.
4. Guarda Client ID, Client Secret e chave em cofre. O adaptador deverá lê-los no backend em montagem somente leitura, acessível exclusivamente à identidade do serviço.
5. Na rotação, testa uma leitura com o novo material antes de revogar o anterior. Em caso de comprometimento, revoga a integração e revisa os acessos.

[Portal oficial](https://developers.inter.co/) · [Dúvidas frequentes](https://developers.inter.co/duvidas-frequentes)

## Sandbox e primeira consulta

Obtenha material próprio de homologação no portal de sandbox. O servidor de homologação é `https://cdpj-sandbox.partners.uatinter.co`. O servidor de produção é `https://cdpj.partners.bancointer.com.br`. Configure clientes e caches separados. O processo exige habilitação explícita para acessar ambientes externos. [Sandbox oficial](https://developers.inter.co/sandbox)

A especificação de OAuth documenta `POST /oauth/v2/token`, mTLS e formulário com `grant_type=client_credentials`, `client_id`, `client_secret` e `scope`. Para a primeira leitura solicite `extrato.read`. O token tem validade nominal de uma hora. O cliente utiliza o `expires_in` retornado e renova sob trava por integração, ambiente e escopos. Limite publicado de cinco solicitações por minuto. [OAuth](https://developers.inter.co/redocusaurus/swagger-token-yaml.yaml)

Comece por `GET /banking/v2/saldo`. O intervalo de `GET /banking/v2/extrato` é de até 90 dias. A variante `/banking/v2/extrato/completo` documenta paginação. A resposta de saldo distingue disponível, limite e bloqueios, com campos condicionais segundo a data. A especificação descreve números monetários e mostra exemplos fracionários. O cliente precisa preservar a representação decimal recebida antes da conversão para centavos. [Banking](https://developers.inter.co/redocusaurus/swagger-banking-yaml.yaml)

## Aceite da leitura externa

Validar com credenciais exclusivas de homologação a cadeia de confiança, o certificado do cliente, o vínculo com a conta, a paginação e as respostas de erro. Cobrir com servidor HTTP de teste renovação concorrente, timeout e 429. Conferir os limites por operação na matriz. Dados de saldo e extrato do sandbox são fictícios. O aceite documental e o teste operacional são registrados separadamente.

## Instalação de uma integração de leitura

Inicialize primeiro o Compose em MOCK pelo README. Identifique o UUID da empresa em GET /api/session após autenticação. Escolha um UUID permanente para a conta local. Configure as variáveis abaixo no arquivo privado .env. Os UUIDs e o número da conta devem corresponder à empresa autorizada.

| Variável | Exemplo de configuração |
|---|---|
| BANK_PROVIDER | inter |
| INTER_ENV | sandbox |
| ENABLE_BANK_READS | true |
| INTER_COMPANY_ID | UUID da empresa existente |
| INTER_ACCOUNT_ID | UUID local reservado à conta e ao ambiente |
| INTER_ACCOUNT_NUMBER | Número com dígito, somente algarismos e sem zeros iniciais |
| INTER_CREDENTIALS_ROOT | Caminho absoluto de um diretório privado fora do Git |
| ENABLE_BANK_WRITES e ENABLE_REAL_PAYMENTS | false |

Dentro de INTER_CREDENTIALS_ROOT/sandbox, crie client_id, client_secret, cert.pem e key.pem com o material dessa integração. O backend usa o UID/GID 65532 no Compose. Dê a essa identidade acesso de travessia ao diretório montado e leitura dos arquivos. Para segredo e chave, use 0600 com proprietário apropriado ou 0640 com grupo dedicado. Somente a subpasta do ambiente selecionado é montada em modo de leitura. A rede adicional permite saída HTTPS da API.

```sh
docker compose -f compose.yml -f compose.inter.yml up -d --build
```

Entre como ADMIN e use **Sincronizar leitura**. O login e a navegação consultam o estado local. Confira o período importado e a data da sincronização. O saldo é consultado no momento da importação e sua data de referência, quando fornecida, aparece separadamente. As páginas do extrato local têm 50 itens. A primeira importação abrange 30 dias; a retomada relê dois dias anteriores ao checkpoint. Após interrupções longas, repita a sincronização até atingir a data atual. Em falha, consulte o erro no painel e o evento em integration_sync_jobs.

Em produção, o proprietário deve configurar explicitamente INTER_ENV=production, APP_ENV=production, APP_ORIGIN HTTPS, diretório production com credenciais próprias e proxy HTTPS. Proteja volumes e backups com criptografia no ambiente de destino. O cliente carrega a identidade mTLS na inicialização; após rotação, recrie o serviço api. A implantação atual configura uma conta por instância. Não habilite múltiplas réplicas com as mesmas credenciais sem coordenação compartilhada das cotas OAuth e Banking.

A importação divide intervalos volumosos em períodos menores. Mais de 10 mil operações num único dia, mais de 100 mil por importação ou duração superior a quatro minutos encerram a tentativa sem avançar o checkpoint. O uso de scroll para esse perfil de volume permanece pendente. O painel externo utiliza os componentes efetivamente informados pelo Inter. Dados de cobranças, pagamentos e conciliação entram nas fases seguintes.

## Outra estação com banco novo

A instalação nova começa sem histórico, usuários, sessões, eventos ou dados financeiros da máquina anterior. O bootstrap gera nova empresa e administrador; os UUIDs de empresa e de conta desta instalação são próprios e não devem ser copiados da base anterior. A leitura da conta Inter após configuração usa as credenciais bancárias da integração autorizada, mas os registros locais permanecem independentes. Evite executar duas instâncias de leitura simultaneamente com as mesmas credenciais sem coordenar as cotas do Inter.

1. Obtenha a versão aprovada do repositório e inicie uma instalação isolada em MOCK seguindo o README. Defina o e-mail e o nome da empresa antes do bootstrap. Guarde em local seguro a senha gerada e a nova chave `secrets/mfa_key`; não copie senhas de banco, sessões nem a chave MFA da estação antiga para um banco novo.
2. Entre no painel local e obtenha `company_id` pela resposta autenticada de `/api/session`, sem compartilhar a resposta. Gere um novo UUID para `INTER_ACCOUNT_ID` e mantenha-o estável; use o identificador da empresa nova em `INTER_COMPANY_ID`. Nunca use CPF/CNPJ ou número da conta como UUID.
3. Transfira `client_id`, `client_secret`, `cert.pem` e `key.pem` da integração Inter autorizada por canal privado para `INTER_CREDENTIALS_ROOT/production`, fora do Git. Confirme validade, titularidade, permissões e legibilidade pelo UID 65532; não exponha conteúdo em terminal compartilhado, histórico ou logs.
4. Antes de habilitar a leitura real, instale e valide um proxy HTTPS confiável acessível somente conforme o escopo autorizado; configure `APP_ENV=production`, `APP_ORIGIN` com a origem HTTPS exata sem barra final, `INTER_ENV=production`, `ENABLE_BANK_READS=true`, UUIDs novos e número de conta autorizado. Mantenha `ENABLE_BANK_WRITES=false` e `ENABLE_REAL_PAYMENTS=false`. O proxy local de outra máquina e arquivos ignorados em `.tools` não são transferidos pelo Git.
5. Configure backup diário, cópia externa cifrada e alertas nessa estação; preserve separadamente a nova chave MFA. Faça testes em MOCK e valide a configuração sem expor serviços nem consultar a conta real até a autorização aplicável. Uma instalação real requer revisão do ambiente e autorização para ativação/publicação; a homologação bancária continua separada do teste MOCK.

A `main` deve conter o commit aprovado antes do clone; o diretório de trabalho não commitado da estação anterior não acompanha o Git.

## Verificar acesso de leitura com conta de produção

No Internet Banking PJ, abra **Integrar → Minhas Integrações** e confira a situação da integração. Quando estiver em **Novo**, abra **Ações** (três pontos) e escolha **Download chave e certificado**. Guarde o certificado `.crt`, a chave `.key` e as duas credenciais da mesma integração em armazenamento privado. O Inter disponibiliza o download uma única vez. Se a integração já estiver ativa, procure os arquivos com o responsável pelo cadastro ou no cofre da empresa. Se os arquivos foram perdidos, crie uma nova integração de leitura e use os quatro itens emitidos para ela. [Instruções de download do Inter](https://ajuda.inter.co/conta-digital-pessoa-juridica/como-faco-o-download-do-certificado-e-credenciais-da-minha-api)

Para uma conta PJ de produção, cadastre uma integração dedicada com a permissão de consulta de saldo e extrato (`extrato.read`). Salve os arquivos client_id, client_secret, cert.pem e key.pem em INTER_CREDENTIALS_ROOT/production. Configure o arquivo privado .env.producao-leitura com APP_ENV e INTER_ENV em production, BANK_PROVIDER=inter, ENABLE_BANK_READS=true, ambas as flags de escrita em false, o caminho absoluto INTER_CREDENTIALS_ROOT e INTER_ACCOUNT_NUMBER com dígito, sem zeros iniciais.

A partir da raiz do checkout preparado, execute o comando abaixo. O ambiente do perfil vale apenas dentro do subshell. O diagnóstico solicita um token por mTLS e consulta o saldo da conta. A saída informa o ambiente e o sucesso ou a classe de falha. O valor fica somente na memória do processo. Esse comando dispensa o PostgreSQL e o servidor web.

```sh
(
  set -a
  . ./.env.producao-leitura
  set +a
  ./backend/target/release/inter-pj-manager verificar-acesso-saldo
)
```

Para conferir o extrato do último dia encerrado em Fortaleza, substitua o argumento por `verificar-acesso-extrato`. O comando usa o mesmo cliente e as mesmas verificações da importação, processa a resposta em memória e informa a data e se houve lançamentos.

Uma consulta bem-sucedida em produção comprova acesso de leitura à conta configurada. Registre essa evidência separadamente da homologação em sandbox.

## Recebimentos Pix em modo de consulta

A opção `ENABLE_PIX_CHARGE_READS` controla a consulta individual de cobranças imediatas no adaptador externo. O valor inicial é `false`. Para um ambiente autorizado de leitura, a integração precisa conceder `cob.read` e o administrador precisa configurar `ENABLE_BANK_READS=true` e `ENABLE_PIX_CHARGE_READS=true` no perfil desse ambiente. As flags de escrita e pagamentos continuam em `false`. O backend pede o token específico ao consultar uma cobrança e mantém seu cache separado do token do extrato. A empresa é determinada pela sessão e a integração fornece o número da conta.

Para exercitar o fluxo local, use MOCK e consulte o identificador preenchido na tela **Cobranças Pix**. O valor mostrado é o original da cobrança. O relatório de recebimentos e a conciliação dependem da consulta aos Pix efetivamente recebidos.
