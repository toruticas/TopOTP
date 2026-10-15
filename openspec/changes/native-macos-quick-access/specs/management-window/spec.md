## Purpose

Separar tarefas de gerenciamento da consulta rápida de OTPs, oferecendo uma janela macOS independente e uma sessão coerente entre a janela e o painel nativo.

## ADDED Requirements

### Requirement: Independent reusable management window

Configurações MUST abrir uma janela normal do macOS, com título, fechamento e redimensionamento adequados, sem ocultá-la ao clicar fora. A janela MUST fornecer navegação para contas, cadastro/edição e importação/exportação, além de informações da versão e das regras de sessão vigentes. Nova entrada, Importar e Exportar em Mais MUST abrir diretamente o fluxo correspondente na mesma janela. Ações repetidas MUST reutilizar a instância existente. Fechar a janela MUST manter o app na barra de menus.

#### Scenario: Open settings and switch applications

- **WHEN** o usuário abre Configurações e depois clica em outro aplicativo
- **THEN** a janela permanece aberta, sujeita aos limites de sessão, sem se comportar como o painel transitório

#### Scenario: Reuse a window for another action

- **WHEN** a janela já está aberta e o usuário escolhe Nova entrada em Mais
- **THEN** a mesma janela recebe foco e abre o cadastro, sem criar outra instância

### Requirement: One session across presentation surfaces

Painel e janela MUST compartilhar uma única autorização nativa e um único escritor do cofre. Abrir a janela a partir de uma sessão válida MUST NOT pedir desbloqueio novamente. O limite de 30 segundos de sessão oculta MUST começar somente quando nenhuma superfície autorizada estiver visível; o prazo de retomada do painel permanece independente. Limites de 120 segundos sem interação e 300 segundos totais MUST continuar aplicáveis e atualizações automáticas MUST NOT renová-los. Bloqueio, expiração ou suspensão MUST limpar ambas as apresentações, descartar operações antigas e manter o cofre bloqueado até nova ação explícita.

#### Scenario: Handoff from the panel to management

- **WHEN** o painel fecha para abrir uma janela de gerenciamento autorizada
- **THEN** a transição não causa bloqueio por uma falsa ocultação prolongada nem outro diálogo de desbloqueio

#### Scenario: Expire while editing

- **WHEN** a sessão expira durante um formulário ou uma transferência
- **THEN** nomes, códigos, segredos digitados e prévias são retirados, e a janela solicita uma ação explícita para novo desbloqueio

### Requirement: Restricted management operations

Somente a janela local de gerenciamento autorizada MUST poder executar suas operações web; janelas não autorizadas e origens remotas MUST ser recusadas. Listagem, eventos e prévias MUST NOT expor seeds existentes, URIs secretas, senhas ou backups descriptografados. Operações sensíveis MUST validar a sessão novamente após esperas e antes de liberar resultado ou confirmar efeitos. Ocultar/minimizar a janela MUST limpar campos secretos; fechar ou bloquear MUST descartar prévias pendentes.

#### Scenario: Unauthorized client

- **WHEN** uma janela ou origem não autorizada tenta listar contas ou solicitar uma transferência
- **THEN** o pedido é recusado sem leitura de credenciais

#### Scenario: Lock during a delayed operation

- **WHEN** o app é bloqueado antes de retornar uma prévia ou concluir uma autorização pendente
- **THEN** a resposta antiga não revela dados nem restabelece a sessão em qualquer superfície

### Requirement: Existing account and backup behavior

A janela MUST preservar cadastro manual/URI, edição com seed opcional, exclusão confirmada, fixação/ordem e importação/exportação de backups Authenticator claros/v3. A importação MUST manter prévia, senhas transitórias, escolha de entradas, mensagens de erro e política de duplicatas. Exportar MUST continuar exigindo autenticação nativa nova, escolha de destino e autorização vigente antes da gravação. Falhas MUST manter o estado previamente confirmado e não declarar sucesso.

#### Scenario: Unsupported or invalid backup entries

- **WHEN** um backup contém entradas inválidas ou não suportadas
- **THEN** a prévia identifica essas entradas e exige a confirmação aplicável para importar somente o subconjunto válido

#### Scenario: Cancel export authentication

- **WHEN** o usuário cancela ou recusa a autenticação adicional de exportação
- **THEN** nenhum backup é gravado e não ocorre nova tentativa automática

#### Scenario: Storage fails during account management

- **WHEN** uma gravação de conta ou importação falha no armazenamento
- **THEN** o app informa a falha, preserva o conjunto confirmado conforme a recuperação existente e não apresenta conclusão indevida
