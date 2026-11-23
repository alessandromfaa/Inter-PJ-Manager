# Configuração do Inter Empresas

O painel inicial funciona com `INTER_ENV=mock`. A integração externa começa na Fase 2.

## Credenciais e ambientes

1. O administrador entra no Internet Banking PJ pelo fluxo de autenticação do banco.
2. Em **Soluções para sua empresa**, seleciona **Nova Integração**, identifica a empresa e o ambiente e solicita a permissão de extrato e saldo para a primeira leitura.
3. Na lista de integrações, segue o fluxo de ativação e baixa certificado e chave. Registra a conta autorizada e a validade do certificado.
4. Guarda Client ID, Client Secret e chave em cofre. O adaptador deverá lê-los no backend em montagem somente leitura, acessível exclusivamente à identidade do serviço.
5. Na rotação, testa uma leitura com o novo material antes de revogar o anterior. Em caso de comprometimento, revoga a integração e revisa os acessos.

[Portal oficial](https://developers.inter.co/) · [Dúvidas frequentes](https://developers.inter.co/duvidas-frequentes)

## Sandbox e primeira consulta

Obtenha material próprio de homologação no portal de sandbox. O servidor de homologação é `https://cdpj-sandbox.partners.uatinter.co`. O servidor de produção é `https://cdpj.partners.bancointer.com.br`. Configure clientes e caches separados. A seleção de ambiente na versão atual falha com diagnóstico se o valor for diferente de `mock`. [Sandbox oficial](https://developers.inter.co/sandbox)

A especificação de OAuth documenta `POST /oauth/v2/token`, mTLS e formulário com `grant_type=client_credentials`, `client_id`, `client_secret` e `scope`. Para a primeira leitura solicite `extrato.read`. O token tem validade nominal de uma hora. A implementação futura deverá usar o `expires_in` retornado e renovar sob trava por integração, ambiente e escopos. Limite publicado de cinco solicitações por minuto. [OAuth](https://developers.inter.co/redocusaurus/swagger-token-yaml.yaml)

Comece por `GET /banking/v2/saldo`. O intervalo de `GET /banking/v2/extrato` é de até 90 dias. A variante `/banking/v2/extrato/completo` documenta paginação. A resposta de saldo distingue disponível, limite e bloqueios, com campos condicionais segundo a data. A especificação descreve números monetários e mostra exemplos fracionários. O cliente precisa preservar a representação decimal recebida antes da conversão para centavos. [Banking](https://developers.inter.co/redocusaurus/swagger-banking-yaml.yaml)

## Aceite da leitura externa

Validar com credenciais exclusivas de homologação a cadeia de confiança, o certificado do cliente, o vínculo com a conta, a paginação e as respostas de erro. Cobrir com servidor HTTP de teste renovação concorrente, timeout e 429. Conferir os limites por operação na matriz. Dados de saldo e extrato do sandbox são fictícios. O aceite documental e o teste operacional são registrados separadamente.
