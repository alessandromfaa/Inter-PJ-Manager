# Matriz das APIs Inter Empresas

Consulta documental em 23/11/2026. OAuth, saldo e extrato completo possuem adaptador implementado e testes locais. Homologação oficial pendente. As demais operações são inventário para fases futuras. As colunas P e S indicam limites publicados em chamadas por minuto para produção e sandbox. O traço indica informação a confirmar no contrato específico.

| Família | Entrega prevista |
|---|---|
| OAuth e Banking de leitura | Fase 2 |
| Pix recebimentos, Cobrança V3 e respectivos callbacks | Fase 3 |
| Ordens de pagamento Banking | Fase 4 |
| Pix Automático | Fase 5, condicionada a elegibilidade |
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

## Pix recebimentos

[Especificação oficial](https://developers.inter.co/redocusaurus/swagger-api-pix-yaml.yaml)

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

[Especificação oficial](https://developers.inter.co/redocusaurus/swagger-api-pix-automatico-yaml.yaml)

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

Verificar escopos e requisito de seis meses de atividade do CNPJ para Pix Automático. A habilitação depende da análise do banco. Crédito imobiliário possui contrato próprio para parceiros. CNAB utiliza intercâmbio de arquivos. [Catálogo](https://developers.inter.co/docs/introducao/nossas-apis).

Para callbacks, verificar o mecanismo de autenticação de cada família e validar a associação da conta antes do processamento. [Webhooks](https://developers.inter.co/docs/webhooks/o-que-e-webhooks).
