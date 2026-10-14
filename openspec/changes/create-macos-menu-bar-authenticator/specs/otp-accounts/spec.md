## Purpose

Permitir gerenciar contas e obter códigos TOTP/HOTP corretos e úteis, com validação de parâmetros, preservação do contador e cópia controlada para o clipboard.

## ADDED Requirements

### Requirement: Valid account registration and updates

O app MUST permitir criar, editar e remover contas somente em uma sessão autorizada. O cadastro MUST validar seed Base32 decodificável não vazio, tipo TOTP/HOTP, algoritmo SHA-1/SHA-256/SHA-512, 6–8 dígitos, período TOTP inteiro de 1–86400 segundos e contador HOTP inteiro não negativo até 9007199254740991. Defaults TOTP MUST ser SHA-1, 6 dígitos e 30 segundos. Editar uma conta MUST permitir substituir seu seed sem revelar o seed existente; diminuir o próximo contador HOTP MUST ser rejeitado.

#### Scenario: Minimal TOTP account
- **WHEN** uma conta TOTP com seed válido é cadastrada sem parâmetros opcionais
- **THEN** ela usa SHA-1, 6 dígitos e período de 30 segundos e fica disponível somente após a persistência bem-sucedida

#### Scenario: Invalid or unsupported parameter
- **WHEN** o usuário informa seed inválido, período zero, algoritmo desconhecido ou contador fracionário
- **THEN** o cadastro é rejeitado com erro do campo correspondente e não altera o cofre

#### Scenario: Editing without revealing the existing seed
- **WHEN** o usuário edita o nome de uma conta existente
- **THEN** seus parâmetros públicos podem ser exibidos, o campo de substituição de seed permanece vazio e o seed original é preservado no armazenamento nativo

#### Scenario: Counter rollback attempt
- **WHEN** uma edição solicita contador HOTP inferior ao próximo contador persistido
- **THEN** a edição é rejeitada sem gerar código nem modificar a conta

### Requirement: Correct time-based codes

O app MUST calcular TOTP de acordo com RFC 6238, usando a hora Unix local, os parâmetros da conta e preservando zeros à esquerda. A visualização MUST acompanhar a janela temporal correta e o tempo restante, inclusive após mudança do relógio ou retorno da suspensão; ela MUST permanecer vazia quando bloqueada.

#### Scenario: RFC test vector
- **WHEN** seed, instante, algoritmo e parâmetros correspondem a um vetor público do RFC 6238
- **THEN** o código corresponde exatamente ao vetor, com a quantidade correta de dígitos

#### Scenario: Time boundary or changed wall clock
- **WHEN** a hora cruza o limite de um período ou o relógio é ajustado durante uma sessão válida
- **THEN** o próximo resultado usa a hora atual e o tempo restante do mesmo período do código, sem acumular deriva de um contador visual

### Requirement: Explicit and durable HOTP consumption

O app MUST gerar HOTP conforme RFC 4226 para SHA-1 e manter paridade com a extensão para as variantes SHA-256/SHA-512 suportadas. Cada ação explícita Gerar próximo MUST calcular com o próximo contador persistido e persistir seu incremento antes de liberar o código. Renderização, busca, atualização TOTP e cópia de um HOTP já emitido na sessão MUST NOT consumir outro contador. Operações concorrentes MUST ser serializadas; estouro do limite MUST impedir emissão.

#### Scenario: Successful generation
- **WHEN** Gerar próximo é solicitado com contador persistido igual a 7
- **THEN** o app entrega o código do contador 7 somente depois de persistir 8 e mantém 8 como próximo contador após reiniciar

#### Scenario: Persistence failure
- **WHEN** a gravação do incremento HOTP falha
- **THEN** nenhum novo código é mostrado ou copiado e o próximo contador persistido permanece inalterado

#### Scenario: Concurrent requests
- **WHEN** duas ações Gerar próximo válidas são processadas para a mesma conta
- **THEN** elas usam valores consecutivos distintos e nenhuma reutiliza o contador da outra

#### Scenario: Passive rendering and repeat copy
- **WHEN** a lista é redesenhada ou o HOTP já emitido é copiado novamente
- **THEN** o contador não é alterado; sem um código emitido na sessão, a UI exige Gerar próximo

#### Scenario: Termination after persistence
- **WHEN** o processo termina após persistir o incremento e antes de entregar o código
- **THEN** a próxima execução mantém o incremento, podendo pular aquele valor sem reutilizá-lo

### Requirement: Search and pinned ordering

Após desbloqueio, o app MUST permitir buscar por emissor/conta e fixar contas, preservando sua ordem entre execuções. Enquanto bloqueado, nomes de contas, resultados e códigos MUST permanecer ocultos.

#### Scenario: Pinned account after restart
- **WHEN** uma conta é fixada e o app é reiniciado e desbloqueado
- **THEN** a conta continua fixada e pode ser encontrada por emissor ou nome

### Requirement: Authorized clipboard copy

O app MUST copiar códigos por ação explícita somente com sessão válida. TOTP MUST ser recalculado no instante da cópia; HOTP MUST copiar apenas o último código emitido na sessão. O conteúdo escrito pelo app MUST ser removido em até 30 segundos, ao bloquear ou sair, caso o clipboard ainda pertença àquela escrita. Uma escrita posterior de outro aplicativo MUST ser preservada.

#### Scenario: TOTP expires before clicking copy
- **WHEN** o período muda entre a exibição e o clique Copiar
- **THEN** o clipboard recebe o código TOTP do novo período

#### Scenario: Unchanged clipboard at timeout
- **WHEN** passam 30 segundos sem outra escrita no clipboard
- **THEN** o código copiado pelo app é removido

#### Scenario: Another application replaces the clipboard
- **WHEN** outro aplicativo escreve no clipboard antes da limpeza programada
- **THEN** TopOTP preserva esse novo conteúdo mesmo se ele coincidir textualmente com o código anterior
