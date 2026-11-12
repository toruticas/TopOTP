# Desenvolvimento

TopOTP usa Vue 3/TypeScript/Vite no WebKit do macOS e Tauri 2/Rust no core. Não há backend remoto nem Chromium/Node no aplicativo empacotado.

## Pré-requisitos

- macOS, Xcode Command Line Tools/SDK instalado com `xcode-select --install`.
- Node **24.21.0**, npm **11.19.0**, Rust **1.99.0**. `.nvmrc`, `engines`, `rust-toolchain.toml` e ambos os lockfiles fixam o ambiente.
- Nenhuma conta Apple, certificado Developer ID ou perfil de provisionamento para build local. A assinatura ad hoc é feita pelo bundler.

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
- `compatibility`: URI e formato Authenticator v3; CBC/MD5 existem apenas no envelope de interoperabilidade.
- `commands`: IPC específico, autorização da janela/origem e diálogos nativos.
- `src/features`: formulários; `src/lib/ipc`: contratos/cliente; `src/app`: apresentação transitória e descarte de respostas antigas.

A suíte cobre RFC 4226/6238, mudanças de relógio, defaults/URIs, fixtures cruzadas, recuperação, falhas de gravação, concorrência HOTP e revogação durante trabalho. `test:interop` executa importação/exportação/OTP originais da extensão contra o Rust, nos dois formatos e sentidos, sem rede; faz parte de `test:rust` e requer as dependências npm instaladas. Vitest verifica estados e limpeza de formulários. WebdriverIO dirige o WKWebView real usando o provider embedded: importação com/sem senha, múltiplas chaves, senha incorreta/cancelamento, exportação clara/v3, reimportação e duplicatas com HOTP mais recente. Exportações do app também passam pelo importador original.

As cinco correções da revisão preparadas para `v0.0.1` passaram em 66 testes: 45 Rust (44 unitários e 1 de interoperabilidade), 17 frontend e 4 E2E no WKWebView com adaptadores sintéticos. A identidade da cópia foi exercitada em pasteboard nativo isolado; os testes de exclusão com falha usam armazenamento simulado. Typecheck, ESLint, Prettier, Clippy, rustfmt e validação OpenSpec estrita também passaram. Essas evidências correspondem ao código revisado, sem afirmar build ou publicação dos artefatos `v0.0.1`.

O build E2E vive em `.artifacts/e2e-target`, tem identificador `.e2e` e usa adaptadores sintéticos de autenticação, armazenamento, arquivo e clipboard. O domínio, IPC, commits e codecs são os reais. Não acessa o Keychain do usuário. A feature E2E é proibida em release por `compile_error!`; nenhum parâmetro de ambiente habilita mocks no build normal. Esses testes não comprovam biometria, observação de bloqueio do sistema ou VoiceOver.

O harness escolhe o arquivo sintético copiando-o para `.artifacts/e2e/import-source.json`; somente o build E2E lê esse caminho fixo. Exportações de teste ficam no mesmo diretório. Não use arquivos pessoais nesses testes.

O serviço Tauri requer WebdriverIO 9; versões de globals/webdriverio são unificadas por overrides para evitar instâncias de estado distintas. `basic-ftp` e `serialize-javascript` têm overrides corrigidos. O audit de dependências de produção está limpo. Ainda há alertas transitivos de desenvolvimento em `extract-zip`/`braces`, sem versão corrigida disponível na consulta; o provider embedded não baixa drivers/arquivos ZIP. Não habilite providers que baixem binários nem use arquivos de terceiros nesses componentes sem revisar os alertas.

## Builds por arquitetura

```sh
rtk proxy rustup target add aarch64-apple-darwin x86_64-apple-darwin
rtk proxy npm run tauri:build -- --target aarch64-apple-darwin --ci
rtk proxy npm run tauri:build -- --target x86_64-apple-darwin --ci
```

O wrapper cria o `.app` ad hoc e empacota o `.dmg` por `hdiutil`, sem automação do Finder. O script DMG padrão do Tauri falhou nesta máquina; o empacotamento direto foi executado com sucesso. O CLI sinaliza depreciação de `hdiutil create` no macOS 27; permanece disponível e é compatível com os sistemas de desenvolvimento anteriores.

Saídas: `src-tauri/target/<target>/release/bundle/{macos,dmg}`. Manifests também são separados: subdiretório `dev` para debug e `vault` para produção no diretório de dados do app, além dos serviços Keychain distintos. `--debug --bundles app` cria somente o `.app` de debug. `scripts/inspect-release.py <app>` verifica assinatura, arquitetura, deployment target e ausência de componentes de testes/Node/Chromium. CI em `.github/workflows/ci.yml` roda verificações sem segredos Apple; os pacotes de release ficam no workflow manual após o merge. Na configuração inicial da CI, a [primeira execução](https://github.com/toruticas/TopOTP/actions/runs/38589241973) passou no job de checks, mas a inspeção de bundle falhou porque o helper chamava RTK, ausente no runner. O helper foi corrigido para chamar as ferramentas macOS diretamente e passou na validação local.

A versão do código é `0.0.1`, primeira versão estável solicitada, com tag de release `v0.0.1`. A beta histórica `v0.0.1-beta.1` continua publicada. Alinhe `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` e `src-tauri/tauri.conf.json` ao versionar; `node scripts/check-release-version.mjs` verifica esses arquivos na CI. `package-dmg.py` lê a versão do `Info.plist` do `.app`, e `inspect-release.py` confere a correspondência com os manifests.

Para esta entrega, abra primeiro o PR para `main`, incluindo as [notas de `v0.0.1`](releases/v0.0.1.md), manifests e correções validadas. O usuário fará o merge. Após o merge, uma execução manual controlada no GitHub deve gerar e inspecionar os builds Apple Silicon/Intel a partir do commit da `main`, calcular `SHA256SUMS` e publicar os três assets na versão estável `v0.0.1`. Os nomes esperados são `TopOTP_0.0.1_aarch64.dmg`, `TopOTP_0.0.1_x64.dmg` e `SHA256SUMS`. Não gere artefatos de publicação antes desse merge nem versione binários no Git. Registre o commit e a execução usados na publicação; os testes anteriores não substituem a inspeção dos novos bundles. As demais notas em `docs/releases/` preservam o histórico das versões.

O workflow [Publish macOS release](../.github/workflows/release.yml) aceita somente execução manual (`workflow_dispatch`), sem inputs, e exige `refs/heads/main`. O checkout usa o SHA imutável da execução (`github.sha`). Primeiro ele reutiliza as verificações de `ci.yml`; depois executa a matriz Apple Silicon/Intel e o job que publica DMGs e `SHA256SUMS`. A CI do PR executa as verificações sem gerar pacotes de release. Após o merge, acione a publicação com:

```sh
rtk proxy gh workflow run release.yml --repo toruticas/TopOTP --ref main
```

## Segurança nativa

Use `npm run probe:keychain` para o probe Swift e `python3 scripts/native-keychain-probe.py --rust` para o adaptador Rust. Eles criam e removem somente um item sintético em namespace exclusivo. Relatórios ficam em `.artifacts/native-keychain-probe`. `--data-protection` reproduz o modelo antigo bloqueado por entitlement; não é o modelo de produção. Consulte [verificação nativa](native-verification.md) antes de afirmar cobertura de hardware.
