## Purpose

Proteger os segredos das contas e limitar sua utilização a sessões autorizadas nativamente no macOS, com falhas seguras e revogação consistente entre armazenamento, interface e operações assíncronas.

## ADDED Requirements

### Requirement: Native protected persistence

O app MUST persistir seeds e registros sensíveis exclusivamente em itens do Keychain tradicional do macOS, com ACL que confie no aplicativo criador para operações sensíveis. O funcionamento MUST NOT exigir conta Apple, perfil de provisionamento ou entitlements do data protection Keychain. Seeds MUST NOT ser persistidos em storage do webview, arquivos de preferências, logs ou arquivos temporários. A única saída de seeds para um arquivo MUST ser uma exportação explicitamente autorizada conforme a capacidade de compatibilidade.

#### Scenario: Successful account storage
- **WHEN** uma conta é cadastrada e o app é reiniciado
- **THEN** o seed é recuperável pelo backend autorizado a partir do Keychain e não aparece em storage web nem arquivos comuns do app

#### Scenario: Keychain unavailable
- **WHEN** o Keychain nega ou não consegue completar uma gravação
- **THEN** o app reporta indisponibilidade, não declara sucesso e não usa armazenamento alternativo em texto puro

### Requirement: Native session authentication

O desbloqueio MUST exigir avaliação bem-sucedida de LocalAuthentication no core nativo antes de permitir leitura de credenciais pelo aplicativo. O app MUST oferecer Touch ID quando disponível e autenticação nativa pela senha do macOS como fallback. Cancelamento, recusa ou falha MUST manter o cofre bloqueado sem ler seus seeds; uma senha mestre própria MUST NOT ser requisito para consultas normais. A autorização MUST ser produzida e mantida pelo backend, sem aceitar booleanos ou provas de sucesso fornecidos pelo frontend. O contrato MUST distinguir o bloqueio de sessão do aplicativo da ACL do Keychain: os itens não exigem biometria independentemente do processo autorizado.

#### Scenario: Touch ID authorizes the application session
- **WHEN** o usuário solicita desbloqueio e LocalAuthentication aprova Touch ID no backend
- **THEN** uma sessão autorizada é estabelecida e pode ler contas pelo Keychain sem novo prompt a cada atualização de código

#### Scenario: Mac without Touch ID
- **WHEN** não há biometria configurada e o usuário solicita desbloqueio
- **THEN** a senha do usuário do macOS pode autorizar a sessão pelo diálogo nativo; se nenhuma política suportada puder autenticar o usuário, o cofre permanece bloqueado

#### Scenario: Authentication is cancelled
- **WHEN** o usuário cancela o diálogo nativo
- **THEN** nenhum seed é lido, nenhuma conta/código é revelado e nenhuma autorização de sessão é criada

#### Scenario: Frontend supplies a forged authentication result
- **WHEN** um cliente tenta fornecer um booleano de sucesso ou invocar uma operação de conta sem autenticação nativa
- **THEN** a operação é recusada e nenhuma leitura de credencial do Keychain é executada

### Requirement: Bounded authorization session

O app MUST iniciar bloqueado e revogar a sessão ao bloquear manualmente, bloquear a sessão do macOS, suspender o sistema ou encerrar. Sem interação explícita do usuário, a sessão MUST expirar após 120 segundos; sua duração máxima MUST ser 300 segundos, e ocultar o painel MUST encerrar a sessão após 15 segundos contínuos oculto. Atualizações automáticas de TOTP MUST NOT renovar esses limites. Retomar o sistema MUST exigir novo desbloqueio.

#### Scenario: Quick panel reopen
- **WHEN** o usuário reabre o painel antes de 15 segundos oculto e os demais limites ainda estão válidos
- **THEN** a sessão existente pode continuar sem novo prompt

#### Scenario: Background code refresh
- **WHEN** códigos continuam sendo atualizados sem interação por 120 segundos
- **THEN** a sessão expira e contas e códigos são removidos da interface

#### Scenario: System sleep or screen lock
- **WHEN** o macOS é bloqueado ou entra em suspensão durante uma sessão
- **THEN** a autorização é revogada e, após retorno, o app exige novo desbloqueio

### Requirement: Native authorization on every sensitive operation

O backend MUST validar a autorização para listagem de contas, geração/cópia de OTP, mutações, importação e exportação. Essa validação MUST ocorrer novamente antes de liberar resultados ou iniciar efeitos sensíveis após uma espera assíncrona. Bloquear MUST invalidar autenticações pendentes, prévias de importação e respostas de sessões anteriores.

#### Scenario: Direct command while locked
- **WHEN** um cliente tenta gerar código ou listar contas sem sessão válida
- **THEN** o backend rejeita a operação independentemente do estado visual do frontend

#### Scenario: Lock during authentication
- **WHEN** Bloquear é acionado e depois chega sucesso de uma autenticação iniciada antes do bloqueio
- **THEN** esse sucesso antigo não restabelece a sessão

#### Scenario: Lock during import or code calculation
- **WHEN** o cofre é bloqueado enquanto uma operação ainda está calculando ou aguardando confirmação
- **THEN** resultados antigos não repovoam a UI, não autorizam nova gravação e não escrevem no clipboard

### Requirement: Minimal presentation boundary

Comandos comuns e eventos de apresentação MUST retornar somente metadados autorizados, estado de sessão e códigos temporários; MUST NOT retornar seeds existentes, senhas, contexto de autenticação ou backups descriptografados. Textos de contas importados MUST ser tratados como texto e nunca como HTML executável.

#### Scenario: Inspect list and code responses
- **WHEN** contas e códigos são obtidos em uma sessão válida
- **THEN** as respostas não contêm seeds, URIs secretas ou material de derivação de chaves

#### Scenario: Malicious imported label
- **WHEN** uma conta possui nome contendo marcação HTML ou script
- **THEN** o nome é exibido como texto e não executa código nem amplia permissões nativas

### Requirement: Recoverable persistence failures

Mutações MUST ser reconhecidas como concluídas somente após sua persistência. Uma operação de importação MUST tornar o conjunto aprovado visível integralmente ou manter o conjunto anterior, mesmo após falha intermediária ou reinício. Exclusão MUST não declarar remoção completa enquanto um item sensível permanecer sem tratamento de recuperação.

#### Scenario: Interrupted import
- **WHEN** a persistência de um conjunto aprovado falha antes de sua conclusão
- **THEN** contas anteriores continuam utilizáveis e novas contas não aparecem parcialmente; a recuperação elimina ou conclui itens pendentes sem arquivos contendo seeds

#### Scenario: Failed account deletion
- **WHEN** o Keychain recusa excluir uma conta
- **THEN** o app reporta a falha e não afirma que o segredo foi eliminado
