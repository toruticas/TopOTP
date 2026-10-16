# Desenvolvimento

TopOTP usa AppKit no acesso rápido, Vue 3/TypeScript/Vite no WebKit para gerenciamento e Tauri 2/Rust no core. Não há backend remoto nem Chromium/Node no aplicativo empacotado.

## Pré-requisitos

- macOS, Xcode Command Line Tools/SDK instalado com `xcode-select --install`.
- Node **24.21.0**, npm **11.19.0**, Rust **1.99.0**. `.nvmrc`, `engines`, `rust-toolchain.toml` e ambos os lockfiles fixam o ambiente.
- Nenhuma conta Apple, certificado Developer ID ou perfil de provisionamento para build local. O wrapper cria/reutiliza um certificado local persistente no Keychain.

Nesta máquina foi usado macOS 27.0.1/arm64 com CLT; macOS 13 é o deployment target declarado, ainda sem execução nesse sistema.

## Comandos

```sh
rtk proxy npm ci
rtk proxy npm run tauri:dev
rtk proxy npm run typecheck
rtk proxy npm run lint
rtk proxy npm run format:check
rtk proxy npm run test
rtk proxy npm run test:rust
rtk proxy npm run test:interop
rtk proxy npm run check:rust
rtk proxy npm run format:rust
rtk proxy npm run test:e2e
rtk proxy npm run tauri:build
```

`nvm` é uma função shell: use `nvm use` na sua sessão antes dos comandos, ou ative a versão com `rtk proxy zsh -lc 'source ~/.nvm/nvm.sh && nvm use'` na sessão correspondente. O Node global não foi substituído. `npm run dev` abre somente a interface de desenvolvimento; Keychain e autenticação requerem `tauri:dev` ou o `.app`.

`npm run bindings` regenera `src/lib/ipc/generated.ts` a partir dos DTOs Rust. O teste Rust compara o arquivo gerado e falha em contratos desatualizados. Não edite esse arquivo manualmente. `npm run licenses` atualiza as atribuições de dependências empacotadas após mudanças de lockfiles. `npm run fixtures` regenera somente fixtures públicas/sintéticas usando CryptoJS e Argon2 de referência; não use contas reais.

## Camadas e testes

- `src-tauri/src/domain`: conta/segredo e OTP, sem dependência de Tauri/frameworks Apple.
- `application`: sessão, coordenador, cofre, commits e transferências; adaptadores injetados pelas interfaces de `ports`/`vault`.
- `platform/macos`: Security, LocalAuthentication, clipboard e observadores nativos.
- `platform/macos/vault_helper`: Store de produção via socketpair privado, com pin/validação do filho. `scripts/vault-helper-v1.swift` é o dono fixo dos itens: valida o audit token/certificado do pai e oferece somente CRUD por UUID, até 32 KiB, sem logs/arquivos/endpoint nomeado.
- `platform/macos/quick_access`: popover, busca, linhas e menu AppKit, confinados à thread principal. `shell.rs` coordena intenção, visibilidade agregada e rotas pendentes da janela `management`.
- `platform/macos/scan`: seleção por monitor e captura regional; ScreenCaptureKit usa SCScreenshotManager em 14+ e um frame completo de SCStream em 13. `sourceRect` está em pontos; width/height de saída, em pixels. Vision lê apenas QR, fora da thread principal.
- `compatibility`: URI e formato Authenticator v3; CBC/MD5 existem apenas no envelope de interoperabilidade.
- `commands`: IPC específico, autorização da janela/origem e diálogos nativos.
- `src/features`: formulários; `src/lib/ipc`: contratos/cliente; `src/app`: apresentação transitória e descarte de respostas antigas.

A suíte cobre RFC 4226/6238, mudanças de relógio, defaults/URIs, fixtures cruzadas, recuperação, falhas de gravação, concorrência HOTP e revogação durante trabalho. `test:interop` executa importação/exportação/OTP originais da extensão contra o Rust, nos dois formatos e sentidos, sem rede; faz parte de `test:rust` e requer as dependências npm instaladas. Vitest verifica estados e limpeza de formulários. WebdriverIO dirige o WKWebView real usando o provider embedded: importação com/sem senha, múltiplas chaves, senha incorreta/cancelamento, exportação clara/v3, reimportação e duplicatas com HOTP mais recente. Exportações do app também passam pelo importador original.

O build E2E vive em `.artifacts/e2e-target`, tem identificador `.e2e` e usa adaptadores sintéticos de autenticação, armazenamento, arquivo e clipboard. O domínio, IPC, commits e codecs são os reais. Não acessa o Keychain do usuário. A feature E2E é proibida em release por `compile_error!`; nenhum parâmetro de ambiente habilita mocks no build normal. Esses testes não comprovam biometria, observação de bloqueio do sistema ou VoiceOver.

