# Matriz das APIs Inter Empresas

Consulta documental em 23/11/2026. OAuth, saldo e extrato completo possuem adaptador implementado e testes locais. Em produção, o diagnóstico obteve token, saldo e extrato completo para um intervalo vazio. O caso observado devolveu zero páginas e registros com `ultimaPagina=false`. Em 24/11/2026, uma segunda consulta de extrato em produção validou uma página com lançamentos. A navegação entre várias páginas foi testada localmente. Homologação em sandbox pendente. As demais operações são inventário para fases futuras. As colunas P e S indicam limites publicados em chamadas por minuto para produção e sandbox. O traço indica informação a confirmar no contrato específico.

| Família | Entrega prevista |
|---|---|
| OAuth e Banking de leitura | Fase 2 |
| Pix recebimentos, Cobrança V3 e respectivos callbacks | Fase 3 |
| Ordens de pagamento Banking | Fase 4 |
| Pix Automático | Fase 5, elegibilidade informada pelo proprietário em 25/11/2026; fluxo local MOCK em desenvolvimento |
| Forum | Opcional, sem prioridade bancária |

## OAuth

[Especificação oficial](https://developers.inter.co/redocusaurus/swagger-token-yaml.yaml)

| Operação e versão | Escopo | P | S | Respostas HTTP |
|---|---|---:|---:|---|
| `POST /oauth/v2/token` | Escopos solicitados no formulário | 5 | 5 | 200, 400, 403, 404, 503 |

## Banking

[Especificação oficial](https://developers.inter.co/redocusaurus/swagger-banking-yaml.yaml)

| Operação e versão | Escopo | P | S | Respostas HTTP |
|---|---|---:|---:|---|
| `GET /banking/v2/extrato` | extrato.read | 10 | 10 | 200, 400, 403, 404, 503 |
| `GET /banking/v2/extrato/exportar` | extrato.read | 10 | 10 | 200, 400, 403, 404, 503 |
| `GET /banking/v2/extrato/completo` | extrato.read | 10 | 10 | 200, 400, 403, 404, 503 |
| `GET /banking/v2/saldo` | extrato.read | 10 | 10 | 200, 400, 403, 404, 503 |

O adapter implementado persiste no ledger local o id, a data de inclusão, a operação, o título, a descrição e o número do documento do extrato completo. O campo `detalhes`, que pode conter dados de terceiros, não é importado.
| `POST /banking/v2/pagamento` | pagamento-boleto.write | 120 | 10 | 200, 400, 401, 403, 404, 406, 409 |
| `GET /banking/v2/pagamento` | pagamento-boleto.read | 120 | 10 | 200, 400, 500 |
| `POST /banking/v2/pagamento/darf` | pagamento-darf.write | 10 | 10 | 200, 400, 401, 403, 404, 500 |
| `GET /banking/v2/pagamento/darf` | pagamento-boleto.read | 120 | 10 | 200, 400, 401, 403, 404, 500 |
| `POST /banking/v2/pagamento/lote` | pagamento-lote.write | 10 | 10 | 202, 400, 401, 403, 404, 500 |
| `GET /banking/v2/pagamento/lote/{idLote}` | pagamento-lote.read | 20 | 10 | 200, 404 |
| `DELETE /banking/v2/pagamento/{codigoTransacao}` | pagamento-boleto.write | 20 | 10 | 204, 404, 422, 500 |
| `POST /banking/v2/pix` | pagamento-pix.write | 60 | 10 | 200, 400, 401, 403, 404, 422, 500 |
| `GET /banking/v2/pix/{codigoSolicitacao}` | pagamento-pix.read | 20 | 10 | 200, 400, 401, 403, 404, 422, 500 |

| `PUT /banking/v2/webhooks/{tipoWebhook}` | webhook-banking.write | 10 | 10 | 204, 400, 503 |
| `GET /banking/v2/webhooks/{tipoWebhook}` | webhook-banking.read | 10 | 10 | 200, 404, 500 |
| `DELETE /banking/v2/webhooks/{tipoWebhook}` | webhook-banking.write | 10 | 10 | 204, 404, 500 |
| `GET /banking/v2/webhooks/{tipoWebhook}/callbacks` | webhook-banking.read | 10 | 10 | 200, 400, 500 |
| `POST /banking/v2/webhooks/{tipoWebhook}/callbacks/retry` | webhook-banking.write | 5 | — | 200, 400, 403, 404, 500 |

### Pagamentos Pix por chave

O piloto serializa R$ 0,01 como número decimal exato. Cada intenção tem um UUID persistido antes do envio, usado no cabeçalho `x-id-idempotente`. A aplicação faz uma tentativa de inclusão com `pagamento-pix.write`. Interpreta `APROVACAO`, `PROCESSADO` e `AGENDADO` como estados distintos. A consulta por `codigoSolicitacao` usa `pagamento-pix.read`, valida o identificador e conserva o status recebido. Somente `PAGO` indica pagamento integral na tela.

Se a resposta se perder, a intenção fica bloqueada para novo envio. Sem o código de solicitação, a conferência depende do extrato e do Internet Banking. A duração da deduplicação do Inter e a recuperação operacional desse caso seguem para homologação.

## Pix recebimentos


Revisão de 24/11/2026 para a consulta individual de cobrança imediata. O OpenAPI da API Pix V2 confirma `GET /cob/{txid}` sob a base `/pix/v2` nos dois ambientes, token OAuth/mTLS com `cob.read` e seleção da conta por `x-conta-corrente`. O identificador aceita 26 a 35 caracteres alfanuméricos ASCII. A consulta retorna um objeto `CobCompleta`, sem paginação. O recorte inicial utiliza identificador, revisão, situação e valor original. O valor nominal e o estado da cobrança têm finalidade de consulta. A conciliação de recebimentos requer verificar os Pix efetivamente recebidos. Cotas e códigos HTTP constam na tabela abaixo. Adaptador e painel implementados, com cache por escopo e cadência conservadora de até 10 consultas por minuto por instância. Testes com servidor local cobrem divergência de txid, escopo, valor, estado e 404. Validação externa específica desta operação pendente.

[Especificação oficial](https://developers.inter.co/redocusaurus/swagger-api-pix-yaml.yaml)

Para a emissão imediata, o contrato revisado em 24/11/2026 especifica PUT com txid escolhido pelo recebedor, OAuth/mTLS com `cob.write`, corpo `CobSolicitada` (calendário, valor e chave) e retorno 201 com `CobGerada`. Trata-se de uma operação individual, sem paginação. A aplicação implementa emissão persistente em MOCK e um adaptador externo restrito ao SANDBOX, protegido por `ENABLE_SANDBOX_COLLECTIONS=true` e `ENABLE_BANK_WRITES=true`. A validação externa no sandbox continua pendente. Produção não expõe a rota de emissão. [Contrato Pix V2](https://developers.inter.co/redocusaurus/swagger-api-pix-yaml.yaml) · [Padrão de iniciação do Banco Central](https://www.bcb.gov.br/content/estabilidadefinanceira/pix/Regulamento_Pix/II_ManualdePadroesparaIniciacaodoPix.pdf)

| Operação e versão | Escopo | P | S | Respostas HTTP |
|---|---|---:|---:|---|
| `PUT /pix/v2/cob/{txid}` | cob.write | 120 | 10 | 201, 400, 403, 404, 503 |
| `PATCH /pix/v2/cob/{txid}` | cob.write | 120 | 10 | 200, 400, 403, 404, 503 |
| `GET /pix/v2/cob/{txid}` | cob.read | 120 | 10 | 200, 403, 404, 503 |
| `POST /pix/v2/cob` | cob.write | 120 | 10 | 201, 400, 403, 503 |
| `GET /pix/v2/cob` | cob.read | 120 | 10 | 200, 403, 503 |
| `POST /pix/v2/cob/pagar/{txid}` | pix.write | — | 10 | 201, 400, 403, 503 |
| `POST /pix/v2/sandbox/cob/pagamento` | pix.write | — | 10 | 200, 400, 404, 500 |
| `PUT /pix/v2/cobv/{txid}` | cobv.write | 120 | 10 | 201, 400, 403, 404, 503 |
| `PATCH /pix/v2/cobv/{txid}` | cobv.write | 120 | 10 | 200, 400, 403, 404, 503 |
| `GET /pix/v2/cobv/{txid}` | cobv.read | 120 | 10 | 200, 403, 404, 503 |
| `GET /pix/v2/cobv` | cobv.read | 120 | 10 | 200, 403, 503 |
| `POST /pix/v2/cobv/pagar/{txid}` | pix.write | — | 10 | 201, 400, 403, 503 |
| Caminho pendente de confirmação¹ | pix.write | — | 10 | 200, 400, 404, 500 |
| `PUT /pix/v2/lotecobv/{id}` | lotecobv.write | 120 | 10 | 202, 400, 403, 404, 503 |
| `PATCH /pix/v2/lotecobv/{id}` | lotecobv.write | 120 | 10 | 202, 400, 403, 404, 503 |
| `GET /pix/v2/lotecobv/{id}` | lotecobv.read | 120 | 10 | 200, 403, 404, 503 |
| `GET /pix/v2/lotecobv/{id}/sumario` | lotecobv.read | 120 | 10 | 200, 403, 404, 503 |
| `GET /pix/v2/lotecobv/{id}/situacao/{situacao}` | lotecobv.read | 120 | 10 | 200, 403, 404, 503 |
| `GET /pix/v2/lotecobv` | lotecobv.read | 120 | 10 | 200, 403, 503 |
| `POST /pix/v2/loc` | payloadlocation.write | 120 | 10 | 201, 400, 403, 503 |
| `GET /pix/v2/loc` | payloadlocation.read | 120 | 10 | 200, 400, 403, 503 |
| `GET /pix/v2/loc/{id}` | payloadlocation.read | 120 | 10 | 200, 403, 404, 503 |
| `DELETE /pix/v2/loc/{id}/txid` | payloadlocation.write | 120 | 10 | 200, 403, 404, 503 |
| `GET /pix/v2/pix/{e2eId}` | pix.read | 120 | 10 | 200, 403, 404, 503 |
| `GET /pix/v2/pix` | pix.read | 120 | 10 | 200, 403, 503 |
| `PUT /pix/v2/pix/{e2eId}/devolucao/{id}` | pix.write | 120 | 10 | 201, 403, 404, 503 |
| `GET /pix/v2/pix/{e2eId}/devolucao/{id}` | pix.read | 120 | 10 | 200, 403, 404, 503 |
| `PUT /pix/v2/webhook/{chave}` | webhook.write | 10 | 10 | 204, 403, 404, 503 |
| `GET /pix/v2/webhook/{chave}` | webhook.read | 10 | 10 | 200, 403, 404, 503 |
| `DELETE /pix/v2/webhook/{chave}` | webhook.write | 10 | 10 | 204, 403, 404, 503 |
| `GET /pix/v2/webhook/callbacks` | webhook.read | 10 | 10 | 200, 400, 403, 503 |
| `POST /pix/v2/webhook/callbacks/retry` | webhook.write | 5 | — | 200, 400, 403, 404, 500 |

## Cobrança V3

[Especificação oficial](https://developers.inter.co/redocusaurus/swagger-cobranca-bolepix-yaml.yaml)

| Operação e versão | Escopo | P | S | Respostas HTTP |
|---|---|---:|---:|---|
| `GET /cobranca/v3/cobrancas` | boleto-cobranca.read | 120 | 10 | 200, 400, 403, 404, 500 |
| `POST /cobranca/v3/cobrancas` | boleto-cobranca.write | 120 | 10 | 200, 400, 403, 500, 503 |
| `GET /cobranca/v3/cobrancas/{codigoSolicitacao}` | boleto-cobranca.read | 120 | 10 | 200, 400, 403, 404, 500 |
| `PATCH /cobranca/v3/cobrancas/{codigoSolicitacao}` | boleto-cobranca.write | 10 | 10 | 200, 400, 403, 404, 500 |
| `GET /cobranca/v3/cobrancas/edicao/{codigoEdicao}` | boleto-cobranca.read | 10 | 10 | 200, 400, 403, 404, 500 |
| `GET /cobranca/v3/cobrancas/{codigoSolicitacao}/pdf` | boleto-cobranca.read | 120 | 10 | 200, 400, 403, 404, 500 |
| `POST /cobranca/v3/cobrancas/{codigoSolicitacao}/cancelar` | boleto-cobranca.write | 120 | 10 | 202, 400, 403, 404, 500 |
| `GET /cobranca/v3/cobrancas/sumario` | boleto-cobranca.read | 120 | 10 | 200, 400, 403, 404, 500 |
| `POST /cobranca/v3/cobrancas/{codigoSolicitacao}/pagar` | boleto-cobranca.write | — | 10 | 204, 400, 403, 500, 503 |
| `PUT /cobranca/v3/cobrancas/webhook` | boleto-cobranca.write | 5 | 5 | 204, 500 |
| `GET /cobranca/v3/cobrancas/webhook` | boleto-cobranca.read | 5 | 5 | 200, 404, 500 |
| `DELETE /cobranca/v3/cobrancas/webhook` | boleto-cobranca.write | 5 | 5 | 204, 404, 500 |
| `GET /cobranca/v3/cobrancas/webhook/callbacks` | boleto-cobranca.read | 10 | 10 | 200, 400, 500 |
| `POST /cobranca/v3/webhook/callbacks/retry` | boleto-cobranca.write | 5 | — | 200, 400, 403, 404, 500 |

## Pix Automático

Em 25/11/2026, o proprietário informou que a empresa é elegível. Essa informação não constitui confirmação independente do Inter nem prova de habilitação dos escopos. Antes de integração externa: validar o cadastro com o banco, autorizar `rec.*` e `solicrec.*`, confirmar credenciais/certificados e homologar consentimento, recorrência, cobranças e callbacks no sandbox oficial. Nenhum fluxo MOCK abaixo envia operações ao banco.

### Fluxo oficial e pré-requisitos de dados

O contrato separa etapas que o MOCK hoje representa numa única solicitação local:

1. `POST /pix/v2/rec` (`rec.write`) registra a recorrência vinculada a contrato, calendário e dados do devedor (nome + CPF ou CNPJ); retorna o `idRec` oficial.
2. `POST /pix/v2/solicrec` (`solicrec.write`) envia a solicitação de consentimento à conta do pagador. Exige `idRec`, prazo de expiração e dados bancários/identificação do pagador (`conta`, `ispbParticipante` e CPF ou CNPJ) conforme schema oficial. O retorno/status deve ser acompanhado; aceite, recusa, expiração e cancelamento são estados distintos.
3. Com a recorrência aceita, `POST /pix/v2/cobr` (`cobr.write`) emite cada cobrança recorrente, com vencimento, valor, recebedor e os dados do devedor exigidos pelo contrato; a liquidação não é inferida da criação.
4. Acompanhar consultas `rec.read`, `solicrec.read`, `cobr.read` e configurar `webhookrec.*` / `webhookcobr.*`, deduplicando notificações antes de atualizar o estado local.

O MOCK não coleta CPF/CNPJ nem dados bancários e usa UUID interno, não os identificadores oficiais de 29 caracteres. Portanto não se pode enviar diretamente o registro MOCK ao Inter. Antes de implementar formulário/armazenamento externo, definir finalidade, consentimento e proteção de dados pessoais. A matriz de integração já usa OAuth mTLS; credenciais de sandbox são esperadas em arquivos privados `sandbox/{client_id,client_secret,cert.pem,key.pem}` sob `INTER_CREDENTIALS_ROOT`, com `INTER_ACCOUNT_NUMBER` e vínculo local configurados. Ainda não foi verificado se esses materiais ou scopes existem no ambiente.

**Inconsistência a confirmar no sandbox:** o schema publicado de `RecBaseObjeto.devedor` e `DadosBancarios` lista simultaneamente CPF e CNPJ como obrigatórios, mas sua descrição e alternativas `oneOf` indicam CPF ou CNPJ. Alguns exemplos também representam números de identificação como número JSON, perdendo zeros iniciais. Preservar identificadores como strings e confirmar com o Inter a validação efetiva antes de considerar o adapter de escrita homologado.

[Especificação oficial](https://developers.inter.co/redocusaurus/swagger-api-pix-automatico-yaml.yaml)

Consulta individual: `GET /pix/v2/rec/{idRec}` usa `rec.read`, retorna `RecCompleta` em HTTP 200, 403 para acesso negado, 404 para recorrência ausente e 503 para indisponibilidade. O adaptador local, restrito a SANDBOX, confere vínculo da empresa, identificador retornado e estados conhecidos; usa cache de token dedicado e limitador Pix. Testado com servidor local sintético; homologação oficial pendente.

| Operação e versão | Escopo | P | S | Respostas HTTP |
|---|---|---:|---:|---|
| `POST /pix/v2/rec` | rec.write | 120 | 10 | 201, 400, 403, 503 |
| `GET /pix/v2/rec` | rec.read | 120 | 10 | 200, 403, 503 |
| `GET /pix/v2/rec/{idRec}` | rec.read | 120 | 10 | 200, 403, 404, 503 |
| `PATCH /pix/v2/rec/{idRec}` | rec.write | 120 | 10 | 200, 400, 403, 404, 503 |
| `PATCH /pix/v2/sandbox/rec/{idRec}/status` | pix.write | — | 10 | 200, 400, 404, 500 |
| `POST /pix/v2/sandbox/cob/pagamento` | pix.write | — | 10 | 200, 400, 404, 500 |
| `POST /pix/v2/solicrec` | solicrec.write | 120 | 10 | 201, 400, 403, 404, 503 |
| `GET /pix/v2/solicrec/{idSolicRec}` | solicrec.read | 120 | 10 | 200, 403, 404, 503 |
| `PATCH /pix/v2/solicrec/{idSolicRec}` | solicrec.write | 120 | 10 | 201, 400, 403, 404, 503 |
| `PATCH /pix/v2/sandbox/solicrec/{idRec}/status` | solicrec.write | — | 10 | 200, 400, 404, 500 |
| `POST /pix/v2/cobr` | cobr.write | 120 | 10 | 201, 400, 403, 503 |
| `GET /pix/v2/cobr` | cobr.read | 120 | 10 | 200, 403, 503 |
| `POST /pix/v2/cobr/{txid}/retentativa/{data}` | cobr.write | 120 | 10 | 201, 400, 403, 404, 503 |
| `GET /pix/v2/cobr/{txid}` | cobr.read | 120 | 10 | 200, 403, 404, 503 |
| `PUT /pix/v2/cobr/{txid}` | cobr.write | 120 | 10 | 201, 400, 403, 404, 503 |
| `PATCH /pix/v2/cobr/{txid}` | cobr.write | 120 | 10 | 200, 400, 403, 404, 503 |
| `PATCH /pix/v2/sandbox/cobr/{txId}/status` | cobr.write | — | 10 | 200, 400, 404 |
| `POST /pix/v2/sandbox/cobr/pagamento` | cobr.write | — | 10 | 200, 400, 404, 500 |
| `POST /pix/v2/locrec` | payloadlocationrec.write | 120 | 10 | 201, 403, 503 |
| `GET /pix/v2/locrec` | payloadlocationrec.read | 120 | 10 | 200, 403, 503 |
| `GET /pix/v2/locrec/{id}` | payloadlocationrec.read | 120 | 10 | 200, 403, 404, 503 |
| `DELETE /pix/v2/locrec/{id}/idRec` | payloadlocationrec.write | 120 | 10 | 200, 403, 404, 503 |
| `PUT /pix/v2/webhookrec` | webhookrec.write | 10 | 10 | 200, 400, 403, 404, 503 |
| `GET /pix/v2/webhookrec` | webhookrec.read | 10 | 10 | 200, 403, 404, 503 |
| `DELETE /pix/v2/webhookrec` | webhookrec.write | 10 | 10 | 204, 403, 404, 503 |
| `PUT /pix/v2/webhookcobr` | webhookcobr.write | 10 | 10 | 200, 400, 403, 404, 503 |
| `GET /pix/v2/webhookcobr` | webhookcobr.read | 10 | 10 | 200, 403, 404, 503 |
| `DELETE /pix/v2/webhookcobr` | webhookcobr.write | 10 | 10 | 204, 403, 404, 503 |

## Forum

[Especificação oficial](https://developers.inter.co/redocusaurus/swagger-api-forum-yaml.yaml)

| Operação e versão | Escopo | P | S | Respostas HTTP |
|---|---|---:|---:|---|
| `GET /forum/v1/publication` | publication.read | 120 | 20 | 200, 204, 403 |
| `POST /forum/v1/publication` | publication.write | 120 | 20 | 200, 400, 403 |
| `PUT /forum/v1/publication` | publication.write | 120 | 20 | 200, 400, 403 |
| `DELETE /forum/v1/publication` | publication.delete | 120 | 20 | 200, 403 |
| `GET /forum/v1/publications` | publication.read | 120 | 20 | 200, 403, 404, 500 |
| `POST /forum/v1/media/upload-url` | publication.write | 120 | 20 | 200, 400, 403 |

## Restrições para as próximas fases

¹ O contrato Pix contém um espaço no caminho da simulação de pagamento com vencimento via QR Code. Confirmar o caminho com o banco antes de qualquer uso. As operações de simulação de pagamento e mudança de estado identificadas como sandbox no contrato são exclusivas de homologação.

O banco descreve dados fictícios no sandbox de leitura. Antes de cada implementação, registrar paginação, modelo tipado, erros, autenticação, suporte de homologação e testes do contrato. A existência do host de homologação por família exige confirmação por operação.

A Cobrança V3 prevê acompanhamento por `codigoSolicitacao`. Aceitação para processamento e liquidação são estados distintos. [Histórico de mudanças](https://developers.inter.co/changelog/18/05/2026).

Para Pix Automático, o proprietário informou elegibilidade, mas a habilitação continua dependendo de validação do banco e concessão dos escopos. Crédito imobiliário possui contrato próprio para parceiros. CNAB utiliza intercâmbio de arquivos. [Catálogo](https://developers.inter.co/docs/introducao/nossas-apis).

Para callbacks, verificar o mecanismo de autenticação de cada família e validar a associação da conta antes do processamento. [Webhooks](https://developers.inter.co/docs/webhooks/o-que-e-webhooks).

## Contratos de recebimentos e autenticação dos callbacks

Na revisão de 24/11/2026, a consulta individual de Pix recebido usa `GET /pix/v2/pix/{e2eId}` e `pix.read`. O identificador tem 32 caracteres alfanuméricos. A consulta individual de Cobrança V3 usa `GET /cobranca/v3/cobrancas/{codigoSolicitacao}` e `boleto-cobranca.read`. Ambas selecionam a conta por `x-conta-corrente`, dispensam paginação e usam OAuth/mTLS. Os limites e erros estão nas tabelas por família. A emissão V3 retorna HTTP 200 com um UUID para acompanhamento. O campo `situacao` distingue `EM_PROCESSAMENTO`, `A_RECEBER`, `RECEBIDO` e `MARCADO_RECEBIDO`. [Cobrança V3](https://developers.inter.co/redocusaurus/swagger-cobranca-bolepix-yaml.yaml) · [Pix V2](https://developers.inter.co/redocusaurus/swagger-api-pix-yaml.yaml)

O adaptador implementa as duas consultas individuais nos ambientes externos configurados. As emissões Pix/V3 ficam restritas ao perfil opt-in de SANDBOX. A fila persiste a referência antes da confirmação, consulta a fonte e registra valor, canal e conciliação. O servidor sintético valida método, conta, corpo, resposta e os escopos `cob.write`, `boleto-cobranca.write`, `pix.read` e `boleto-cobranca.read`. A homologação das emissões e callbacks ainda é pendência para o aceite externo da Fase 3.

O Inter orienta obter a CA específica dos callbacks em **Minhas integrações → Certificado Webhook**. O receptor HTTPS deve validar o certificado de cliente com essa CA, exigir TLS 1.2 ou superior e conferir a conta. O callback Pix traz a lista `pix`, e o da Cobrança V3 traz um array com código, situação e instante. Os dados de liquidação devem ser consultados na fonte antes de qualquer efeito. [Configuração oficial](https://developers.inter.co/docs/webhooks/como-config-webhooks/index.html) · [Callback](https://developers.inter.co/docs/webhooks/callback-webhook/index.html)
