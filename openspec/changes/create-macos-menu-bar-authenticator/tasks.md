## 1. Fundação reproduzível do projeto

- [x] 1.1 Verificar os pré-requisitos macOS/Xcode CLT, selecionar versões compatíveis de Node 24 LTS/npm e Rust stable e registrar versões exatas em `.nvmrc`, `engines` e `rust-toolchain.toml`.
- [x] 1.2 Criar o projeto Tauri 2 + Vue 3/TypeScript/Vite preservando README, licença e OpenSpec; configurar lockfiles npm/Cargo e target do frontend compatível com macOS 13.
- [x] 1.3 Criar as camadas `domain`, `application`, `platform/macos`, `compatibility`, `commands` e funcionalidades frontend conforme o design, com interfaces de relógio, armazenamento, autenticação, clipboard e arquivos.
- [x] 1.4 Configurar TypeScript estrito, `vue-tsc`, ESLint/Prettier, Vitest/Vue Test Utils, rustfmt/Clippy e os scripts de desenvolvimento/build/validação especificados no design.
- [x] 1.5 Definir identidade estável do app e namespaces separados dev/test/release para Keychain; adicionar `.gitignore` para builds, arquivos pessoais e material de assinatura.
- [x] 1.6 Completar `docs/development.md` e atualizar README/AGENTS.md com caminhos e comandos realmente existentes, registrando que builds locais e releases assinados têm verificações distintas.

## 2. Comprovação da proteção nativa antes de integrar o cofre

- [x] 2.1 Integrar bindings macOS de Security/LocalAuthentication e criar um `.app` mínimo com item sintético no Keychain tradicional, ACL restrita ao aplicativo e assinatura ad hoc, sem conta Apple.
- [x] 2.2 Comprovar escrita/leitura e isolamento do item sintético no Keychain tradicional; acesso negado ou indisponível deve falhar sem armazenar segredo fora do Keychain.
- [ ] 2.3 Validar em macOS real desbloqueio por Touch ID e fallback à senha do macOS, incluindo cancelamento/recusa; registrar evidência e limitações sem usar contas reais.
- [ ] 2.4 Verificar sessão nativa limitada após LocalAuthentication e seu descarte/invalidação; comprovar que exportação exige uma avaliação nativa nova, sem sucesso de autenticação fornecido pelo frontend.
- [ ] 2.5 Provar observação de bloqueio do macOS, suspensão/retomada e inativação de sessão, documentando em `docs/native-verification.md` os sinais usados e os testes reais executados.

## 3. Domínio OTP e parâmetros de contas

- [x] 3.1 Implementar modelo de conta e validações Base32, tipo, algoritmo, dígitos, período e contador, com defaults e representação de contadores sem perda de precisão.
- [x] 3.2 Implementar TOTP SHA-1/SHA-256/SHA-512 e validar vetores públicos RFC 6238, zeros à esquerda, período customizado e fronteiras/mudanças do relógio.
- [x] 3.3 Implementar HOTP SHA-1 com vetores RFC 4226 e variantes SHA-256/SHA-512 com referência upstream; testar incremento fora do intervalo suportado.
- [x] 3.4 Implementar interpretação/validação de URI `otpauth://` com prévia sem seed, defaults TOTP, contador obrigatório HOTP e rejeição de tipos/parâmetros não suportados.

## 4. Persistência do cofre e recuperação

- [x] 4.1 Implementar registros versionados por credencial no Keychain tradicional, com ACL do aplicativo, seed/metadata sensível em `kSecValueData`, sem sincronização e com atributos externos restritos a identificadores opacos.
- [x] 4.2 Implementar manifesto de referências/ordem/fixação e preferências sem seeds, rótulos de conta ou contadores autoritativos; escrever o manifesto atomicamente.
- [x] 4.3 Implementar criação/edição/exclusão com reconhecimento após commit, preservação de seed quando não substituído e tratamento de itens pendentes/referências ausentes.
- [x] 4.4 Implementar staging e journal somente de IDs para commits de conjuntos; testar falha em etapas intermediárias e reinício mantendo o conjunto anterior ou o novo integralmente.
- [x] 4.5 Aplicar wrappers/zeroização de buffers sensíveis e erros/logs sem payload; verificar com seeds sintéticos que arquivos comuns e logs do app não os persistem.

## 5. Shell e ciclo de vida na barra de menus