`rtk proxy npm run test:e2e:keychain` é a variante local com **Store/helper e Keychain reais**, assinada com a identidade persistente no namespace `.e2e.broker.v1`. Auth, clipboard e diálogos continuam sintéticos; todas as contas são públicas. Limpa apenas referências desse namespace de testes. `rtk proxy python3 scripts/vault-helper-upgrade-probe.py` reproduz o problema direto no login e comprova dois clientes diferentes/rollback/isolações no helper fixo. Ambos exigem macOS e a identidade local já autorizada para codesign; não leem valores do cofre pessoal.

O preparador cria o helper uma vez por arquitetura e recusa fonte, certificado, bytes ou CDHash divergentes. Preserve `~/Library/Application Support/TopOTP Build/vault-helper-v1/` junto da identidade de assinatura. Não apague o cache para destravar builds: a falta do dono de um cofre já criado exige recuperação do artefato, não regeneração. Um novo protocolo/helper ou uma troca de arquitetura exige planejamento e transferência explícitos. O wrapper passa pins na compilação e inclui helper/metadados em Resources; o inspetor exige que os pins, a arquitetura e o CDHash permaneçam iguais após assinar o bundle. Builds fora desse fluxo não habilitam o cofre de produção; CI ad hoc continua somente para compilação/inspeção, sem fallback direto.

O harness escolhe o arquivo sintético copiando-o para `.artifacts/e2e/import-source.json`; somente o build E2E lê esse caminho fixo. Exportações de teste ficam no mesmo diretório. Não use arquivos pessoais nesses testes.

O fluxo E2E também cancela/confirma a remoção pela tela Geral, verifica cofre vazio e recusa à geração antiga. O probe `scripts/management-window-probe.swift` aceita somente o executável isolado em `.artifacts/e2e-target/debug/topotp`: ativa esse processo, passa o foco ao Finder e verifica que a mesma janela permanece na lista de janelas visíveis do macOS. O resultado fica em `.artifacts/e2e/management-window-focus.json`; nenhuma imagem ou conteúdo de outro aplicativo é lido. Requer Finder disponível e uma sessão gráfica. A autenticação/armazenamento desse fluxo continuam sintéticos.

`rtk proxy cargo run --manifest-path src-tauri/Cargo.toml --features native-ui-probe --bin quick-access-probe` executa o probe AppKit independente com dados sintéticos e encerra sozinho. Verifica ancoragem, teclado, busca com acentos, accordion Mais, Configurações fixa fora da expansão, alturas/rolagem, callbacks, ocultação de códigos, Command-F e retomada/expiração com o relógio de apresentação ajustado no harness. O JSON fica em `.artifacts/native-quick-access/native-probe.json`; a captura de view é parcial e não comprova a composição visual do WindowServer. Não execute simultaneamente com outro teste GUI, pois foco é global. Essa feature também é proibida em release e o executável não abre Core/Keychain/LocalAuthentication.

Testes unitários Vision criam imagens em memória via `qrcodegen`, somente dev-dependency. Leem TOTP/HOTP e Unicode usando o framework real, sem capturar a tela ou solicitar permissão. Testes do core cobrem revisão sem seed/URI, escolha única, duplicatas, falha de armazenamento e revogação; testes do capturador cobrem decisões de permissão e timeout/cancelamento/resultados revogados. Isso não substitui captura real, macOS 13 ou segundo monitor. O harness WKWebView dirige `management`, sem automatizar AppKit.

O serviço Tauri requer WebdriverIO 9; versões de globals/webdriverio são unificadas por overrides para evitar instâncias de estado distintas. `basic-ftp` e `serialize-javascript` têm overrides corrigidos. O audit de dependências de produção está limpo. Ainda há alertas transitivos de desenvolvimento em `extract-zip`/`braces`, sem versão corrigida disponível na consulta; o provider embedded não baixa drivers/arquivos ZIP. Não habilite providers que baixem binários nem use arquivos de terceiros nesses componentes sem revisar os alertas.

## Builds por arquitetura

```sh
rtk proxy rustup target add aarch64-apple-darwin x86_64-apple-darwin
rtk proxy npm run tauri:build -- --target aarch64-apple-darwin --ci
rtk proxy npm run tauri:build -- --target x86_64-apple-darwin --ci
```

O wrapper cria o `.app` com uma identidade local persistente e empacota o `.dmg` por `hdiutil`, sem automação do Finder. Na primeira preparação, o macOS pode exigir autorização para `codesign` usar a chave **TopOTP Local Signing**; digite a senha somente no diálogo do sistema. Isso não é autenticação do cofre OTP. Builds seguintes reutilizam o certificado; não recrie a identidade se o Keychain estiver indisponível. Para CI/artefatos ad hoc, use explicitamente `TOPOTP_ADHOC_BUILD=1`; esses artefatos não preservam confiança entre builds. O CLI sinaliza depreciação de `hdiutil create` no macOS 27; permanece disponível.

