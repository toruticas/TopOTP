# TopOTP — orientações para agentes

## Objetivo e estado atual

TopOTP será um autenticador TOTP/HOTP para a barra de menus do macOS, com interface web embarcada em Tauri 2 e segurança nativa em Rust. A referência funcional e de interoperabilidade é [Authenticator-Extension/Authenticator](https://github.com/Authenticator-Extension/Authenticator).

O aplicativo está implementado, com cofre, OTP e transferências pela interface. Touch ID foi confirmado pelo usuário; builds locais arm64/x86_64 e testes automatizados foram executados. A mudança permanece aberta para as verificações manuais descritas em `docs/native-verification.md`; não transforme mocks/preflight em evidência de hardware.

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
- Para a entrega `v0.0.1`, envie a branch de trabalho e abra um PR para `main`; o usuário fará o merge. Gere os artefatos e publique a versão no GitHub somente depois do merge, a partir do commit da `main`, por execução manual controlada.
- Escreva comunicação e documentação do projeto em português brasileiro. Preserve os títulos estruturais exigidos pelo OpenSpec e os identificadores de código em inglês.
- Não copie configurações ou código da extensão sem revisar dependências do navegador, segurança e licença. Registre origem, commit e licença de qualquer trecho reaproveitado.

## Arquitetura planejada

- Frontend: Vue 3, Composition API, TypeScript estrito e Vite; componentes por funcionalidade, estilos locais e estado transitório. Escolha motivada pela interface Vue da extensão, com modernização da camada de apresentação.
- Backend: Tauri 2 e Rust; domínio OTP e casos de uso separados dos adaptadores macOS e dos comandos IPC.
- Dependências apontam para o domínio: `commands → application → domain`; adaptadores implementam interfaces usadas pelos casos de uso. Componentes não acessam Keychain, arquivos ou clipboard diretamente.
- `src/lib/ipc/` centraliza contratos e chamadas tipadas. `src-tauri/src/domain/` não depende de Tauri nem de frameworks Apple.
- Integração macOS isolada em `src-tauri/src/platform/macos/`. Nenhum backend inseguro é habilitado automaticamente em plataformas não suportadas.
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
- Use somente contas sintéticas e vetores públicos dos RFCs em testes. Não peça nem leia credenciais reais do usuário para validar o app.

## Ferramentas e validação planejadas

Node 24.21.0/npm 11.19.0 e Rust 1.99.0 estão fixados no projeto, com lockfiles, `vue-tsc`, ESLint, Prettier, Vitest/Vue Test Utils, rustfmt e Clippy configurados. Consulte `docs/development.md` para requisitos e comandos; não instale dependências da extensão em bloco.

Os scripts `dev`, `tauri:dev`, `build`, `tauri:build`, `typecheck`, `lint`, `format:check`, `test`, `test:rust`, `test:interop`, `check:rust`, `format:rust` e `probe:keychain` existem. `test:interop` executa o codec Rust contra as funções originais do upstream preservadas em `tests/reference/`, com checksums/licença; não edite essas cópias. O importador original exige enums numéricos em strings na exportação; a importação TopOTP aceita também nomes e números JSON inteiros. `EntryStorage.import`/`backupGetExport` podem produzir um arquivo v3 misturando essas representações; preserve a cobertura da fixture `authenticator-v3-imported.json`. Consulte o guia de compatibilidade antes de alterar esses campos. A suíte funcional e `test:e2e` existem. A feature `e2e` usa dados/adaptadores sintéticos e é proibida em release. Contratos são gerados por `npm run bindings` e verificados por teste Rust.

Validar comportamento por camadas: vetores RFC e importação no domínio; falhas de persistência e corridas de sessão nos casos de uso; estados e acessibilidade na UI; integração real de Keychain/Touch ID em um `.app` macOS. WebdriverIO para o desktop deve usar um build exclusivo de testes; os plugins de automação não podem integrar o release.

Ao concluir uma tarefa, marcar somente itens comprovadamente entregues e registrar os testes executados. Diferenciar teste com adaptador simulado de teste nativo, build local de release assinado, e suporte planejado de suporte verificado.

Versão do código: `0.0.1`, tag de release `v0.0.1`, primeira versão estável solicitada. A beta histórica `v0.0.1-beta.1` permanece publicada; não confunda a preparação do PR com build ou publicação da estável. Mantenha versões npm/Cargo/Tauri e lockfiles alinhadas. O nome do DMG deve vir da versão do `.app`, sem constante fixa no empacotador. A publicação no GitHub deve incluir DMGs por arquitetura e `SHA256SUMS`, vinculados ao commit da `main` após o merge. Builds permanecem ad hoc e não notarizados, e as pendências nativas continuam abertas enquanto não houver evidência correspondente.