- [x] 5.1 Criar tray Rust, ícone template e menu Abrir/Bloquear/Sair, com startup bloqueado, painel oculto e `ActivationPolicy::Accessory` sem Dock.
- [x] 5.2 Implementar instância única e encaminhar segunda abertura à instância existente, mantendo um único escritor do cofre.
- [ ] 5.3 Posicionar o painel a partir do tray/monitor/DPI, limitar às bordas e validar abertura em monitor secundário.
- [x] 5.4 Implementar toggle, Escape, perda de foco e fechar para ocultar, preservando o fluxo durante diálogos nativos de autenticação/arquivo.
- [x] 5.5 Ligar Bloquear/Sair nativos à revogação de autorização e limpeza de transientes, funcionando mesmo com frontend indisponível.
- [x] 5.6 Corrigir o posicionamento Retina com coordenadas AppKit e movimento síncrono antes de exibir; testar alinhamento abaixo do ícone, bordas e origens de monitores secundários.

## 6. Sessão e casos de uso autorizados

- [x] 6.1 Implementar estados bloqueado/autenticando/desbloqueado e autorização privada de sessão após LocalAuthentication no Rust, sem ler credenciais antes da autenticação e preservando bloqueio em todas as falhas.
- [x] 6.2 Implementar geração de sessão, timers de 120 s sem interação/300 s máximos/30 s oculto, eventos do sistema e limpeza de UI/transientes; polling de códigos não renova autorização.
- [x] 6.3 Ordenar commits e bloqueio no coordenador nativo; testar bloqueio durante autenticação, cálculo, diálogo e importação, incluindo respostas antigas e sucessos tardios.
- [x] 6.4 Implementar gestão/listagem autorizadas, atualização TOTP e consumo HOTP serializado com incremento persistido antes de emitir; testar falha de gravação, concorrência, reinício e contador que não pode retroceder.
- [x] 6.5 Implementar cópia nativa de TOTP atual/HOTP emitido, limpeza em 30 s/bloqueio/sair por identidade da escrita e preservação de nova escrita de outro app, mesmo com texto igual.

## 7. Contratos IPC e permissões

- [x] 7.1 Expor somente os comandos necessários e DTOs de apresentação tipados, com verificação Rust/TypeScript, erros estáveis e autorização em todas as operações sensíveis.
- [x] 7.2 Restringir comandos do aplicativo via manifest/permissões e capability da janela/origem local; configurar CSP, navegação e DevTools sem permissões genéricas de shell, rede, filesystem ou clipboard.
- [x] 7.3 Verificar rejeição de cliente/janela não autorizado e comandos enquanto bloqueado; inspecionar respostas/eventos para provar ausência de seeds, senhas e payloads de backup.

## 8. Interface cotidiana de contas

- [x] 8.1 Criar painel em português com estados bloqueado/autenticando/vazio/lista/erro e desbloqueio por UI nativa, sem campo de senha mestre própria.
- [x] 8.2 Implementar lista, busca, fixação/ordem, código/tempo restante, Copiar e Gerar próximo, sem consumir HOTP por renderização nem exibir nomes/códigos bloqueados.
- [x] 8.3 Implementar cadastro manual/URI, edição com substituição opcional de seed e exclusão confirmada; limpar campos sensíveis e só refletir sucesso depois do commit.
- [x] 8.4 Mascarar o painel e limpar códigos/segredos digitados ao ocultar; limpar toda a apresentação ao bloquear e descartar respostas de gerações expiradas, incluindo reabertura rápida e atualização após mudança do relógio.
- [x] 8.5 Verificar navegação por teclado, nomes acessíveis, foco, indicação não exclusiva por cor e renderização de labels hostis como texto.
- [x] 8.6 Retomar tela, busca, rolagem, foco, formulários não secretos e seleção em menos de 30 segundos; testar limite exato, ocultações repetidas, limpeza de senhas/seeds e revogação nativa antes de revelar.

## 9. Importação de backup pela interface

