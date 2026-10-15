# Verificação nativa e fronteira de segurança

Evidências iniciais: 14/10/2026, macOS 27.0.1 arm64, CLT/SDK local, assinatura ad hoc e sem conta/perfil Apple. A continuidade de assinatura foi reavaliada em 15/10/2026, conforme o registro de migração abaixo.

## Modelo adotado

O usuário escolheu evitar conta Apple. O app usa Keychain tradicional (`kSecUseDataProtectionKeychain: false`), sem sincronização, com ACL padrão restrita ao aplicativo criador. Um `LAContext` novo com `deviceOwnerAuthentication` autoriza a sessão no core antes de ler qualquer credencial. A UI não fornece sucesso de autenticação; recebe somente estado/geração.

**Touch ID protege a sessão do TopOTP. O item do Keychain não tem bloqueio biométrico independente do aplicativo.** O Keychain protege os valores em repouso e verifica a confiança do processo por ACL. Esse modelo não protege contra comprometimento do processo nativo/macOS. A distinção segue a [Apple TN3137](https://developer.apple.com/documentation/technotes/tn3137-on-mac-keychains).

No Mac de build, a chave privada de assinatura foi autorizada para `codesign`. A confiança por assinatura também não protege contra software do mesmo usuário que consiga usar essa chave para assinar outro executável com o identificador do TopOTP. A chave não integra os bundles distribuídos. Não apresentar a autorização de sessão como proteção biométrica independente dos itens, nem como proteção contra uso indevido da identidade de assinatura na máquina de desenvolvimento.

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

## Painel AppKit e scanner em desenvolvimento

Em 15/10/2026, `native-macos-quick-access` adicionou popover AppKit para OTP e janela Tauri `management` para formulários. A beta instalada foi preservada. `auth.rs` permanece igual à base `530bf1c`. Após a regressão, `keychain.rs` passou a impedir UI própria de armazenamento e distinguir recusa de acesso; a migração usa namespace novo. As confirmações humanas acima referem-se à interface anterior.

| Verificação da nova interface                                                                | Evidência                                                                                                 |
| -------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Ancoragem, foco/entrada no NSSearchField, busca sem acentos e Command-F                      | Probe AppKit real com dados sintéticos; aprovado                                                          |
| Menu Mais sem fechamento prematuro; cópia                                                    | Controles AppKit reais; callback de cópia sintético; aprovado                                             |
| Busca/rolagem/seleção/cursor em 29,999 s; descarte em 30 s; códigos limpos ao ocultar        | Probe ajusta somente seu relógio de apresentação; aprovado                                                |
| Vision lendo dois QRs TOTP/HOTP com Unicode                                                  | Framework real, imagem gerada em memória; aprovado, sem captura/permissão                                 |
| Revisão por metadados, escolha única, duplicatas, falha de armazenamento e revogação         | Testes Rust/Vue com adaptadores sintéticos; aprovado                                                      |
| Backups claros/v3, senhas incorretas/múltiplas, enums mistos, exportação/reimportação e HOTP | Quatro fluxos E2E no WKWebView de gerenciamento; aprovado                                                 |
| Clique bloqueado → Touch ID/senha/cancelamento → painel, sem prompts repetidos               | **Touch ID/painel confirmados pelo usuário no harness**; senha alternativa e cancelamento ainda pendentes |
| Captura real, concessão/recusa de permissão, Escape e revisão de QR na tela                  | **Captura/revisão confirmadas pelo usuário no harness**; recusa e Escape reais ainda pendentes            |
| Segundo monitor, VoiceOver, suspensão/bloqueio com duas superfícies                          | **Pendente em hardware**                                                                                  |
| Intel/macOS 13                                                                               | Compilação não equivale a execução; caminho SCStream ainda precisa de teste real                          |

Relatórios em `.artifacts/native-quick-access/`, incluindo baseline de hashes dos bundles preservados e `native-probe.json`. O PNG de `cacheDisplayInRect` captura parcialmente a view e não representa a composição final do WindowServer; não o use como aprovação visual. O probe independente não instancia Core/Keychain/LocalAuthentication nem captura a tela.

Checks finais: 61 testes unitários Rust, interoperabilidade contra o código original, 17 testes Vue, quatro fluxos E2E, typecheck/build, ESLint, Prettier, rustfmt e Clippy aprovados. Bundles e DMGs arm64/x86_64 foram gerados separadamente em `.artifacts/native-quick-access/target/`; assinatura ad hoc, arquitetura, versão, capability e ausência de automação/Node/Chromium aprovadas pelo inspetor. Os dois binários incorporam os assets frontend atuais. `hdiutil verify` e os hashes dos bundles estáveis preservados foram conferidos; checksums dos DMGs em `SHA256SUMS`. Esses builds locais não foram publicados nem usados para autenticar no vault pessoal.

Após os checks, por solicitação explícita do usuário em 15/10/2026, o build arm64 foi instalado em `/Applications/TopOTP.app` e iniciado para teste manual. A instância anterior foi encerrada normalmente; seu bundle foi preservado em `.artifacts/native-quick-access/rollback/20261015-131550/TopOTP.app`. Assinatura e hash do aplicativo instalado foram conferidos, sem manipular os dados do cofre. Recibo em `.artifacts/native-quick-access/installation.json`; resultados humanos continuam pendentes.

O scanner só solicita permissão por ação explícita. Seleção/painel são removidos antes da captura. Em 14+ há uma captura SCScreenshotManager; em 13, um frame completo de SCStream seguido de parada. Limites: uma operação, 16 milhões de pixels, 16 QRs, 8 KiB/payload e 15 s. Imagem/URI não passam pelo IPC. Recursos Apple são liberados, mas suas cópias internas não têm garantia de zeroização física. Payloads mutáveis Rust usam `Zeroizing`.

### Regressão observada e rollback — 15/10/2026

O usuário relatou pedidos insistentes de senha ao testar o build nativo instalado. O teste 7.3 falhou; o candidato foi retirado. O bundle anterior, hash SHA-256 `e8782748…`, foi restaurado em `/Applications` e reaberto pelo caminho original em `src-tauri/target/aarch64-apple-darwin/release/bundle/macos/TopOTP.app`. O rollback não alterou dados/ACLs do cofre. Recibo em `.artifacts/native-quick-access/rollback-after-regression.json`. **Após o rollback, o usuário confirmou que o desbloqueio por Touch ID voltou ao normal: “Sim, voltou ao normal”.** Isso valida a restauração, sem aprovar o candidato retirado.

Comparar somente os fontes de autenticação foi insuficiente. `codesign -d -r-` mostra identidades designadas diferentes: estável `cdhash 66788b685e80553a9e26bdac079543e57f7a088b`; candidato `cdhash c7e990fe81c5e303fc5b7af87a7d97095c9b5bf4`. O candidato falha ao avaliar o requisito do estável, enquanto a cópia restaurada passa. A Apple documenta que o Keychain usa requisitos de assinatura para reconhecer aplicativos/atualizações em [TN2206](https://developer.apple.com/library/archive/technotes/tn2206/). Essa incompatibilidade é um fato; atribuir o diálogo relatado especificamente à ACL é uma hipótese fundamentada, não confirmação do título ou conteúdo do pedido.

Touch ID autoriza a sessão do app, mas não torna outra identidade de código confiável para itens já existentes. Leituras autorizadas de OTP continuam consultando o Keychain; seu acesso interativo pode pedir senha quando a confiança do binário não corresponde. Os mocks de armazenamento e o probe AppKit não exercitaram essa atualização. Não aceitar um teste de assinatura válida ou fontes inalterados como prova de continuidade de acesso.

Para atualizações normais, `scripts/verify-update-identity.py <atual.app> <candidato.app>` deve passar, seguido de teste de atualização com itens sintéticos e identidade estável. O script é somente leitura e não abre o Keychain. A migração inicial ad hoc→certificado é um caminho separado escolhido pelo usuário: não pode passar artificialmente no requisito antigo. A política Touch ID/senha e as ACLs dos itens OTP anteriores permanecem preservadas.

Para o smoke test pendente, use QRs públicos/sintéticos e namespace de cofre exclusivo. Verifique cancelamento sem retry; painel após aprovação; Mais/teclado; retomada e expiração; minimizar/restaurar Configurações; recusa da captura e orientação; captura autorizada e escolha entre dois QRs; bloqueio durante captura/revisão e suspensão/retomada. Registre cada sistema/monitor separadamente. Não substitua o app estável nem use o vault pessoal nesses testes.

### Continuidade e migração — 15/10/2026

`scripts/keychain-update-probe.py` reproduziu a falha em dois binários Swift diferentes, com mesmo identificador e assinaturas ad hoc: leitura do segundo recusada com `-25293`, enquanto o criador tinha acesso. Tudo ocorreu em Keychains descartáveis, com payload público, UI do Keychain desativada e nenhuma leitura do cofre pessoal.

Com certificado local descartável de codeSigning, o segundo build realizou dez leituras e uma atualização; o primeiro leu o valor atualizado, e outro identificador com o mesmo certificado permaneceu recusado. Os Keychains/identidades temporários foram removidos. Relatório `.artifacts/keychain-upgrade-probe/report.json`. Isso comprova a continuidade naquele teste, sem comprovar biometria. O usuário autorizou a nova chave para `codesign` no macOS. A repetição com a identidade persistente efetiva (`--local-identity`) **passou**: dez leituras entre builds, atualização, leitura de retorno e recusa ao identificador estrangeiro (`-25293`). Relatório `.artifacts/keychain-upgrade-probe/persistent-identity-report.json`. Não houve conta Apple, alteração de trust settings ou acesso ao cofre pessoal. Essa autorização de build não é o desbloqueio OTP; a Apple descreve essa separação de ACL/partition list na [documentação do security](https://github.com/apple-oss-distributions/Security/blob/main/SecurityTool/macOS/security.1).

Foi criada a identidade local **TopOTP Local Signing**, sem conta Apple ou alteração de trust settings. A chave fica no Keychain; os metadados públicos fixam seu fingerprint, e material transitório protegido é removido. O wrapper reutiliza a identidade, recusa substituição automática e usa diretório de build informado. Artefatos de CI permanecem explicitamente ad hoc.

O usuário escolheu exportar um **backup criptografado pela UI estável com todas as contas e contadores atuais**. A nova versão mantém o cofre anterior intacto e usa serviço `signed.vault.v1` e diretório `vault-signed-v1`. A rota de migração detecta somente a existência do manifesto antigo; não tenta ler seus seeds. A importação usa os codecs/prévia/commit existentes e orienta não consumir HOTP na versão antiga após exportar. IDs internos são novos; identificadores de origem, metadados, ordem, favoritos e contadores do backup são preservados. Os dois cofres não sincronizam alterações posteriores.

Checks após essas mudanças: **65 testes Rust, um teste de interoperabilidade upstream e 18 testes Vue aprovados**, incluindo exportação criptografada entre dois cofres sintéticos, contadores HOTP atuais e cofre de origem inalterado; recusa de acesso revoga a sessão e vinte ciclos de atualização não repetem leituras. Typecheck/build e lint aprovados. Os quatro fluxos E2E também passaram novamente após a migração. A assinatura efetiva passou; hardware continua registrado separadamente, sem promover mocks a evidência nativa.

O `.app` **TopOTP Teste Nativo** (`native-smoke`, somente debug) exercita o shell completo com LocalAuthentication/Keychain reais e duas contas públicas sintéticas, em identifier/serviço/diretório próprios. Seu build foi assinado com o certificado persistente e iniciado com a indicação **Teste** na barra de menus. O usuário confirmou no teste isolado: **“Touch ID funcionou e a lista ficou aberta sem repetir senha”**. O manifesto exclusivo do harness contém as duas contas sintéticas esperadas, corroborando os commits reais no Keychain após autenticação. Isso valida o caminho nativo do harness, sem declarar concluída a migração pessoal ou os demais testes manuais. Os dois `.app`/`.dmg` de produção foram assinados e inspecionados em `.artifacts/native-quick-access/signed-target/`, com capability/versão/arquitetura/ausência de automação/Node/Chromium aprovadas e `hdiutil verify` aprovado. Nenhum build novo substituiu o estável neste ciclo.

O helper `scripts/native-qr-fixture.swift` exibe dois QRs públicos TOTP/HOTP em uma janela AppKit de teste, sem arquivo de imagem, Keychain ou captura. O usuário executou o fluxo de seleção/captura sobre essa janela e confirmou: **“A revisão do QR apareceu corretamente”**. Isso comprova o caminho de captura/revisão no macOS arm64 atual; não comprova recusa/Escape, segundo monitor ou o caminho SCStream de macOS 13.

### Confirmação do shell nativo com Touch ID

Em 15/10/2026, o usuário confirmou o desbloqueio do **TopOTP Teste Nativo** e a permanência da lista sem pedidos repetidos de senha. Foram usados NativeAuthenticator/Keychain reais, certificado local persistente e serviço exclusivo `.native-smoke.signed.vault.v1`, com duas contas públicas sintéticas. A UI do Keychain tradicional estava desativada, enquanto LocalAuthentication continuou funcionando. A política e o ator de `auth.rs` não foram modificados. Senha alternativa, cancelamento, VoiceOver, suspensão/segundo monitor e captura real seguem separados dessa confirmação.

### Instalação do build com certificado persistente

Após confirmar Touch ID e captura/revisão QR no harness, o usuário informou já possuir backups e dispensou nova exportação: “Pode chutar tudo pro ralo, eu ja tenho backup das coisas”. A instalação foi autorizada com esses backups existentes; seu conteúdo/frescor/contadores não foi lido nem verificado pelo agente.

O build arm64 foi instalado em `/Applications/TopOTP.app`, SHA-256 `5a46add69c2bc8ac04ca3bccf2e21c23b70d5a7891613560ba3fae1bab33da92`, com certificado local SHA-1 `8281259d9ca4fb7dbf1bf1dfe4e5066a0ffa006a`. A assinatura e o hash do destino foram conferidos após a cópia. As instâncias estável/teste/QR foram encerradas normalmente; a instalação anterior foi preservada em `.artifacts/native-quick-access/rollback/before-local-signed-20261015-214853/TopOTP.app`. Nenhum item/manifesto do cofre antigo foi apagado ou reescrito.

Recibo: `.artifacts/native-quick-access/signed-migration-installation.json`. Os DMGs assinados por arquitetura e `SHA256SUMS` ficam em `.artifacts/native-quick-access/signed-target/`. O pré-release publicado não foi alterado. A restauração do backup pessoal pertence ao usuário pela UI; esta instalação não confirma que essa etapa foi concluída, nem promove o teste sintético a teste do cofre pessoal.
