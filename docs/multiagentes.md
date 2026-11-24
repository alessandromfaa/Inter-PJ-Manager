# Desenvolvimento multiagente do Inter PJ Manager

## Perfis do checkout

O coordenador classifica o trabalho pela especialidade, complexidade e risco. A combinação de modelo e esforço é fixa por perfil. Um ajuste simples de formulário cabe a `frontend`. Já uma mudança que altere a aprovação de um pagamento requer decisão arquitetural quando afetar contratos, implementação pelo responsável, testes e revisões independentes. As regras de encaminhamento e aceite ficam no [AGENTS.md](../AGENTS.md).

| Perfil | Modelo | Raciocínio | Responsabilidade |
| --- | --- | --- | --- |
| Coordenador | `gpt-6.2-astra` | `high` | Classificar, delegar, integrar e apresentar evidências |
| `architect` | `gpt-6.2-astra` | `xhigh` | Decisões arquiteturais e limites entre módulos |
| `backend` | `gpt-6.2-sol` | `high` | Rust, Axum, jobs e regras de negócio |
| `frontend` | `gpt-6.2-sol` | `medium` | Interface React, acessibilidade e fluxos |
| `banking` | `gpt-6.2-astra` | `xhigh` | Contratos oficiais e integração com o Inter |
| `security` | `gpt-6.2-astra` | `xhigh` | Revisão de segurança e de funcionalidades financeiras |
| `database` | `gpt-6.2-astra` | `high` | PostgreSQL, isolamento, migrações e transações |
| `tester` | `gpt-6.2-sol` | `high` | Testes de contrato, integração e regressão |
| `documentation` | `gpt-6-luna` | `medium` | Documentação de operação e continuidade |
| `reviewer` | `gpt-6.2-astra` | `xhigh` | Revisão da entrega por agente diferente do autor |

O limite configurado permite quatro subagentes abertos simultaneamente, além do coordenador. A profundidade escolhida é um. Só o coordenador abre subagentes. Atribua cada arquivo a um único responsável por rodada e aguarde a implementação antes de iniciar a revisão final. O encaminhamento automático depende das instruções que o coordenador recebe. A política de bloqueio por P0/P1 é aplicada no aceite pelo coordenador e pelo revisor. [Documentação oficial de subagentes](https://developers.openai.com/codex/subagents)

## Ativação

1. No terminal, entre na raiz do repositório. Revise o conteúdo de `.codex/` e do `AGENTS.md`.
2. Confira a versão com `codex --version`. Consulte os modelos e esforços anunciados no catálogo com `codex debug models`. Uma delegação concluída confirma o uso do modelo pela conta naquela sessão. A validação deste projeto usou o CLI 0.171.0.
3. Inicie uma **nova sessão** com `codex --strict-config`. Confirme a confiança no checkout específico quando o CLI solicitar. A confiança permite carregar a configuração local e também pode habilitar servidores MCP definidos pelo projeto. Se receber aviso de configuração local ignorada, interrompa a tarefa e verifique a confiança deste repositório. [Configuração e precedência](https://developers.openai.com/codex/config-basic)
4. Confira modelo e esforço da sessão em `/status`. Envie o objetivo normalmente. O coordenador usa o `AGENTS.md` e os perfis registrados em `.codex/agents/`. Acompanhe as delegações com `/subagents`. Perfis da organização, parâmetros de linha de comando e a disponibilidade na conta podem influenciar os valores efetivos. [Referência de configuração](https://developers.openai.com/codex/config-reference)

## Demonstração em leitura

Execute o comando na raiz de um checkout já confiável. O exemplo deixa a seleção do especialista a cargo do coordenador e restringe o shell à leitura do checkout.

```sh
codex exec --strict-config --sandbox read-only \
  'Delegue uma consulta documental conforme AGENTS.md. Peça ao especialista que identifique o ambiente bancário padrão e o ambiente excluído do projeto. Aguarde sua resposta e informe perfil, resultado e metadados disponíveis.'
```

Se preferir registrar a confiança pelo modo não interativo, revise os arquivos e acrescente `--trust` ao comando acima. Essa decisão vale também para sessões futuras, mesmo com `--ephemeral`. Se houver uma decisão explícita anterior de desconfiança para o checkout, o CLI exige que o operador a altere primeiro na configuração pessoal.

O resultado esperado é a delegação para `documentation`, seguida de uma resposta identificando MOCK e a exclusão do SEFIN. Confira o evento de criação do subagente e a resposta da thread. Apenas mencionar o especialista na resposta principal é insuficiente. Os filhos herdam o sandbox, o provedor e as permissões da sessão principal. Para impor leitura em toda a revisão, use o sandbox de leitura e disponibilize apenas ferramentas e servidores MCP com permissões de leitura. O sandbox do shell permite que ferramentas externas tenham permissões próprias. [Funcionamento dos perfis personalizados](https://developers.openai.com/codex/subagents)

### Validação de 24/11/2026

O catálogo do CLI anunciou os três modelos e os níveis solicitados. Os dez arquivos TOML foram analisados. Em um perfil local temporário com confiança restrita a este checkout, a chamada `config/read` do app-server resolveu `gpt-6.2-astra/high`, agentes habilitados, limite quatro e profundidade um. No perfil pessoal sem confiança para este checkout, o CLI emitiu um aviso, ignorou a camada do projeto e resolveu `xhigh`.

Em uma sessão real com o checkout montado para leitura, o coordenador recebeu uma consulta sem indicação do nome do especialista. Os metadados das threads registraram o pai como `gpt-6.2-astra/high` e o filho com o papel `documentation`, modelo `gpt-6-luna` e esforço `medium`. O filho identificou MOCK e SEFIN e a sessão concluiu. O executor de shell aninhado recusou a abertura de processos durante esse ensaio. O conteúdo consultado foi o `AGENTS.md` carregado pelo Codex no contexto da sessão.

Em outra consulta em leitura, o perfil nativo `tester` confirmou que um P1 aberto impede o avanço de fase. O runtime registrou `gpt-6.2-sol/high`. A revisão independente foi delegada a `reviewer` com modelo e esforço registrados como `gpt-6.2-astra/xhigh`. O revisor confirmou as 14 regras e o fechamento de um P2 no guia sobre o alcance do sandbox. A revisão terminou sem P0 ou P1 aberto. Os demais perfis passaram pela validação de configuração e catálogo.

## Registro de uma entrega

Registre no relatório a tarefa, seu risco, o agente executor, os metadados disponíveis de modelo e esforço, os testes e seus resultados, o revisor e os achados abertos. Se a conta perder acesso a um modelo ou se o cliente ignorar um perfil, informe a divergência e suspenda o trabalho dependente até decisão humana. Antes de publicar, confirme que os achados P0 e P1 foram resolvidos e que há autorização para o destino. Mantenha dados bancários e credenciais fora do relatório.
