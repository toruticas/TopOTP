## Why

A beta atual funciona, mas concentra o acesso rápido a códigos e todos os formulários em um painel web. O próximo passo é oferecer uma interação própria do macOS, inspirada na lista de Wi-Fi: desbloquear pelo ícone, encontrar/copiar um OTP e acessar tarefas secundárias por “Mais”.

## What Changes

- Substituir a apresentação cotidiana do painel web por um painel AppKit ancorado no ícone, com controles nativos, lista compacta de contas, código/validade e aparência adaptada ao sistema.
- Ao clicar no ícone com o cofre bloqueado, solicitar imediatamente Touch ID/senha do macOS uma única vez; apresentar a lista após sucesso. Cancelamento, expiração, notificações e atualização de códigos não disparam novas tentativas automaticamente.
- Adicionar busca nativa por serviço/conta, navegação por teclado e cópia de TOTP ao acionar uma linha. Preservar geração HOTP explícita e incremento persistido antes de entregar o código.
- Disponibilizar “Mais” no rodapé, abrindo menu nativo com Bloquear, Nova entrada, Escanear QR da tela, Importar, Exportar, Configurações e Sair. Preservar busca/rolagem/seleção em memória ao reabrir antes de 30 segundos na mesma sessão.
- Abrir uma janela de gerenciamento independente, com navegação para contas, cadastro/edição, transferências e configurações. Reaproveitar Vue e os formulários existentes nessa janela; clicar fora dela não a oculta.
- Introduzir leitura de QR por seleção de uma região da tela, conforme a escolha do usuário, com captura nativa e decodificação local. A conta passa por revisão antes da confirmação; imagem e URI secreta não são gravadas em arquivos temporários nem copiadas para o clipboard.
- Reutilizar o core Rust, Keychain tradicional e adaptador estável de LocalAuthentication. Coordenar autorização, visibilidade, bloqueio e respostas assíncronas entre as duas superfícies sem mudar a política de autenticação.
- Corrigir a continuidade de identidade entre builds com um certificado local persistente, sem conta Apple. O usuário escolheu migrar uma única vez por backup criptografado exportado pelo TopOTP estável; o novo cofre usa namespace próprio e preserva integralmente o antigo. Recusas do Keychain encerram a operação sem diálogos próprios de senha nem novas leituras automáticas.
- Ampliar testes de coordenação nativa, busca, HOTP, integração da janela e QR com dados sintéticos, registrando separadamente as verificações reais de hardware/permissões.

## Capabilities

### New Capabilities

- `native-otp-quick-access`: painel AppKit, desbloqueio iniciado pelo clique, busca/lista/cópia, menu Mais e retomada transitória.
- `management-window`: janela independente de gerenciamento, navegação para tarefas existentes e coordenação de sessão/permissões com o painel nativo.
- `screen-qr-enrollment`: seleção de área da tela, captura/decodificação local de QR OTP, revisão e gravação autorizada.
- `local-vault-upgrades`: assinatura local persistente, validação de atualizações e migração inicial por backup com preservação do cofre anterior.

### Modified Capabilities

Nenhuma especificação foi publicada em `openspec/specs/`. A mudança original `create-macos-menu-bar-authenticator` permanece aberta com suas pendências manuais; esta mudança acrescenta os contratos das novas superfícies e substitui sua apresentação cotidiana, preservando os contratos de cofre, OTP e backups já implementados.

## Impact

Afeta `src-tauri/src/lib.rs`, a coordenação de sessão em `application/`, os adaptadores em `platform/macos/`, comandos/capabilities, `src/app/` e testes/documentação. O domínio OTP e o codec de backups claros/v3, inclusive enums numéricos compatíveis, permanecem a referência de comportamento.

AppKit será acessado pelos bindings `objc2` já adotados. A captura e a leitura de QR acrescentam integrações com ScreenCaptureKit e Vision, com dependências selecionadas e fixadas na implementação. Manter macOS 13 como mínimo exige um caminho compatível para captura, sem usar incondicionalmente APIs mais recentes. A permissão de gravação da tela é solicitada somente ao iniciar o scanner.

Não inclui câmera, QR de arquivo, Google migration, novos tipos OTP, sincronização, notarização, conta Apple, alteração de formato dos registros ou publicação de outra versão. A assinatura local e a migração foram acrescentadas após reproduzir a regressão de atualização ad hoc e receber a escolha explícita do usuário. A experiência é inspirada na imagem de referência; não depende de APIs privadas nem promete reproduzir pixel a pixel o menu Wi-Fi do sistema. A confirmação anterior de Touch ID não comprova a nova coordenação de interface.
