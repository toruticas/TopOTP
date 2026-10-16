## Purpose

Oferecer consulta e cópia rápida de OTPs por uma lista nativa do macOS, com desbloqueio iniciado pelo clique no ícone e ações secundárias acessíveis sem sobrecarregar o painel.

## ADDED Requirements

### Requirement: First use through the normal native panel

Um cofre sem entradas MUST seguir o mesmo fluxo de autenticação/apresentação de um cofre existente: após desbloqueio explícito, mostrar o painel nativo vazio com Mais disponível. A presença de arquivos de versões anteriores MUST NOT abrir uma janela de migração nem restringir as ações. O usuário MUST poder escolher Nova entrada, Importar ou Escanear QR da tela como primeira ação. Nenhuma dessas operações MUST iniciar automaticamente ou criar contas antes de confirmação.

#### Scenario: Open an empty vault with older data preserved

- **WHEN** o usuário desbloqueia um cofre atual vazio, com ou sem manifesto de versão anterior
- **THEN** o painel aparece abaixo do ícone, sem linhas de OTP e sem janela automática, mantendo as ações de Mais disponíveis

#### Scenario: Cancel a first import and choose another action

- **WHEN** o usuário cancela a seleção ou revisão da primeira importação e escolhe Nova entrada ou Escanear QR da tela
- **THEN** a ação escolhida funciona normalmente, sem exigir migração ou adicionar uma conta automaticamente

### Requirement: Native quick access surface

O app MUST iniciar bloqueado, com um único ícone na barra de menus e sem janela de gerenciamento aberta. Acionar o ícone com sessão válida MUST apresentar uma lista de controles nativos imediatamente abaixo dele, limitada à área disponível do monitor correspondente. A aparência MUST acompanhar o tema e as preferências de acessibilidade do sistema; Escape, novo clique no ícone ou clique fora MUST ocultar o painel. Fechá-lo MUST manter o aplicativo em execução.

A altura MUST acompanhar a quantidade de linhas visíveis, inclusive após filtrar, até o máximo atual de 480 pontos. Poucas contas MUST NOT reservar a área de uma lista cheia. Acima do limite, somente a lista MUST rolar, mantendo cabeçalho, busca, Mais, ações expandidas e Configurações acessíveis. Redimensionar MUST preservar ancoragem, foco/cursor e o estado autorizado.

#### Scenario: One account and an overflowing list

- **WHEN** o painel apresenta uma conta e depois uma lista maior que a área disponível
- **THEN** a primeira apresentação é compacta; a segunda para na altura máxima e usa rolagem interna sem mover o cabeçalho/rodapé para fora do painel

#### Scenario: Startup without user interaction

- **WHEN** o app é iniciado sem o usuário acionar seu ícone
- **THEN** permanece bloqueado, sem solicitar autenticação nem exibir nomes ou códigos

#### Scenario: Open an authorized list on another display

- **WHEN** o ícone é acionado com sessão válida em um monitor de escala diferente
- **THEN** o painel aparece abaixo daquele ícone, dentro do monitor e sem primeiro aparecer no centro da tela

### Requirement: One authentication attempt per explicit activation

Acionar o ícone enquanto bloqueado MUST solicitar imediatamente desbloqueio nativo por Touch ID ou senha do macOS, sem exigir um botão intermediário. Somente uma tentativa MUST permanecer ativa; cliques adicionais durante essa tentativa MUST NOT criar outro diálogo. Sucesso MUST apresentar a lista e permitir interação; cancelamento, recusa ou falha MUST manter o cofre bloqueado, sem ler credenciais nem iniciar outra tentativa. Timers, atualização de códigos, foco, retorno de suspensão e eventos de estado MUST NOT solicitar autenticação automaticamente.

#### Scenario: Successful unlock from the icon

- **WHEN** o usuário aciona o ícone bloqueado e conclui a autenticação nativa
- **THEN** a lista é apresentada após o sucesso e permanece interativa até uma ação normal de fechamento ou bloqueio

#### Scenario: Cancel followed by background activity

- **WHEN** o usuário cancela o desbloqueio e depois ocorrem atualizações de estado ou foco
- **THEN** nenhuma nova solicitação aparece e nenhuma conta é revelada; outro clique explícito pode iniciar nova tentativa

#### Scenario: Repeated clicks and stale approval

- **WHEN** o usuário clica repetidamente durante uma autenticação, bloqueia o app e depois chega o sucesso daquela tentativa
- **THEN** houve no máximo um diálogo ativo e o sucesso antigo não desbloqueia nem reabre o painel

### Requirement: Compact account list and deliberate code actions

A lista MUST apresentar serviço, conta e código TOTP com validade textual, respeitando fixação/ordem já existentes. Acionar uma linha TOTP MUST copiar seu código atual e dar confirmação acessível. HOTP MUST oferecer geração explícita, persistir o incremento antes de entregar o código e permitir copiar o último código emitido na sessão. Abrir, filtrar, redesenhar ou reabrir a lista MUST NOT consumir HOTP. Falha de armazenamento MUST ser informada sem apresentar código ou sucesso não confirmado.

