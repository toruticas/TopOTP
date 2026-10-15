# TopOTP

Autenticador **TOTP/HOTP para a barra de menus do macOS**, com Vue 3/TypeScript no WebKit nativo e Tauri 2/Rust no core. Opera offline e usa o Keychain tradicional do macOS para as chaves.

Desbloqueio por **Touch ID ou senha do macOS, sem conta Apple**. Touch ID foi confirmado pelo usuário no `.app` ad hoc. A autenticação protege a sessão do aplicativo; o item do Keychain usa ACL do app e não possui uma exigência biométrica independente.

Versão de testes: **[v0.0.1-beta.1](https://github.com/toruticas/TopOTP/releases/tag/v0.0.1-beta.1)**. Os pacotes `.dmg` para Apple Silicon e Intel e seus checksums ficam no pré-release. A versão estável `v0.0.1` fica reservada para depois das validações mínimas do produto.

## Recursos implementados

- TOTP/HOTP SHA-1/256/512, 6–8 dígitos, períodos personalizados e contador persistido antes da exibição.
- Cadastro manual/URI com revisão, edição, exclusão confirmada, busca, fixação, ordem e cópia nativa.
- Importação/exportação pela interface: JSON claro e Authenticator v3 criptografado, múltiplas senhas, prévia/seleção e duplicatas preservadas.
- Sessões limitadas, revogação de operações antigas, commits com recuperação e IPC restrito ao painel local.
- Painel ancorado abaixo do ícone, com retomada da tela/busca/rolagem/formulários não secretos ao reabrir em menos de 30 segundos.

Importação/exportação verificadas contra as funções originais de [Authenticator-Extension/Authenticator](https://github.com/Authenticator-Extension/Authenticator) no commit fixado: JSON claro/v3, códigos TOTP, parâmetros e próximo contador HOTP. Testes E2E exercitam ambos os formatos pelo app, com adaptadores nativos sintéticos. A extensão completa no navegador ainda não foi testada. Veja [detalhes e diferenças de formato](docs/upstream-compatibility.md).

## Executar e compilar

Node 24.21.0/npm 11.19.0, Rust 1.99.0 e Xcode Command Line Tools são necessários.

```sh
rtk proxy npm ci
rtk proxy npm run tauri:dev
rtk proxy npm run tauri:build
```

O build cria `.app` e `.dmg` com assinatura ad hoc, sem notarização. Builds arm64 e x86_64 foram gerados; Intel e macOS 13 ainda aguardam execução correspondente. O pré-release é experimental e mantém essas limitações documentadas.

Consulte [uso e recuperação](docs/usage.md), [desenvolvimento e checks](docs/development.md), [evidências nativas](docs/native-verification.md) e [compatibilidade upstream](docs/upstream-compatibility.md).

A mudança [create-macos-menu-bar-authenticator](openspec/changes/create-macos-menu-bar-authenticator/tasks.md) continua aberta para verificações manuais de senha/cancelamento, sistema suspenso/bloqueado, dois monitores, clipboard, VoiceOver e atualização de ACL/Gatekeeper.

## Licença

MIT. Aplicativo e ícones originais; os testes incluem módulos upstream com licença e checksums preservados. Consulte [atribuições](THIRD_PARTY_NOTICES.md) e [proveniência das fixtures](tests/fixtures/README.md).