- [x] 9.1 Criar fixtures sintéticas reproduzíveis do upstream fixado para JSON claro, v3 criptografado, múltiplos `keyId`, HOTP e defaults omitidos; registrar proveniência e licença de qualquer trecho reaproveitado.
- [x] 9.2 Implementar codec JSON claro com preservação de parâmetros/metadados, classificação explícita de entradas inválidas/não suportadas e limites de arquivo/registros/campos.
- [x] 9.3 Implementar Argon2/envelope CryptoJS v3 no Rust, limites PHC/ciphertext antes de derivação, múltiplas senhas, progresso/cancelamento e tempo máximo por tentativa; validar senha incorreta e chave ausente.
- [x] 9.4 Criar fluxo Importar backup no painel com diálogo nativo, entrada transitória de senhas, prévia sem seeds e seleção/confirmar/cancelar, inteiramente utilizável pelo app.
- [x] 9.5 Implementar duplicatas mantendo existente por padrão, recusa de redução HOTP, confirmação explícita de subconjunto e commit do conjunto aprovado pelo mecanismo de staging/manifesto.
- [x] 9.6 Testar versões antigas/desconhecidas, arquivos excessivos, parâmetros criptográficos hostis, falhas de Keychain e bloqueio/cancelamento durante importação; nenhum dado descriptografado vai a disco ou para a UI.
- [x] 9.7 Aceitar enums numéricos em JSON além de nomes/strings numéricas, reproduzindo backups v3 mistos por `EntryStorage.import`/`backupGetExport` originais e testando a importação completa sem alterar a autenticação nativa.

## 10. Exportação de backup pela interface

- [x] 10.1 Implementar serialização compatível clara/v3 para contas selecionadas, defaults/campos preservados, salts/IDs aleatórios e contador HOTP não consumido pela exportação.
- [x] 10.2 Criar fluxo Exportar backup com seleção de contas/formato, criptografia v3 padrão, senha/confirmar senha transitórias e reconhecimento explícito da alternativa em texto claro.
- [x] 10.3 Exigir autenticação nativa nova e diálogo de destino; gravar pelo backend com permissões restritas, ciphertext temporário/commit atômico ou destino final novo para texto claro, retornando somente status.
- [ ] 10.4 Testar recusa de autenticação, senha não confirmada, cancelamento, bloqueio antes do commit e falha de arquivo, com limpeza de saídas incompletas e sem temporários em texto claro.
- [x] 10.5 Demonstrar ida e volta com a extensão de referência para backups claros/v3, igualdade de códigos TOTP e próximo contador HOTP; documentar a limitação de integridade do envelope de compatibilidade.

## 11. Verificação integrada e CI

- [x] 11.1 Configurar CI macOS com instalação por lockfiles, checagens TypeScript/lint/formatação, testes frontend/Rust e build, sem exigir conta real, biometria automatizada ou credenciais de release.
- [x] 11.2 Configurar WebdriverIO/Tauri embedded exclusivamente para build de testes e cobrir pelo app os fluxos de conta/importação/exportação/bloqueio com dados sintéticos; distinguir mocks de autenticação real.
- [ ] 11.3 Executar testes reais do `.app` para Touch ID/senha/cancelamento, bloqueio/suspensão/retomada, dois monitores, instância única, clipboard, VoiceOver e continuidade HOTP após reinício; registrar resultados verificáveis.
- [x] 11.4 Inspecionar o bundle de produção para confirmar ausência de servidor/plugins/mocks de automação, conteúdo remoto e permissões amplas; executar o fluxo local offline.

## 12. Build local e documentação de entrega

- [x] 12.1 Gerar builds locais `.app`/`.dmg` arm64 e x86_64 para macOS 13+, registrando quais targets foram apenas compilados e quais foram executados em hardware/sistema correspondente.
- [x] 12.2 Configurar assinatura ad hoc local e documentar comportamento do Gatekeeper e confiança do Keychain após atualizações; nenhuma conta/perfil Apple será exigida. Deixar publicação pública fora da implementação e Developer ID/notarização como opção futura, sem alegar verificações não executadas.
- [x] 12.3 Atualizar README, AGENTS.md e guias de desenvolvimento/uso/recuperação com os fluxos efetivos do app, compatibilidade verificada, preservação do Keychain no uninstall/rollback e comportamento de HOTP após interrupção.
- [x] 12.4 Executar as checagens aplicáveis finais e validar a mudança OpenSpec; marcar somente tarefas entregues, registrando qualquer verificação nativa ou de target ainda pendente com sua causa.
- [x] 12.5 Preparar a primeira beta `0.0.1-beta.1` com manifests/lockfiles alinhados, empacotamento versionado, notas, DMGs arm64/x86_64 inspecionados e checksums para o pré-release no GitHub.

## 13. Correções da revisão do repositório

