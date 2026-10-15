## Purpose

Evitar pedidos repetidos de senha por perda de confiança entre builds e permitir migrar o cofre ad hoc preservando contas e contadores atuais.

## ADDED Requirements

### Requirement: Persistent local signing identity

Builds locais destinados à instalação MUST reutilizar uma identidade de assinatura persistente no Keychain, sem exigir conta Apple nem alterar configurações globais de confiança. A chave privada MUST NOT integrar repositório, bundle ou artefatos distribuídos. Identidade ausente/expirada MUST interromper o build, sem recriar silenciosamente outra. Atualizações normais MUST passar na avaliação de requisitos designados nos dois sentidos e em teste sintético real de acesso ao Keychain entre builds. Builds ad hoc de CI MUST ser identificados como insuficientes para continuidade do cofre.

#### Scenario: Two different builds with the same identity

- **WHEN** dois binários diferentes usam o mesmo identificador e certificado local
- **THEN** o segundo lê e atualiza itens sintéticos criados pelo primeiro sem UI do Keychain, e outro identificador assinado pelo mesmo certificado continua recusado

#### Scenario: Identity is unavailable

- **WHEN** a identidade registrada não está disponível no Keychain
- **THEN** o build para com instrução recuperável e não cria outra chave automaticamente

### Requirement: No storage authentication loop

O armazenamento MUST operar sem UI própria de Keychain. Recusa de confiança ou necessidade de interação MUST retornar erro distinto, revogar a sessão e impedir novas leituras automáticas. O diálogo explícito de LocalAuthentication MUST permanecer disponível com a política existente. Falha MUST NOT ampliar ACLs, substituir autenticação por um booleano ou salvar seeds fora do Keychain.

#### Scenario: Keychain denies a visible refresh

- **WHEN** uma leitura autorizada falha por acesso recusado
- **THEN** a sessão é revogada, apresentações/prévias são limpas e ticks seguintes não repetem leituras ou prompts

### Requirement: One time encrypted backup migration

A migração inicial escolhida pelo usuário MUST usar backup criptografado exportado na UI estável com as contas e contadores atuais. A nova identidade MUST usar um cofre separado e MUST NOT ler/apagar/reescrever os itens antigos. Havendo manifesto antigo e nenhum manifesto novo, o app MUST orientar a importação após autenticação em vez de apresentar um cofre vazio como concluído. Senha MUST ser fornecida somente no app, transitoriamente. Prévia e commit MUST preservar ordem, favoritos, parâmetros e próximo contador HOTP do arquivo, sem consumir códigos. Cancelamento ou falha MUST preservar o cofre antigo e manter a migração disponível.

#### Scenario: Import the latest encrypted stable backup

- **WHEN** o usuário escolhe o arquivo, fornece sua senha e confirma as contas
- **THEN** o commit grava somente o novo cofre e mantém parâmetros, favoritos, ordem e contadores atuais; o cofre antigo continua preservado

#### Scenario: Cancel or fail before commit

- **WHEN** seleção, senha ou gravação não é concluída
- **THEN** a orientação continua disponível e nenhum sucesso de migração é declarado

#### Scenario: HOTP after the export

- **WHEN** o usuário prepara a migração
- **THEN** a interface orienta usar o backup mais recente e não consumir novos códigos na versão antiga depois de exportar, pois os cofres não sincronizam contadores
