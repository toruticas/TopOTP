# TopOTP — orientações para agentes

## Objetivo e estado atual

TopOTP será um autenticador TOTP/HOTP para a barra de menus do macOS, com interface web embarcada em Tauri 2 e segurança nativa em Rust. A referência funcional e de interoperabilidade é [Authenticator-Extension/Authenticator](https://github.com/Authenticator-Extension/Authenticator).

O aplicativo está implementado, com cofre, OTP e transferências pela interface. A mudança ativa `native-macos-quick-access` substitui o painel web por AppKit e mantém os formulários na janela `management`; adiciona captura regional com ScreenCaptureKit/Vision. Touch ID foi confirmado pelo usuário na beta anterior e no novo shell `native-smoke` com autenticação/Keychain reais e contas sintéticas: painel permaneceu aberto, sem repetir pedidos de senha. O usuário também confirmou a revisão de QR capturado da tela no harness. A importação pessoal ainda pertence ao usuário pela UI. As mudanças permanecem abertas para as verificações manuais descritas em `docs/native-verification.md`; não transforme mocks/preflight em evidência de hardware.

Leia nesta ordem:

1. `README.md` e este arquivo.
2. `openspec/config.yaml`.
3. `proposal.md`, `design.md`, `specs/**/spec.md` e `tasks.md` da mudança ativa, identificada por `rtk proxy openspec list --json`.
4. `docs/upstream-compatibility.md` antes de alterar importação/exportação.

## Fluxo de trabalho

- Preserve o fluxo OpenSpec: propor → implementar → validar → arquivar. Uma solicitação de proposta gera documentação e configuração de planejamento; a implementação começa com `/opsx:apply`, `$openspec-apply-change` ou pedido explícito equivalente.
- Use as skills OpenSpec locais em `.agents/skills/`. Não execute `openspec init` novamente em um projeto já inicializado; esse comando pode migrar/remover atalhos globais.
- Use os caminhos retornados por `openspec status` e `openspec instructions`. Não assuma que uma mudança está no diretório local caso o CLI indique uma store.
- Execute comandos shell com o prefixo `rtk`; para comandos sem suporte específico, use `rtk proxy <comando>`.
- Envie commits para `origin/main`. A branch local desta worktree pode ter outro nome; use `git push origin HEAD:main` para tornar o destino explícito.
- Escreva comunicação e documentação do projeto em português brasileiro. Preserve os títulos estruturais exigidos pelo OpenSpec e os identificadores de código em inglês.
- Não copie configurações ou código da extensão sem revisar dependências do navegador, segurança e licença. Registre origem, commit e licença de qualquer trecho reaproveitado.

## Arquitetura planejada

- Frontend: Vue 3, Composition API, TypeScript estrito e Vite; componentes por funcionalidade, estilos locais e estado transitório. Escolha motivada pela interface Vue da extensão, com modernização da camada de apresentação.
- Backend: Tauri 2 e Rust; domínio OTP e casos de uso separados dos adaptadores macOS e dos comandos IPC.
- Dependências apontam para o domínio: `commands → application → domain`; adaptadores implementam interfaces usadas pelos casos de uso. Componentes não acessam Keychain, arquivos ou clipboard diretamente.
- `src/lib/ipc/` centraliza contratos e chamadas tipadas. `src-tauri/src/domain/` não depende de Tauri nem de frameworks Apple.
- Integração macOS isolada em `src-tauri/src/platform/macos/`. Nenhum backend inseguro é habilitado automaticamente em plataformas não suportadas.
- `shell.rs` coordena intenção/visibilidade/rotas. O painel AppKit chama o core diretamente; objetos AppKit ficam na thread principal. Apenas `management` e a origem local podem usar IPC. Rotas nativas pendentes aguardam a autorização/UI pronta.
- `quick_access/` contém views nativas e o probe sintético; `scan/` contém seleção por monitor, ScreenCaptureKit e Vision. No macOS 14+ usar SCScreenshotManager; no 13, um frame completo SCStream e parada imediata. `sourceRect` usa pontos, resolução de saída usa pixels.
- O runtime deve operar offline, com assets empacotados e WebKit nativo. Não adicionar servidor, analytics, sincronização em nuvem ou Node/Chromium embarcado nesta mudança.

## Regras de segurança

- Seeds OTP, senhas de backup e material criptográfico pertencem ao core nativo. Seeds persistidos ficam exclusivamente no Keychain do macOS.
- Não persistir segredos em LocalStorage, SessionStorage, IndexedDB, stores web, preferências, logs, fixtures reais ou arquivos temporários. O formulário pode receber um segredo novo transitoriamente; limpe seus campos após envio/cancelamento.
- Usar o Keychain tradicional com ACL do aplicativo para persistência, sem exigir conta Apple. LocalAuthentication autoriza uma sessão privada no backend antes de qualquer leitura de seed; não aceitar booleanos de sucesso vindos do frontend. A ACL do item não oferece bloqueio biométrico independente do app; documentar essa fronteira de segurança.
- Use LocalAuthentication/Security diretamente no macOS. Não assumir que o plugin biométrico móvel do Tauri atende ao desktop.
- A autorização de sessão deve ser checada no Rust em cada operação sensível, inclusive depois de operações assíncronas. Bloquear ou suspender invalida contextos, operações pendentes e respostas antigas.
- O painel usa pontos AppKit do `NSStatusItem`/`NSScreen` para abrir abaixo do ícone; não misturar pixels Retina com a busca de monitor. Reabertura em menos de 30 segundos preserva navegação, busca, rolagem, foco e formulários não secretos somente em memória, na mesma sessão. Limpar senhas/seeds/URIs e códigos ao ocultar; descartar o restante após 30 segundos/bloqueio/expiração e revalidar a sessão antes de revelar. Eventos repetidos de ocultação não renovam o prazo.
- Nunca retornar seed, URI `otpauth://`, payload de backup ou senha em comandos de listagem/geração. Exportações autorizadas são gravadas pelo backend no destino escolhido.
- HOTP requer incremento persistido antes de entregar um novo código; renderização e atualização TOTP não consomem contadores HOTP.
- Clipboard, autenticação, arquivos e exportações passam por casos de uso específicos. Não expor leitura arbitrária de Keychain, filesystem, shell ou clipboard ao webview.
- Não habilitar conteúdo remoto, HTML arbitrário, permissões amplas, DevTools ou automação de testes no bundle de produção.
- Scanner só inicia por ação explícita, com sessão válida e permissão nativa. Limites: uma operação, 16 milhões de pixels, 16 QRs, 8 KiB por payload e timeout de 15 s após seleção. Não criar comando IPC de captura genérica, arquivo de screenshot ou URI secreta de retorno. Revisão QR usa UUID/geração, escolhe exatamente uma conta e reutiliza o commit de importação. Revogação cancela overlays/trabalho e invalida resultados.
- Use somente contas sintéticas e vetores públicos dos RFCs em testes. Não peça nem leia credenciais reais do usuário para validar o app.
- Regressão real em 15/10/2026: o build nativo instalado voltou a pedir senhas repetidamente e foi retirado. Antes de instalar outro build, rode `python3 scripts/verify-update-identity.py <app-atual> <candidato>`; saída não zero bloqueia a atualização normal. Assinatura ad hoc válida não mantém identidade entre builds: o candidato falhou no requisito do estável. Fonte de auth/Keychain inalterada não comprova compatibilidade de atualização. Exigir também teste de upgrade com itens sintéticos antes de retestar com o usuário; não remover ACL, ampliar confiança ou pedir senhas para contornar a falha.
- O usuário escolheu uma migração inicial separada: backup criptografado exportado pela UI estável com contas/contadores atuais. O novo certificado local não passa no requisito ad hoc antigo; não falsificar aprovação do preflight. Preservar serviço/manifesto antigo e usar `signed.vault.v1`/`vault-signed-v1`, com orientação de importação. Não gerar HOTP na versão anterior depois de exportar. Contadores dos dois cofres não sincronizam.
- Depois dos testes nativos aprovados, o usuário dispensou nova exportação: “Pode chutar tudo pro ralo, eu ja tenho backup das coisas”. Isso autoriza instalar usando seus backups existentes, sem pedir confirmação de exportação novamente. Não declarar frescor/contadores desses backups verificados, não ler arquivos pessoais e não apagar o cofre antigo sem necessidade. A restauração é feita pelo usuário na UI.
- Builds locais usam `scripts/local-signing-identity.py`, que fixa uma identidade própria no Keychain e metadados públicos em `~/Library/Application Support/TopOTP Build/`. Não regenerar silenciosamente a chave, não exportá-la para git/artefatos, não alterar trust settings. A autorização inicial do macOS para codesign usar essa nova chave de assinatura é distinta do desbloqueio OTP; nunca pedir a senha por chat ou contornar a autorização. CI deve usar `TOPOTP_ADHOC_BUILD=1` e registrar que não valida continuidade do cofre.
- A UI do Keychain tradicional fica desativada no processo do app; LocalAuthentication mantém o diálogo explícito existente. Acesso recusado revoga a sessão e interrompe leituras automáticas. O teste real entre dois binários/Keychain descartável vive em `scripts/keychain-update-probe.py`; `--local-identity` valida o certificado efetivamente usado pelos builds.

## Ferramentas e validação planejadas

Node 24.21.0/npm 11.19.0 e Rust 1.99.0 estão fixados no projeto, com lockfiles, `vue-tsc`, ESLint, Prettier, Vitest/Vue Test Utils, rustfmt e Clippy configurados. Consulte `docs/development.md` para requisitos e comandos; não instale dependências da extensão em bloco.

Os scripts `dev`, `tauri:dev`, `build`, `tauri:build`, `typecheck`, `lint`, `format:check`, `test`, `test:rust`, `test:interop`, `check:rust`, `format:rust` e `probe:keychain` existem. `test:interop` executa o codec Rust contra as funções originais do upstream preservadas em `tests/reference/`, com checksums/licença; não edite essas cópias. O importador original exige enums numéricos em strings na exportação; a importação TopOTP aceita também nomes e números JSON inteiros. `EntryStorage.import`/`backupGetExport` podem produzir um arquivo v3 misturando essas representações; preserve a cobertura da fixture `authenticator-v3-imported.json`. Consulte o guia de compatibilidade antes de alterar esses campos. A suíte funcional e `test:e2e` existem. A feature `e2e` usa dados/adaptadores sintéticos e é proibida em release. Contratos são gerados por `npm run bindings` e verificados por teste Rust.

Validar comportamento por camadas: vetores RFC e importação no domínio; falhas de persistência e corridas de sessão nos casos de uso; estados e acessibilidade na UI; integração real de Keychain/Touch ID em um `.app` macOS. WebdriverIO para o desktop deve usar um build exclusivo de testes; os plugins de automação não podem integrar o release.

O executável `quick-access-probe` requer `native-ui-probe`, não acessa o core/Keychain e testa controles AppKit com contas sintéticas. A feature é proibida em release. Os testes Vision geram QRs com `qrcodegen` (somente dev-dependency), sem pedir permissão ou capturar a tela. E2E dirige somente `management`; não equivale a teste do popover/captura. Preserve bundles estáveis e produza builds desta mudança em `.artifacts/native-quick-access/`, sem substituir a instalação em uso nem publicar outra beta automaticamente.

`native-smoke` exige `tauri.native-smoke.conf.json`, identifier/serviço/diretório isolados e é proibida em release. Executa o shell completo com autenticação e Keychain reais; inclui somente duas contas públicas sintéticas após autorização nativa. Não equivale ao E2E com auth mockada. Não usar contas pessoais para validar biometria/captura nesse harness.

Ao concluir uma tarefa, marcar somente itens comprovadamente entregues e registrar os testes executados. Diferenciar teste com adaptador simulado de teste nativo, build local de release assinado, e suporte planejado de suporte verificado.

Canal atual: `0.0.1-beta.1`, tag `v0.0.1-beta.1`; `v0.0.1` será a primeira versão estável. Mantenha versões npm/Cargo/Tauri e lockfiles alinhadas. O nome do DMG deve vir da versão do `.app`, sem constante fixa no empacotador. O pré-release publicado permanece ad hoc. Novos builds locais usam certificado local persistente; nenhum é notarizado. Não publicar outra versão automaticamente.
