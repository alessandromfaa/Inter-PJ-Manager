# Inter PJ Manager

- Escreva interface, mensagens operacionais e documentação em português brasileiro.
- Desenvolva no checkout local. O ambiente SEFIN está fora do escopo. Serviços de desenvolvimento escutam em loopback.
- Use MOCK no desenvolvimento e nos testes comuns. Bloqueie inicialização com ambiente desconhecido ou escrita bancária habilitada enquanto a capacidade estiver indisponível.
- Verifique o contrato publicado pelo Inter e atualize docs/inter-api-matrix.md antes de integrar uma operação. Registre incertezas e teste o adapter sem movimentação real.
- Resolva a empresa a partir da sessão no backend. Use SQL parametrizado e chaves compostas para vínculos multiempresa.
- Faça cálculos financeiros com representação exata. Distinga solicitação, aprovação, processamento, liquidação e resultado indeterminado.
- Preserve Argon2id, validação de origem, RBAC, cookies HttpOnly e Secure em produção. Escritas sensíveis exigem MFA, dupla autorização e idempotência.
- Mantenha certificados, chaves, dumps, senhas e tokens fora de Git, logs e frontend.
- Execute formatação, lint, auditorias, testes e build pertinentes. Relate separadamente validações documentais, mocks e homologação bancária.
- Solicite autorização para operações destrutivas, publicação, mudanças em produção ou transação real.

## Coordenação multiagente

O coordenador usa o modelo e o esforço de raciocínio definidos em `.codex/config.toml`. Os perfis em `.codex/agents/` vinculam cada especialidade ao modelo e ao esforço acordados.

Se o cliente avisar que ignorou a configuração do projeto, omitir o perfil solicitado ou registrar um modelo ou esforço divergente, suspenda a delegação afetada. Informe a divergência e siga o guia de ativação e diagnóstico. [Operação multiagente](docs/multiagentes.md)

1. Ao receber uma tarefa, classifique sua natureza, complexidade (rotineira, intermediária ou arquitetural) e risco (baixo, moderado, alto ou crítico). Identifique arquivos, limites de confiança e critério de aceite. Consultas simples exigem apenas uma classificação breve.
2. Delegue o trabalho do projeto automaticamente ao perfil correspondente. Use `backend` para Rust/API, `frontend` para React/UX, `database` para SQL/migrations, `tester` para verificação e `documentation` para documentação. Em tarefas mistas, separe as responsabilidades e indique quem integra. Use o nome do perfil na chamada de delegação.
3. Acione `architect` antes de decisões arquiteturais relevantes, especialmente quando alteram limites entre módulos, contratos públicos, persistência ou recuperação de falhas.
4. Atribua integrações com o Banco Inter a `banking`. Exija a fonte oficial e evidências distintas para MOCK, sandbox e produção.
5. Toda mudança de funcionalidade financeira exige revisão de `security`, por agente diferente do autor, inclusive mudanças de autenticação, exposição de dados, callbacks e operações idempotentes.
6. Use paralelismo apenas quando entradas, dependências e critérios de aceite permitirem trabalho independente. O limite é de quatro subagentes concorrentes. O coordenador organiza as filas e integra resultados dependentes em sequência.
7. Atribua um responsável por arquivo durante cada rodada. Se surgir sobreposição de edição, serialize as alterações ou redistribua a propriedade antes de escrever. Preserve as alterações locais do usuário. Revisores trabalham em leitura.
8. Antes de declarar a implementação concluída, execute os testes pertinentes ao diff e registre comando e resultado. Para configuração, exercite o carregamento pelo consumidor real. Uma verificação pendente deve aparecer no aceite.
9. Cada entrega relevante passa por `reviewer` independente depois da implementação. O autor corrige os achados e o revisor confirma a resolução. A revisão financeira de `security` também é obrigatória.
10. P0 compreende impacto crítico imediato, como exposição ativa de segredo ou movimentação indevida. P1 compreende falha grave plausível, como quebra de isolamento ou duplicação de operação financeira. Com P0/P1 aberto, bloqueie o aceite, a publicação e o avanço de fase até correção e nova revisão.
11. No relatório, registre por tarefa os agentes acionados, modelo e esforço observados, evidência de execução e resultado. Distinga configuração solicitada e metadados efetivos. Se o runtime omitir um dado, identifique-o como indisponível.
12. Se faltar um perfil, modelo ou nível de raciocínio, interrompa a parte dependente, informe o item indisponível e solicite decisão humana para uma substituição. Não escolha silenciosamente outro modelo ou agente genérico.
13. Envie a cada agente o menor contexto suficiente, com objetivo, arquivos, restrições e saída esperada. Evite análises duplicadas e criação de agentes sem trabalho definido. Encerre agentes concluídos para liberar capacidade.
14. Solicite aprovação humana antes de operações destrutivas, exposição à internet, alteração em produção, publicação externa, transação financeira real ou exceção à política de modelos. Considere a aprovação já concedida para o escopo exato. SEFIN permanece fora do escopo.