- [x] 13.1 Preservar a identidade original da escrita no clipboard e testar a substituição por outra escrita antes do retorno da cópia, em pasteboard isolado.
- [x] 13.2 Preservar o índice externo atualizado de registros v3 e cobrir a divergência legítima em relação ao índice cifrado interno.
- [x] 13.3 Impedir exportações acima dos limites de arquivo/ciphertext aceitos na importação, com orientação de erro e testes de borda.
- [x] 13.4 Informar exclusão pendente quando a limpeza do Keychain falha após o commit, preservar a recuperação e testar o aviso na interface.
- [x] 13.5 Exibir dígitos, período TOTP e próximo contador HOTP na prévia de importação/URI, com testes de apresentação sem seeds.

## 14. Publicação estável solicitada pelo usuário

- [x] 14.1 Alinhar manifests/lockfiles em `0.0.1` e preparar documentação/notas sem ocultar verificações nativas pendentes.
- [x] 14.2 Separar checks de PR e publicação manual da `main`, com builds por arquitetura, inspeção e assets DMG/SHA256SUMS no GitHub; validar a configuração sem gerar pacotes antes da PR.
- [x] 14.3 Enviar a branch e abrir PR das correções/versionamento para o usuário revisar e fazer merge.
- [ ] 14.4 Confirmar o merge pelo usuário e identificar o commit resultante na `main`.
- [ ] 14.5 Executar o workflow a partir desse commit, verificar artefatos/checksums e confirmar a release `v0.0.1` publicada no GitHub.

## 15. Correções da avaliação de segurança antes de produção

- [x] 15.1 Ignorar o período irrelevante na identidade HOTP, preservando os períodos TOTP distintos; reproduzir e cobrir a deduplicação de backups reidentificados e de entradas do mesmo arquivo sem retroceder contadores.
- [x] 15.2 Revalidar duplicatas contra o cofre atual no commit autorizado e preservar a prévia quando a validação falhar, com regressões para mutação entre prévia e confirmação e falha de leitura.
- [x] 15.3 Atualizar dependências/overrides de desenvolvimento com correções compatíveis e registrar auditorias npm de produção/completa e eventuais alertas residuais.
- [x] 15.4 Executar os testes Rust/interoperabilidade, frontend, E2E e checks pertinentes; atualizar documentação/evidências sem concluir verificações nativas ou publicação pendentes.

## Evidências e pendências da entrega

