## Why

TopOTP precisa oferecer acesso rápido a códigos de autenticação de dois fatores diretamente na barra de menus do macOS, preservando a usabilidade e a interoperabilidade de dados do Authenticator Extension. Como o repositório ainda não possui aplicativo, esta mudança estabelece sua fundação técnica e um modelo de segurança que protege os segredos no Keychain e usa autenticação nativa sem exigir uma senha mestre própria a cada consulta.

## What Changes

- Criar um app offline com Tauri 2, core Rust e interface Vue 3/TypeScript/Vite, usando o WebKit do macOS e assets embarcados.
- Disponibilizar ícone na barra de menus e painel compacto imediatamente abaixo do ícone para listar, buscar, fixar, cadastrar, editar e remover contas, visualizar TOTP e gerar HOTP por ação explícita. Fechar o painel mantém o app em execução; reabrir em menos de 30 segundos retoma navegação, busca, rolagem e formulários sem guardar senhas/seeds, enquanto a sessão continua válida.
- Implementar TOTP/HOTP, algoritmos SHA-1/SHA-256/SHA-512, 6–8 dígitos, períodos TOTP configuráveis e cópia de códigos com limpeza condicional do clipboard.
- Persistir seeds e registros sensíveis exclusivamente no Keychain tradicional do macOS (`login`), com acesso restrito ao aplicativo. Autorizar sessões no Rust por LocalAuthentication, preferindo Touch ID e permitindo senha do macOS, sem exigir conta Apple ou perfil de provisionamento. O bloqueio biométrico será aplicado pelo backend do app, não pelo próprio item do Keychain; usar sessões curtas e bloqueio automático.
- Reproduzir os fluxos cotidianos da extensão dentro da interface do app: cadastro, consulta/cópia e importação/exportação de backups com seleção de arquivo e entrada de senha. Adotar compatibilidade de dados com [Authenticator-Extension/Authenticator](https://github.com/Authenticator-Extension/Authenticator) via JSON para os tipos OTP suportados, incluindo o formato criptografado v3, e cadastro por URI `otpauth://`. Exportações exigem nova autenticação nativa e senha de backup para o formato criptografado; o desbloqueio normal usa Touch ID ou senha do usuário do macOS.
- Provisionar ferramentas reproduzíveis, organização de código por domínio/casos de uso/adaptadores, contratos IPC tipados, testes pertinentes, CI macOS e documentação para agentes. OpenSpec, contexto inicial e orientações de agentes são preparados nesta etapa; o app e suas dependências serão implementados nas tarefas.
- Limitar a primeira entrega a macOS 13 ou superior, com builds Apple Silicon e Intel. Não incluir nuvem, servidor, analytics, autofill de navegador, QR/câmera/captura de tela, Google migration, Steam/Battle.net, seeds hexadecimais, GOST ou leitura de formatos criptografados anteriores ao v3. Não prometer paridade integral com a extensão. A beta `v0.0.1-beta.1` foi publicada como pré-release. Por solicitação posterior explícita do usuário, preparar `v0.0.1` estável com as correções da revisão, via PR e merge pelo usuário antes de gerar/publicar DMGs e checksums a partir da `main` no GitHub. Manter as verificações nativas pendentes documentadas, sem declará-las concluídas pela promoção de versão.

## Capabilities

### New Capabilities

- `macos-menu-bar-shell`: ciclo de vida do app, instância única, ícone de barra de menus, painel e ações nativas de bloquear/sair.
- `otp-accounts`: gestão de contas, validação de parâmetros, geração correta de TOTP/HOTP, busca/fixação e cópia de códigos.
- `keychain-vault`: persistência nativa com ACL do aplicativo, autorização de sessão por LocalAuthentication, bloqueio e proteção das fronteiras IPC.
- `authenticator-compatibility`: contratos de backup e URI, migração com prévia e confirmação, exportação autorizada e rastreabilidade do upstream.

### Modified Capabilities

Nenhuma. O repositório não possui especificações de comportamento anteriores.

## Impact

A implementação introduzirá `src/`, `src-tauri/`, manifests e lockfiles npm/Cargo, configurações de qualidade e CI, além de documentação de desenvolvimento e validação nativa. Rust será responsável por geração de OTP, arquivos de backup, clipboard, Keychain e LocalAuthentication; o frontend receberá apenas dados de apresentação e códigos autorizados.

Não haverá alteração de contratos existentes nem migração de dados locais do TopOTP. A importação lerá arquivos escolhidos pelo usuário e não modificará a extensão. Qualquer reaproveitamento de código MIT do upstream manterá atribuição e commit de origem, conforme [pesquisa de compatibilidade](../../../docs/upstream-compatibility.md).

O aplicativo usará identidade estável e assinatura ad hoc local, sem conta Apple. O Keychain tradicional e a autorização por LocalAuthentication devem ser comprovados em macOS antes da entrega. Builds sem Developer ID/notarização podem exigir aprovação do usuário no Gatekeeper quando distribuídos; esse comportamento será documentado. Certificados Apple não serão pré-requisito de uso nesta mudança.
