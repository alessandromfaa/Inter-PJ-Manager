# Instruções para o Codex na outra estação

Copie o texto abaixo e envie ao Codex na máquina nova. O endereço `172.17.154.31` foi observado na estação antiga, mas pode não ser alcançável pela rede da nova estação.

```text
Quero instalar o Inter PJ Manager nesta máquina, com banco novo, usando
as credenciais de leitura do Inter que estão na estação antiga.

Trabalhe diretamente e execute todas as etapas locais possíveis. Peça minha
ajuda apenas quando precisar de autenticação SSH, sudo, endereço de rede ou
alguma informação que não possa descobrir com segurança.

A estação antiga tem estas informações:

- Usuário SSH: alessandro
- Endereço observado: 172.17.154.31
- Repositório: git@github.com:alessandromfaa/Inter-PJ-Manager.git
- Branch: main
- Arquivos da integração Inter:
  /home/alessandro/codex/inter/secrets/inter/production/client_id
  /home/alessandro/codex/inter/secrets/inter/production/client_secret
  /home/alessandro/codex/inter/secrets/inter/production/cert.pem
  /home/alessandro/codex/inter/secrets/inter/production/key.pem
- O número da conta está em INTER_ACCOUNT_NUMBER no arquivo privado:
  /home/alessandro/codex/inter/.env.producao-leitura

O endereço 172.17.154.31 foi observado na estação antiga, mas pode não ser
alcançável desta máquina. Teste a conexão SSH; se não funcionar, peça-me um
endereço acessível. Não desative a verificação da chave do servidor SSH e
não exponha SSH ou a aplicação à internet para resolver conectividade.

Execute este fluxo:

1. Confira Git, Docker, Docker Compose, OpenSSL e SCP. Clone a branch main:

   git clone -b main git@github.com:alessandromfaa/Inter-PJ-Manager.git
   cd Inter-PJ-Manager

   Se o acesso ao GitHub por SSH falhar, explique a autenticação necessária
   e aguarde; não coloque tokens em comandos, logs ou arquivos versionados.

2. Leia AGENTS.md, README.md e docs/inter.md. Esta instalação deve ter banco,
   empresa, usuários, sessões e segredos de aplicação novos. Não copie o
   banco, os backups, as senhas do PostgreSQL ou a chave MFA antiga.

3. Crie a configuração inicial em MOCK a partir de .env.example. Pergunte-me
   qual e-mail de administrador e nome de empresa desejo usar antes do
   primeiro bootstrap. Não mostre senhas geradas na conversa nem nos logs.
   Inicie a instalação local, confirme /ready, faça login e confira a sessão.

4. Crie uma pasta privada dentro do clone e copie da estação antiga apenas
   os quatro arquivos do Inter:

   mkdir -p secrets/inter/production

   Para cada nome em client_id, client_secret, cert.pem e key.pem, execute
   o SCP a partir da outra estação, usando o usuário e o endereço acima.
   Exemplo para o primeiro arquivo:

   scp alessandro@172.17.154.31:/home/alessandro/codex/inter/secrets/inter/production/client_id \
     secrets/inter/production/client_id

   Repita para os outros três nomes. Não exiba o conteúdo dos arquivos.
   Verifique se são arquivos regulares não vazios e se o backend do Docker,
   que usa UID 65532, consegue lê-los. Ajuste proprietário e permissões com
   segurança conforme o tipo de Docker desta máquina.

5. Obtenha somente INTER_ACCOUNT_NUMBER do arquivo privado da estação antiga
   por SSH ou SCP, sem imprimir seu valor na conversa nem colocá-lo no histórico
   de comandos. Grave-o somente na configuração privada da nova instalação.
   Não copie os UUIDs antigos: obtenha o company_id da empresa criada no banco
   novo e gere um UUID novo e estável para INTER_ACCOUNT_ID.

6. Prepare a configuração para leitura de produção:

   APP_ENV=production
   INTER_ENV=production
   ENABLE_BANK_READS=true
   ENABLE_BANK_WRITES=false
   ENABLE_REAL_PAYMENTS=false

   Configure INTER_CREDENTIALS_ROOT com o caminho absoluto da nova pasta
   secrets/inter, os dois UUIDs novos e o número da conta. Configure
   APP_ORIGIN com a origem HTTPS exata, sem barra final.

7. Prepare e valide HTTPS local em loopback. Não exponha a aplicação à rede
   ou à internet. Se usar certificado local autoassinado, explique como
   confiar nele na máquina nova e verifique o acesso HTTPS; não trate um
   aviso do navegador como certificado publicamente confiável.

8. Prepare o backup diário para o projeto e os caminhos desta máquina,
   preservando todos os backups e sem configurar exclusão automática.
   Informe a localização dos backups e o resultado da primeira execução.

9. Valide a instalação, a configuração efetiva e os controles. Mantenha
   ENABLE_BANK_WRITES e ENABLE_REAL_PAYMENTS em false. Não realize
   sincronização bancária real, transação, publicação externa, alteração de
   DNS ou acesso a outros ambientes sem minha autorização específica.

10. Ao terminar, informe o endereço local de acesso, o e-mail de login,
    como consultar a senha inicial em local privado, o resultado dos testes
    e o que ainda falta para usar a leitura real. Não mostre credenciais,
    chaves, tokens, dados bancários ou conteúdo de arquivos secretos.
```
