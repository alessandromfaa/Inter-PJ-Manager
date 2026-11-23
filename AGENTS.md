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
