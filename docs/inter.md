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
