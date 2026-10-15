## Purpose

Permitir cadastrar uma conta OTP a partir de um QR em uma região da tela escolhida pelo usuário, com processamento local, revisão e preservação das fronteiras do cofre.

## ADDED Requirements

### Requirement: Explicit authorized region capture

Escanear QR da tela MUST iniciar seleção de uma região somente após ação explícita e sessão válida. O app MUST solicitar a permissão de captura do macOS quando necessária e informar sua ausência sem repetir prompts automaticamente. A seleção MUST permitir cancelar por Escape, funcionar com monitores Retina e de escalas/origens distintas e remover o próprio painel da região capturada. A região MUST ficar limitada ao monitor em que o arraste começou. O app MUST NOT capturar periodicamente ou iniciar leitura de QR em segundo plano.

#### Scenario: Select a region on a secondary display

- **WHEN** o usuário escolhe uma região em um monitor secundário com escala distinta
- **THEN** a imagem analisada corresponde à região escolhida naquele monitor, sem incluir a interface de seleção

#### Scenario: Denied permission or cancelled selection

- **WHEN** a permissão é recusada ou o usuário cancela a seleção
- **THEN** nenhuma conta é criada, nenhuma imagem é persistida e uma nova captura depende de outra ação explícita

### Requirement: Local ephemeral image processing

Captura e decodificação MUST ocorrer localmente, com imagem e conteúdo secreto apenas transitoriamente em memória. O app MUST NOT salvar a captura em disco, clipboard, logs, storage web ou serviços externos. A interface de revisão MUST receber somente metadados permitidos e identificadores de prévia, sem imagem, seed ou URI secreta. Recursos de captura e buffers controlados pelo app MUST ser liberados após conclusão, erro, cancelamento ou revogação.

#### Scenario: Process a synthetic QR offline

- **WHEN** uma região com QR OTP é capturada sem conexão de rede
- **THEN** a conta pode ser revisada sem arquivo temporário, escrita no clipboard ou requisição externa

### Requirement: Supported QR validation and clear outcomes

A leitura MUST aceitar somente URIs OTP TOTP/HOTP com os tipos, algoritmos e parâmetros já suportados pelo app. QR ausente, payload inválido, protocolo não suportado e limites excedidos MUST resultar em mensagens distintas e recuperáveis, sem abrir links externos ou alterar o cofre. Se houver múltiplos QRs válidos, o app MUST permitir escolher uma conta por seus metadados antes da confirmação, sem importar todas automaticamente.

#### Scenario: Valid HOTP QR

- **WHEN** o QR contém uma URI HOTP suportada com contador obrigatório
- **THEN** a prévia preserva o próximo contador e nenhum código HOTP é consumido pela leitura

#### Scenario: Unsupported URI or malformed parameters

- **WHEN** o QR contém um endereço web, Google migration ou parâmetros OTP inválidos
- **THEN** o app informa o motivo compatível, não abre a URI e não altera o cofre

#### Scenario: More than one valid code

- **WHEN** a região contém vários QRs OTP válidos
- **THEN** o app apresenta escolha por metadados e cadastra somente a conta escolhida após confirmação

### Requirement: Reviewed enrollment and safe persistence

Um QR válido MUST abrir revisão na janela de gerenciamento com serviço, conta, tipo e parâmetros não secretos. Somente confirmação explícita em sessão vigente MUST persistir a conta no Keychain. Duplicatas MUST manter a conta existente por padrão e MUST NOT reduzir contador HOTP. Cancelar MUST descartar a prévia; falha de persistência MUST informar erro sem declarar cadastro concluído.

#### Scenario: Review then confirm

- **WHEN** o usuário revisa uma conta válida e confirma com sessão vigente
- **THEN** o cadastro usa a persistência existente e aparece nas duas superfícies somente após o commit

#### Scenario: Existing account or failed storage

- **WHEN** a conta já existe ou sua persistência é recusada
- **THEN** a duplicata preserva o registro existente, ou a falha informa que o cadastro não foi concluído, sem reduzir contadores

### Requirement: Revocation across capture and review

A operação MUST revalidar a autorização após permissão, seleção, captura e decodificação, e novamente antes do commit. Bloqueio, suspensão ou expiração MUST descartar resultados e prévias antigos. Conclusões tardias MUST NOT abrir a janela com dados, salvar a conta ou iniciar outro desbloqueio automaticamente.

#### Scenario: Lock while decoding

- **WHEN** o cofre é bloqueado durante a leitura da imagem e depois chega um QR válido
- **THEN** o resultado é descartado e não reabre a revisão nem salva dados
