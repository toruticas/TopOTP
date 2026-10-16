## Purpose

Evitar pedidos repetidos de senha por perda de confiança entre builds e permitir migrar o cofre ad hoc preservando contas e contadores atuais.

## ADDED Requirements

### Requirement: Stable owner in the default login Keychain

Atualizações MUST verificar também o dono exigido pela partição dos itens no Keychain login; requisitos designados iguais MUST NOT ser tratados como prova suficiente. Com certificado local sem Team ID, o app MUST usar um helper de armazenamento cujo CDHash permanece fixo entre builds da interface. O build MUST preservar esse artefato por arquitetura e recusar substituição ou regeneração silenciosa. Ausência/troca do helper MUST falhar sem ler credenciais, sem fallback direto e sem solicitar senha do Keychain.

#### Scenario: Update the interface while the helper stays fixed

- **WHEN** dois builds diferentes e legitimamente assinados acessam o mesmo item sintético no login pelo helper fixo
- **THEN** leitura/atualização e retorno ao primeiro build funcionam sem UI do Keychain, com CDHash e partição do helper preservados

#### Scenario: Replace the helper

- **WHEN** o helper ou a identidade registrada diverge do artefato fixado
- **THEN** build/validação recusam a atualização normal e não tentam abrir o cofre como se fosse compatível

### Requirement: Private bounded native storage transport

O helper MUST aceitar somente um socket anônimo herdado do processo pai. MUST validar a identidade do cliente pelo audit token do socket, certificado local e identificador do projeto, revalidando antes das operações. O core MUST validar o processo filho pelo requisito/CDHash fixado antes de enviar material sensível. O namespace MUST derivar da identidade validada, nunca de um parâmetro de serviço. O protocolo MUST oferecer somente CRUD de um UUID, limitar valores a 32 KiB e encerrar em transporte inválido/timeout. Não haverá porta, socket nomeado, comando de shell, caminho arbitrário ou API web para esse canal. A autorização por sessão/época e a biometria MUST permanecer no core/ator existentes; o helper não representa proteção biométrica independente do item.

#### Scenario: Another client or unsigned parent

- **WHEN** um cliente de outro namespace ou sem a assinatura exigida tenta ler um UUID sintético
- **THEN** não recebe o valor do outro cliente; o processo sem assinatura autorizada é recusado antes do acesso ao Keychain

#### Scenario: Broken transport or stale session

- **WHEN** o helper morre, o protocolo excede os limites ou a sessão é revogada durante uma operação
- **THEN** o cofre é bloqueado, respostas antigas são descartadas e não ocorre reconexão/autenticação automática

### Requirement: Preserve the direct vault during owner migration

A passagem do dono direto ao helper MUST usar novo serviço/manifesto, detectar o cofre direto e orientar importação por backup na UI. O cofre direto MUST permanecer intacto e utilizável pela versão restaurada. O agente MUST NOT ler os valores pessoais nem automatizar a exclusão. A mesma migração MUST NOT ser exigida em atualizações normais da interface com helper fixo.

#### Scenario: Cancel the transfer

- **WHEN** o usuário cancela a importação para o novo dono
- **THEN** nenhum item antigo é alterado e a orientação continua disponível

### Requirement: Persistent local signing identity

Builds locais destinados à instalação MUST reutilizar uma identidade de assinatura persistente no Keychain, sem exigir conta Apple nem alterar configurações globais de confiança. A chave privada MUST NOT integrar repositório, bundle ou artefatos distribuídos. Identidade ausente/expirada MUST interromper o build, sem recriar silenciosamente outra. Atualizações normais MUST passar na avaliação de requisitos designados nos dois sentidos, na preservação do dono fixo e em teste sintético de acesso ao Keychain login padrão entre builds. Um chaveiro descartável selecionado explicitamente MUST NOT ser usado como única qualificação. Builds ad hoc de CI MUST ser identificados como insuficientes para continuidade do cofre.

#### Scenario: Two different builds with the same identity

- **WHEN** dois binários diferentes usam o mesmo identificador e certificado local, conservando o helper fixo
- **THEN** o segundo lê e atualiza os itens sintéticos pelo helper sem UI do Keychain, e outro identificador assinado pelo mesmo certificado não recebe os itens desse namespace

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