- Em 13/11/2026, a avaliação de segurança foi corrigida: HOTP ignora o período na identidade e a importação recalcula duplicatas contra o cofre atual sob o coordenador. Seis testes Rust e um E2E foram acrescentados; quatro regressões falharam antes da correção. Os 73 testes passaram: 50 unitários Rust, 1 de interoperabilidade, 17 frontend e 5 E2E no WKWebView com adaptadores sintéticos. Typecheck, ESLint, Prettier, Clippy, rustfmt, alinhamento de versões e validação OpenSpec estrita passaram; a revisão independente não identificou regressão no ajuste. Erros de leitura/seleção mantêm a prévia, inclusive após descriptografia; falhas posteriores de persistência continuam usando a recuperação do cofre e exigem reiniciar a importação. `npm ci` foi reproduzido e as auditorias npm completa/produção retornaram zero alertas, contra 36 pacotes afetados antes. Overrides seletivos corrigem ferramentas transitivas e removem `extract-zip`; as 25 entradas de produção do lockfile e a versão `0.0.1` foram preservadas. Nenhum pacote de distribuição foi gerado/publicado e as verificações nativas abertas continuam pendentes.
- A [PR #1](https://github.com/toruticas/TopOTP/pull/1) foi aberta da branch `release/v0.0.1` para `main`. O merge pertence ao usuário; as tarefas de gerar/verificar/publicar a release ficam pendentes até o commit resultante estar na `main`.
- A preparação de `v0.0.1`, solicitada explicitamente pelo usuário, alinhou as versões e acrescentou `check-release-version.mjs` e o workflow manual `Publish macOS release`. A CI do PR executa somente verificações; a geração/publicação de DMGs depende de execução na `main` após o merge pelo usuário. `actionlint` validou ambos os workflows e a checagem de versões passou em `0.0.1`. Os pacotes de produção dessa versão e sua release ainda não foram gerados/publicados.
- A validação local foi repetida com `0.0.1`: 45 testes Rust/interoperabilidade e 17 frontend passaram. As primeiras repetições E2E falharam por cliques antes da habilitação e perda de foco do painel, confirmada pela captura mascarada. O harness passou a aguardar os controles e a usar a reabertura nativa durante esperas limitadas; os quatro casos passaram após esse ajuste, preservando um único clique HOTP, os códigos esperados e o próximo contador. O runtime de produção não foi alterado para acomodar a automação.
- As cinco correções da revisão foram concluídas com 66 testes aprovados: 44 unitários Rust, 1 de interoperabilidade, 17 frontend e 4 E2E no WKWebView com adaptadores sintéticos. Typecheck, ESLint, Prettier, Clippy, rustfmt e validação OpenSpec estrita passaram. O clipboard foi exercitado em pasteboard nativo isolado, incluindo substituição por texto igual/diferente; a exclusão com falha foi testada com armazenamento simulado, journal e nova tentativa no desbloqueio. A regressão upstream reordena envelopes v3 sem recifrar; os testes de exportação verificam limites de tamanho e restauração de seleção menor. Nenhuma credencial pessoal foi acessada, nenhum bundle de produção foi atualizado e as seis verificações manuais anteriores continuam pendentes.
- Touch ID confirmado pelo usuário no `.app` ad hoc; abertura pelo ícone do build final também confirmada.
- 40 testes Rust (incluindo interoperabilidade com funções originais), 12 testes frontend e 4 testes E2E no WKWebView aprovados. Typecheck, lint, Prettier, rustfmt, Clippy e validação OpenSpec aprovados.
- Probes reais Swift/Rust comprovaram Keychain tradicional; outro executável recebeu negativa de acesso. Não foi exigida conta/perfil Apple.
- `.app`/`.dmg` arm64/x86_64 gerados com assinatura ad hoc e inspecionados. Aplicativo arm64 iniciado localmente, sem listener TCP. Intel foi somente compilado.
- Tarefas 2.3/2.4/2.5/5.3/10.4/11.3 permanecem abertas pelas partes que exigem senha/cancelamento, nova autenticação de exportação, sistema bloqueado/suspenso, segundo monitor, VoiceOver, clipboard e HOTP com Keychain real após restart. Código dessas rotas existe; mocks não comprovam esses testes.
- Tarefa 10.5 concluída contra funções originais do commit fixado, preservadas com MIT/checksums. Ida e volta clara/v3, parâmetros, códigos TOTP/HOTP e contador foram comprovados; a extensão completa no navegador não foi executada.
- Tarefa 9.7 reproduziu rejeição parcial com enums numéricos emitidos por `EntryStorage.import`/`backupGetExport` originais. O teste falhou antes da correção; depois, todas as quatro contas da fixture v3 de uma única senha foram importadas pelo WKWebView, com códigos comparados à referência. Também foram verificadas 54 combinações de enums e a rejeição de valores desconhecidos/negativos/fracionários. O arquivo pessoal foi examinado somente estruturalmente, sem senha/descriptografia e sem integrar fixtures ou testes.
- Os bundles corrigidos de importação foram gerados e inspecionados em `.artifacts/import-fix/{aarch64-apple-darwin,x86_64-apple-darwin}/release/bundle/`. O `.app` estável do caminho original foi preservado, incluindo uma cópia exata em `.artifacts/stable-e741ad1/`; a nova versão não foi iniciada sobre o vault pessoal. A validação do backup pessoal exige repetir a importação localmente com a senha do usuário.
- A preparação da beta `0.0.1-beta.1` alinhou os manifests/lockfiles e repetiu os 56 testes automatizados. O primeiro E2E local parou no desbloqueio sintético; a repetição isolada passou sem alterações no código do app. DMGs Apple Silicon/Intel foram gerados e inspecionados com essa versão, e `SHA256SUMS` foi preparado em `.artifacts/releases/v0.0.1-beta.1/assets/`. As notas estão em `docs/releases/v0.0.1-beta.1.md`; as seis verificações manuais anteriores continuam pendentes.
- A [CI passou no commit `6b4907a`](https://github.com/toruticas/TopOTP/actions/runs/38602620026), incluindo o ajuste de posicionamento e retomada. Gatekeeper/quarentena e confiança de ACL após atualizar o binário permanecem documentados como verificações futuras.
- Posicionamento usa pontos AppKit e retomada permanece apenas em memória por menos de 30 s. Os testes cobrem geometria Retina/bordas/origens negativas, busca, formulário, rolagem/foco, seleções, limpeza de senhas/seeds, limite exato e bloqueio. O usuário confirmou manualmente no macOS que o build `6b4907a` abre abaixo do ícone e retoma a mesma tela ao clicar fora e reabrir em menos de 30 segundos: “Sim, os dois funcionam”. A ferramenta de captura continua retornando `cgWindowNotFound`/timeout; execução real em segundo monitor segue pendente.
