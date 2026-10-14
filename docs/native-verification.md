# Verificação nativa e fronteira de segurança

Evidências desta implementação: 14/10/2026, macOS 27.0.1 arm64, CLT/SDK local, assinatura ad hoc, zero identidades válidas de codesigning e sem conta/perfil Apple.

## Modelo adotado

O usuário escolheu evitar conta Apple. O app usa Keychain tradicional (`kSecUseDataProtectionKeychain: false`), sem sincronização, com ACL padrão restrita ao aplicativo criador. Um `LAContext` novo com `deviceOwnerAuthentication` autoriza a sessão no core antes de ler qualquer credencial. A UI não fornece sucesso de autenticação; recebe somente estado/geração.

**Touch ID protege a sessão do TopOTP. O item do Keychain não tem bloqueio biométrico independente do aplicativo.** O Keychain protege os valores em repouso e verifica a confiança do processo por ACL. Esse modelo não protege contra comprometimento do processo nativo/macOS. A distinção segue a [Apple TN3137](https://developer.apple.com/documentation/technotes/tn3137-on-mac-keychains).

O modelo anterior com data protection Keychain e `SecAccessControl.userPresence` retornou **-34018 / errSecMissingEntitlement** nos probes Swift e Rust ad hoc. Ele foi substituído explicitamente, sem fallback automático para arquivos ou browser storage.

## Resultados reais

| Verificação                                                                                  | Resultado                                                      |
| -------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Swift: adicionar/ler/remover item sintético no Keychain tradicional                          | Status 0 em todas as operações                                 |
| Swift: outro executável, identidade diferente, sem interação de Keychain                     | Leitura negada, status -25293                                  |
| Rust/objc2: adicionar/ler/atualizar/remover item sintético                                   | Status 0 em todas as operações                                 |
| Preflight LocalAuthentication                                                                | Touch ID e device owner disponíveis; preflight não autentica   |
| `.app` ad hoc: painel inicia bloqueado, interface empacotada `tauri://localhost`             | Observado via UI nativa                                        |
| Touch ID no aplicativo                                                                       | **Usuário confirmou: “Eu consegui desbloquear pelo touch id”** |
| Assinaturas dos bundles arm64/x86_64                                                         | Verificação `codesign --verify --deep --strict` aprovada       |
| Senha do macOS e cancelamento/recusa nativos                                                 | Ainda pendentes                                                |
| Build final: clique no ícone da barra de menus                                               | Usuário confirmou: “Sim, o painel aparece”                     |
| Build `6b4907a`: painel abaixo do ícone e retomada após clicar fora/reabrir em menos de 30 s | Usuário confirmou ambos no macOS: “Sim, os dois funcionam”     |
| Runtime arm64 local                                                                          | Iniciado sem listener TCP; assets empacotados e CSP local      |
| Instância única                                                                              | Segunda execução encaminhou abertura à instância existente     |

Relatórios dos probes em `.artifacts/native-keychain-probe/{report,rust-report}.json`; inspeções de release em `.artifacts/release-inspection-*.json`. Esses arquivos locais contêm somente resultados de diagnóstico e não são versionados. Nenhuma conta real foi usada para gerar fixtures/testes.

## Sessão e sinais

Limites: 120 s sem interação explícita, 300 s totais, 30 s oculto. Polling não renova a sessão. Revogação é anunciada atomicamente antes de aguardar o mutex de commit; resultados antigos não são divulgados. Testes com relógio/adaptadores simulados comprovam os limites, sucesso tardio, bloqueio durante cálculo/importação e concorrência HOTP. Não constituem teste de suspensão física.

O posicionamento passou a usar pontos AppKit da janela/tela do `NSStatusItem`, movendo o painel sincronamente antes de mostrar. A busca anterior recebia pixels Retina e podia não encontrar monitor, deixando a janela centralizada. Testes de geometria cobrem alinhamento abaixo do ícone, limites das bordas e origens negativas; execução real em monitor secundário segue pendente. Testes Vue/Rust comprovam retomada em 29,999 s, descarte em 30 s, preservação de rolagem/foco/formulários/seleções, limpeza de senhas/seeds e revogação antes de revelar a tela.

Observadores públicos registrados: `NSWorkspaceWillSleepNotification`, `NSWorkspaceDidWakeNotification`, `NSWorkspaceScreensDidSleepNotification`, `NSWorkspaceSessionDidResignActiveNotification`. O sinal distribuído `com.apple.screenIsLocked` é uma defesa adicional **não documentada como contrato público da Apple**; não é a única proteção. Sair/Bloquear/timers independem do renderer.

Exportação cria uma avaliação LocalAuthentication nova e consome um recibo nativo privado, além de revalidar a geração antes da gravação. Clipboard usa `NSPasteboard.changeCount`; outra escrita é preservada mesmo com texto igual. Testes de lógica cobrem essas fronteiras; nova autenticação de exportação e clipboard entre aplicativos ainda precisam de smoke test humano.

## Verificações ainda necessárias

- Senha do macOS, cancelamento/recusa e nova autenticação de exportação no `.app`.
- Bloquear o macOS, suspender/retomar e trocar sessão: retorno deve permanecer bloqueado.
- Monitor secundário com DPI distinto; navegação por teclado completa e VoiceOver.
- Clipboard com outro aplicativo e continuidade HOTP após reinício usando Keychain real.
- Confiança da ACL após atualizar assinatura ad hoc e comportamento de Gatekeeper com arquivo efetivamente baixado/quarentenado.
- Execução em Intel e macOS 13. Os bundles foram compilados aqui; isso não comprova funcionamento em outro hardware/sistema.

Não arquivar a mudança como totalmente verificada antes de resolver as tarefas de hardware abertas.
