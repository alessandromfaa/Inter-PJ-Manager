# Inter PJ Manager

Gestão financeira empresarial em português brasileiro. A primeira entrega oferece autenticação própria, painel e extrato de demonstração, auditoria por empresa e infraestrutura local. O ambiente inicial é **MOCK**. Os dados bancários são fictícios e as sessões e os eventos de acesso são persistidos no PostgreSQL.

## Iniciar do zero

Use Linux com Git, OpenSSL, Docker Engine e Docker Compose v2, ou Podman com provedor Compose que suporte dependências por saúde e término bem-sucedido. A primeira compilação precisa de internet para baixar imagens e dependências. Execute na raiz do checkout, em ambiente de desenvolvimento autorizado.

```sh
cp .env.example .env
./infrastructure/scripts/iniciar.sh
docker compose ps -a
curl -f http://127.0.0.1:8080/ready
```

Abra **http://127.0.0.1:8080**. O login inicial é `admin@local.test`. Leia a senha gerada em `secrets/admin` em um terminal privado. Para personalizar o e-mail ou o nome da empresa, edite ADMIN_EMAIL e COMPANY_NAME antes da primeira inicialização. A pasta secrets tem modo 0700 e deve permanecer sob controle do usuário local. O inicializador gera valores aleatórios apenas quando os arquivos estão ausentes.

O bootstrap aplica as migrations e cria a primeira empresa e seu administrador em transação. Reiniciar o serviço preserva o usuário, a senha e os dados no volume. O PostgreSQL e a API ficam na rede interna do Compose. A porta publicada da interface escuta em loopback. Para encerrar preservando os dados, use `docker compose down`.

### Executar neste checkout sem motor de contêineres

Neste checkout foram preparadas ferramentas portáteis em .tools. Para desenvolver na máquina local sem alterar os pacotes do sistema, execute:

```sh
./infrastructure/scripts/mock-local.sh
```

Acesse **http://127.0.0.1:5173** com `admin@local.test`. A senha fica em `.tools/mock-local/admin`. O script mantém um cluster de desenvolvimento próprio, preserva os dados entre execuções e encerra seus processos ao receber Ctrl+C. Em um clone novo, essa alternativa requer Rust, Node.js, PostgreSQL com pg_config, OpenSSL e flock disponíveis. A instalação via Compose exige apenas as ferramentas da primeira seção.

### Podman

```sh
ENGINE=podman ./infrastructure/scripts/iniciar.sh
podman compose ps -a
```

O mesmo manifesto é usado pelos dois motores. Em sistemas com SELinux, configure o contexto dos arquivos montados conforme as regras locais. Use a mesma identidade e motor durante backup e restauração.

## Funcionalidades disponíveis

- Login com Argon2id, encerramento de sessão, bloqueio temporário após falhas e proteção de origem.
- Painel de novembro de 2026 com saldo disponível, entradas, saídas, evolução e últimas movimentações. Extrato com busca e filtro de tipo.
- Layout responsivo, temas claro e escuro, indicação permanente de MOCK e identificação da base fictícia.
- Auditoria local restrita a ADMIN e AUDITOR. Todos os cinco perfis existem no backend. O provisionamento de usuários adicionais ocorre por comando administrativo local.
- Tabelas e vínculos multiempresa para as fases seguintes. A Fase 2 permite sincronização explícita de saldo e extrato em SANDBOX ou PRODUCTION. O administrador configura credenciais e habilita a leitura. Escritas bancárias continuam bloqueadas. Consulte o [guia de configuração](docs/inter.md).

Em MOCK, abra **Cobranças Pix** e clique em **Consultar cobrança** para consultar o exemplo fictício já preenchido. A tela mostra situação, revisão e valor original. Para emitir, entre como ADMIN ou OPERATOR, escolha **Nova cobrança em MOCK**, informe o valor e selecione **Emitir cobrança fictícia**. Você pode copiar o código ou consultar a cobrança pelo txid. Se a resposta se perder, **Retomar emissão** recupera a mesma cobrança na aba. O QR Code de demonstração é impróprio para pagamentos reais.

## Guias

| Assunto | Documento |
|---|---|
| Arquitetura, dados, fluxos e decisões | [Arquitetura](docs/architecture.md) |
| Evidências de testes da primeira entrega | [Validação da Fase 1](docs/validacao-fase-1.md) |
| Entregas, dependências e critérios por fase | [Plano incremental](docs/implementation-plan.md) |
| Ambiente local, desenvolvimento e testes | [Desenvolvimento](docs/desenvolvimento.md) |
| Configuração, administração, HTTPS, atualização, backup, restauração e diagnóstico | [Operação](docs/operacao.md) |
| Controles implementados e requisitos de produção | [Segurança](docs/seguranca.md) |
| Autenticação e rotas internas | [Referência HTTP](docs/api-interna.md) |
| Credenciais e homologação no Banco Inter | [Integração Inter](docs/inter.md) |
| Endpoints, versões, limites e permissões | [Matriz do Inter](docs/inter-api-matrix.md) |

O código próprio aguarda a definição de licença pelo proprietário. Consulte [LICENSE](LICENSE).
