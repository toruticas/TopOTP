## Purpose

Oferecer acesso rápido às contas de autenticação na barra de menus do macOS, mantendo um ciclo de vida previsível e controles explícitos de privacidade e encerramento.

## ADDED Requirements

### Requirement: Menu bar lifecycle

O aplicativo MUST iniciar bloqueado, com um único ícone na barra de menus e sem janela principal visível ou ícone no Dock. Fechar o painel MUST manter o processo e o ícone ativos; apenas a ação Sair encerra o aplicativo.

#### Scenario: Startup and panel close
- **WHEN** o usuário inicia o app, abre o painel e depois o fecha
- **THEN** o painel inicialmente exige desbloqueio e o app permanece acessível pela barra de menus após o fechamento

#### Scenario: Explicit quit
- **WHEN** o usuário escolhe Sair no menu nativo
- **THEN** o app invalida a sessão, limpa os dados transitórios e encerra o processo e seu ícone

### Requirement: Panel positioning and interaction

O app MUST abrir ou ocultar seu painel por clique no ícone, posicionando-o junto ao ícone dentro da área visível do monitor correspondente. Escape ou clique fora MUST ocultar o painel, respeitando o fluxo de uma autenticação nativa em andamento.

#### Scenario: Icon on a secondary display
- **WHEN** o ícone é acionado em um monitor secundário com escala diferente
- **THEN** o painel recebe foco e aparece completamente dentro da área disponível daquele monitor

#### Scenario: Authentication dialog changes focus
- **WHEN** o macOS exibe o diálogo de autenticação após uma ação explícita
- **THEN** a perda de foco causada por esse diálogo não encerra prematuramente o fluxo, e seu cancelamento mantém o cofre bloqueado

### Requirement: Single application instance

O app MUST manter uma única instância capaz de acessar o cofre e modificar contadores. Uma segunda abertura MUST direcionar o usuário à instância existente sem criar outro ícone nem outro escritor.

#### Scenario: Application is launched twice
- **WHEN** o usuário abre o aplicativo enquanto ele já está em execução
- **THEN** a instância existente apresenta seu painel e não surge um segundo processo escritor ou ícone

### Requirement: Native security actions

O menu nativo MUST oferecer Abrir, Bloquear e Sair. Bloquear MUST permanecer disponível sem depender de o frontend estar respondendo.

#### Scenario: Frontend is unresponsive
- **WHEN** a UI deixa de responder e o usuário escolhe Bloquear no menu nativo
- **THEN** a autorização nativa é revogada e operações sensíveis posteriores são negadas

### Requirement: Offline and accessible interface

O app MUST executar listagem e geração OTP sem conexão de rede, usando recursos locais. O painel MUST permitir navegação por teclado, anunciar controles e estado bloqueado a tecnologias assistivas, e apresentar conta, código e tempo restante sem depender apenas de cores.

#### Scenario: Offline use
- **WHEN** a conexão de rede está indisponível e o cofre é desbloqueado
- **THEN** contas locais e códigos continuam disponíveis sem requisição a servidor

#### Scenario: Keyboard-only operation
- **WHEN** o usuário navega pelo painel somente com teclado
- **THEN** consegue buscar uma conta, copiar TOTP, solicitar HOTP e bloquear o cofre, com foco visível e nomes acessíveis