Saídas: `src-tauri/target/<target>/release/bundle/{macos,dmg}`, ou o diretório `CARGO_TARGET_DIR` informado. O debug de desenvolvimento usa `dev-signed-v1`; produção usa `vault-broker-v1` com o helper fixo. Os diretórios anteriores `dev`, `vault` e `vault-signed-v1` permanecem preservados. `--debug --bundles app` cria somente o `.app` de debug. `scripts/inspect-release.py <app>` verifica assinatura, arquitetura, deployment target, helper/pins e ausência de componentes de testes/Node/Chromium. CI roda checks e gera artefatos ad hoc sem chaves Apple ou locais persistentes; eles não habilitam o cofre.

Para preservar o `.app` estável, informe `CARGO_TARGET_DIR` absoluto sob `.artifacts/` ao wrapper `npm run tauri:build`. O wrapper prepara os pins e recursos do helper mesmo com `--no-bundle` e grava `topotp-bundle-config.json` no target. Se empacotar separadamente com `tauri bundle`, reutilize esse arquivo por `--config`; omiti-lo produz um bundle sem o dono fixo. Empacote com `scripts/package-dmg.py <app>`. O inspetor aceita um segundo argumento de diretório para guardar os relatórios junto desses artefatos. Não inicie um bundle de produção com o vault pessoal para testar scanner; use o harness sintético. Não publique nova versão/tag automaticamente.

Antes de instalar/abrir uma atualização sobre o cofre existente, execute `rtk proxy python3 scripts/verify-update-identity.py <app-atual> <candidato>`. Ele compara requisitos designados nos dois sentidos e o dono estável, sem abrir o Keychain. Para o modelo direto, exige o mesmo CDHash do executável; para o novo, exige helper, certificado, arquitetura e pins preservados. A passagem direto → helper retorna `migrationRequired`, mesmo com certificado igual. Falha bloqueia a instalação normal: não resolver por ACL irrestrita ou prompts repetidos. Mesmo com preflight aprovado, testar atualização no login padrão com itens sintéticos é necessário; o preflight não atesta ACLs existentes nem substitui biometria real. Os diagnósticos e rollbacks estão no guia de verificação nativa.

A migração ad hoc→certificado é separada e foi escolhida explicitamente pelo usuário: exportar backup criptografado pela versão estável, instalar o candidato validado e importar pela UI em um cofre novo. Não tentar passar artificialmente no requisito antigo, modificar sua ACL ou usar seus seeds nos testes. O arquivo precisa conter todas as contas e os contadores atuais; não gerar HOTP no app anterior depois da exportação. O cofre antigo não sincroniza os incrementos da nova versão.

`rtk proxy python3 scripts/keychain-update-probe.py` reproduz a recusa ad hoc e testa assinatura com identidade descartável, usando só Keychains de teste. `--local-identity` repete com o certificado efetivo do build. Esse probe não qualifica o login padrão: seus resultados positivos não detectaram a partição por CDHash que causou a regressão de 16/10. Para continuidade do cofre, use `scripts/vault-helper-upgrade-probe.py`, que reproduz a recusa direta e testa o helper no login real com contas sintéticas. Nenhum dos probes testa Touch ID.

Para o teste do shell completo com autenticação/Keychain reais, compile **somente debug** com `--features native-smoke --config src-tauri/tauri.native-smoke.conf.json`. O `.app` se chama TopOTP Teste Nativo, tem identifier e serviço próprios, e cria apenas duas contas públicas sintéticas após desbloqueio. Compile/bundle em artefatos isolados e assine com a identidade persistente. Não execute junto com E2E ou probe AppKit, pois foco/diálogos pertencem à sessão macOS. A feature é proibida em release.

A versão atual é `0.0.1-beta.1`, com tag `v0.0.1-beta.1`; a estável `v0.0.1` será publicada após as validações mínimas. Alinhe `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` e `src-tauri/tauri.conf.json` ao versionar. `package-dmg.py` lê a versão do `Info.plist` do `.app`, e `inspect-release.py` confere a correspondência com os manifests. Publique os pacotes por arquitetura e `SHA256SUMS` como assets de um pré-release, associado ao commit da `main`; não versionar binários no Git. As notas de cada versão ficam em `docs/releases/`.

## Segurança nativa

Use `npm run probe:keychain` para o probe Swift e `python3 scripts/native-keychain-probe.py --rust` para o adaptador Rust. Eles criam e removem somente um item sintético em namespace exclusivo. Relatórios ficam em `.artifacts/native-keychain-probe`. `--data-protection` reproduz o modelo antigo bloqueado por entitlement; não é o modelo de produção. Consulte [verificação nativa](native-verification.md) antes de afirmar cobertura de hardware.
