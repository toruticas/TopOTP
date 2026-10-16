# TopOTP

Autenticador **TOTP/HOTP para a barra de menus do macOS**, com painel AppKit nativo, gerenciamento em Vue 3/TypeScript no WebKit e Tauri 2/Rust no core. Opera offline e usa o Keychain tradicional do macOS para as chaves.

Desbloqueio por **Touch ID ou senha do macOS, sem conta Apple**. A autenticação protege a sessão no core Rust; o Keychain usa ACL/partição do componente nativo de armazenamento, sem exigência biométrica independente do item. Touch ID foi confirmado pelo usuário também com o componente fixo, seguido de desbloqueio/importação pela interface na versão instalada.

Versão de testes: **[v0.0.1-beta.1](https://github.com/toruticas/TopOTP/releases/tag/v0.0.1-beta.1)**. Os pacotes `.dmg` para Apple Silicon e Intel e seus checksums ficam no pré-release. A versão estável `v0.0.1` fica reservada para depois das validações mínimas do produto.

## Recursos implementados

- TOTP/HOTP SHA-1/256/512, 6–8 dígitos, períodos personalizados e contador persistido antes da exibição.
- Cadastro manual/URI com revisão, edição, exclusão confirmada, busca, fixação, ordem e cópia nativa.
- Importação/exportação pela interface: JSON claro e Authenticator v3 criptografado, múltiplas senhas, prévia/seleção e duplicatas preservadas.
- Painel AppKit abaixo do ícone, altura ajustada às contas e rolagem interna no limite, busca nativa sem distinção de acentos e seção **Mais** expansível e Configurações fixa no rodapé. Cadeado ao lado do título para bloquear; busca desativada no cofre vazio. Clique bloqueado inicia uma autenticação; a lista aparece após aprovação.
- Primeiro acesso pela mesma lista nativa vazia, com Nova entrada, Importar e Escanear disponíveis em Mais, sem etapa inicial obrigatória.
- Retomada de busca, rolagem, seleção e foco em menos de 30 segundos, na mesma sessão. Configurações e formulários abrem em janela própria.
- Leitura local de QR TOTP/HOTP de uma área da tela, com revisão e escolha de uma conta. Imagens e URIs reconhecidas não são enviadas à janela web.
- Sessões limitadas, revogação de operações antigas, commits com recuperação e IPC restrito à janela local de gerenciamento.
- Remoção confirmada dos dados do cofre atual em Geral, com autorização nativa e limpeza de chaves/transientes. Janela de gerenciamento permanece aberta ao trocar de aplicativo.

O painel nativo e o scanner estão na mudança em desenvolvimento [native-macos-quick-access](openspec/changes/native-macos-quick-access/tasks.md); ainda não integram o pré-release publicado. Em 16/10 foi comprovado que o certificado local sozinho não mantém acesso aos itens do Keychain login: sua partição estava presa ao hash do build anterior. A correção usa um helper de armazenamento fixo; testes no login real passaram entre clientes diferentes, com isolamento e recusa a assinaturas/componentes incorretos. O build corrigido foi instalado e o usuário confirmou desbloqueio/importação pela interface em um cofre separado, com o anterior preservado. Veja as [evidências nativas](docs/native-verification.md).

Importação/exportação verificadas contra as funções originais de [Authenticator-Extension/Authenticator](https://github.com/Authenticator-Extension/Authenticator) no commit fixado: JSON claro/v3, códigos TOTP, parâmetros e próximo contador HOTP. Testes E2E exercitam ambos os formatos pelo app, com adaptadores nativos sintéticos. A extensão completa no navegador ainda não foi testada. Veja [detalhes e diferenças de formato](docs/upstream-compatibility.md).

## Executar e compilar

Node 24.21.0/npm 11.19.0, Rust 1.99.0 e Xcode Command Line Tools são necessários.

```sh
rtk proxy npm ci
rtk proxy npm run tauri:dev
rtk proxy npm run tauri:build
```

O build local cria `.app` e `.dmg` com certificado local persistente e preserva o helper assinado por arquitetura, sem conta Apple e sem notarização. Não regenere o helper: ele é o dono estável dos itens do cofre. A passagem do modelo direto usa backup pela UI estável e mantém o cofre anterior. Artefatos ad hoc de CI servem apenas à verificação de compilação/empacotamento; não habilitam o cofre. Intel e macOS 13 ainda aguardam execução correspondente. Veja o [guia de desenvolvimento](docs/development.md).

Consulte [uso e recuperação](docs/usage.md), [desenvolvimento e checks](docs/development.md), [evidências nativas](docs/native-verification.md) e [compatibilidade upstream](docs/upstream-compatibility.md).

A mudança [create-macos-menu-bar-authenticator](openspec/changes/create-macos-menu-bar-authenticator/tasks.md) continua aberta para verificações manuais de senha/cancelamento, sistema suspenso/bloqueado, dois monitores, clipboard, VoiceOver e atualização de ACL/Gatekeeper.

## Licença

MIT. Aplicativo e ícones originais; os testes incluem módulos upstream com licença e checksums preservados. Consulte [atribuições](THIRD_PARTY_NOTICES.md) e [proveniência das fixtures](tests/fixtures/README.md).