#### Scenario: Copy near a TOTP boundary

- **WHEN** o usuário aciona uma conta TOTP após a janela de validade do código anteriormente exibido
- **THEN** o app copia o código atual autorizado e informa a conclusão

#### Scenario: Filter a HOTP account

- **WHEN** uma conta HOTP é encontrada pela busca ou a lista é reaberta
- **THEN** seu contador permanece inalterado até uma ação explícita de geração

#### Scenario: HOTP persistence fails

- **WHEN** o armazenamento recusa o incremento solicitado
- **THEN** o app informa a falha e não entrega o novo código

### Requirement: Native search and keyboard interaction

O painel MUST oferecer campo de busca nativo que filtre serviço e conta enquanto o usuário digita, sem diferenciar maiúsculas/minúsculas ou acentos. MUST permitir focar a busca por teclado, navegar nos resultados, copiar a seleção e acessar Mais com nomes e foco acessíveis. A busca MUST NOT persistir histórico. Busca sem resultado e cofre vazio MUST ter mensagens distintas e manter Mais disponível. Rótulos importados MUST ser tratados como texto.

A busca MUST ficar desativada quando não há contas no cofre e voltar a ficar ativa após o primeiro cadastro/importação. Uma busca sem resultados em um cofre com contas MUST continuar editável. Com busca desativada, o foco MUST poder alcançar Mais e Bloquear.

#### Scenario: Filter and activate from the keyboard

- **WHEN** o usuário digita parte de um serviço ou conta e navega pelos resultados com o teclado
- **THEN** os itens correspondentes aparecem imediatamente e podem ser acionados sem mouse

#### Scenario: Empty result with an imported label

- **WHEN** a busca não encontra uma conta, inclusive após importar rótulos contendo marcação
- **THEN** o app informa ausência de resultados, preserva as ações do rodapé e não interpreta rótulos como conteúdo executável

### Requirement: Native More accordion and fixed Settings footer

Mais MUST expandir/recolher uma seção nativa dentro do mesmo painel, com as ações Nova entrada, Escanear QR da tela, Importar, Exportar e Sair. MUST iniciar recolhido, indicar o estado por chevron e expô-lo à acessibilidade. As ações MUST seguir a ordem visual e ser alcançáveis por teclado somente quando expandidas. Recolher com foco numa ação MUST retornar o foco a Mais. Expandir/recolher MUST NOT fechar o painel, apagar a busca ou consumir HOTP.

Configurações MUST permanecer visível como última opção, fora da seção expansível e da lista rolável, separada das demais ações e disponível com Mais recolhido ou expandido. Nenhuma ação MUST ter reticências. A altura MUST crescer quando houver espaço até 480 pontos; nesse limite, a lista OTP MUST reduzir sua viewport e continuar rolável sem ocultar ações ou Configurações.

Bloquear MUST ficar acessível por um cadeado com nome acessível ao lado do título TopOTP, acima da busca, sem duplicação em Mais. Bloquear e Sair MUST funcionar independentemente da janela web. Sair MUST revogar a sessão e encerrar o processo; as ações de gerenciamento MUST direcionar à seção correspondente de uma única janela.

#### Scenario: Expand and choose an action

- **WHEN** o usuário aciona Mais e escolhe Importar
- **THEN** as ações aparecem no próprio painel e a janela de gerenciamento abre o fluxo de importação após a escolha

#### Scenario: Settings with More collapsed or expanded

- **WHEN** Mais está recolhido ou expandido, inclusive com muitas contas e após rolar a lista
- **THEN** Configurações continua como última opção visível fora da expansão e abre a janela de gerenciamento

#### Scenario: Collapse while an action has focus

- **WHEN** Mais é recolhido com foco em uma das ações internas
- **THEN** o foco volta para Mais e o teclado não alcança ações ocultas

#### Scenario: Lock without a responding management window

- **WHEN** a janela web não responde e o usuário aciona o cadeado no cabeçalho nativo
- **THEN** a sessão é revogada e o painel deixa de revelar contas e códigos

### Requirement: Short lived presentation and shared revocation

Busca, rolagem, seleção, expansão de Mais e foco do painel MUST ser conservados somente em memória por menos de 30 segundos oculto, na mesma sessão autorizada. Códigos MUST ser removidos da apresentação ao ocultar e recalculados após revalidar a sessão na reabertura, sem consumir HOTP. Ao completar 30 segundos oculto, bloquear, expirar ou suspender, o estado de retomada MUST ser descartado. Ocultações repetidas MUST NOT renovar o prazo. Um bloqueio com painel aberto MUST retirar os dados sem iniciar novo prompt.

#### Scenario: Reopen before the deadline

- **WHEN** o usuário reabre em menos de 30 segundos, com a mesma sessão ainda válida
- **THEN** busca, posição, seleção, expansão de Mais e foco são retomados com códigos atuais autorizados

#### Scenario: Deadline or revocation

- **WHEN** passam 30 segundos oculto ou a sessão é revogada enquanto uma atualização está pendente
- **THEN** o estado anterior é descartado, respostas antigas não repovoam o painel e só uma ativação explícita inicia outro desbloqueio
