## 1. Preparação e protótipo nativo isolado

- [x] 1.1 Registrar a base `530bf1c`/`v0.0.1-beta.1`, preservar a instalação estável e configurar os artefatos/contas sintéticas em namespace de testes, mantendo os lockfiles e as ferramentas fixadas.
- [x] 1.2 Criar o módulo AppKit de acesso rápido e um protótipo com dados sintéticos; comprovar ancoragem no ícone, foco editável na busca e uso de Mais sem fechamento prematuro antes de integrar leituras do cofre.

## 2. Coordenação de ativação e sessão

- [x] 2.1 Extrair o caso de uso compartilhado de desbloqueio, preservando o adaptador de LocalAuthentication, o comprovante nativo, o guard de diálogo e a autorização por geração de sessão; não alterar ACL/formato dos itens existentes. A migração deliberada usa namespace próprio em 9.3.
- [x] 2.2 Implementar a intenção de apresentação com uma única tentativa por ativação explícita; sucesso vigente abre o painel, cancelamento encerra a tentativa e foco/timers/notificações nunca iniciam outro prompt.
- [x] 2.3 Implementar visibilidade agregada do painel/janela/transições e prazo independente de retomada do painel; preservar limites de 30/120/300 segundos e impedir renovação por ocultações repetidas ou polling.
- [x] 2.4 Centralizar alterações de estado para painel e janela, limpando ambas ao bloquear/expirar/suspender e descartando callbacks/resultados de gerações e intenções antigas.
- [x] 2.5 Testar com relógio e autenticação sintéticos cliques repetidos, cancelamento seguido de eventos, aprovação atrasada, foco do diálogo, handoff entre superfícies e limites exatos de sessão/retomada.

## 3. Painel de OTP com controles AppKit

- [x] 3.1 Integrar a lista nativa ao core autorizado, com ordem/fixação, emissor/conta, TOTP/validade e atualização somente enquanto visível; manter leituras fora da thread principal e validar a geração antes de atualizar views.
- [x] 3.2 Implementar busca nativa por emissor/conta com equivalência de caso/acentos, sem recentes persistidos; adicionar estados vazio/sem resultados/erro, teclado, foco e nomes acessíveis.
- [x] 3.3 Ligar linha TOTP à cópia atual e manter Gerar próximo/Copiar explícitos para HOTP; testar fronteira de TOTP, erro de persistência e ausência de consumo HOTP por filtro/reabertura.
- [x] 3.4 Implementar Mais com todas as ações especificadas e Bloquear/Sair independentes da janela web; integrar toggle, Escape, clique externo e navegação pelo menu sem usar perda genérica de foco para reabrir.
- [x] 3.5 Preservar busca/rolagem/seleção/foco em memória por menos de 30 segundos, limpar códigos ao ocultar e descartar a retomada no limite ou em revogação; verificar composição/ações das views com dados sintéticos.

## 4. Janela independente de gerenciamento

- [x] 4.1 Criar a janela única `management`, com decorações/redimensionamento e navegação Geral/Contas/Transferências; exibir versão/regras vigentes e reutilizar a janela em ações repetidas.
- [x] 4.2 Implementar rotas tipadas pendentes até a UI estar pronta para Nova entrada, Importar, Exportar e revisão QR; reaproveitar formulários, edição/exclusão e ordenação existentes sem alterar os codecs.
- [x] 4.3 Restringir IPC/capabilities à janela/origem local de gerenciamento, atualizar bindings e retirar o acesso web do painel substituído; testar cliente não autorizado e ausência de segredos nas respostas.
- [x] 4.4 Integrar foco, fechamento, minimização e limpeza de campos/prévias com a sessão compartilhada; preservar autenticação adicional de exportação e testar cancelamento/bloqueio durante operações e transições.

## 5. Seleção e captura de uma área da tela

- [x] 5.1 Adicionar/fixar somente os bindings necessários de ScreenCaptureKit/Vision e tipos auxiliares, com disponibilidade compatível com macOS 13 e sem automação em produção.
- [x] 5.2 Implementar seleção AppKit por monitor com arraste/Escape, restrição ao monitor inicial e conversão consistente de pontos/pixels; testar Retina, escalas distintas, bordas e origens negativas.
- [x] 5.3 Implementar solicitação explícita da permissão e captura em memória com caminho macOS 14+ e frame único de stream no macOS 13, removendo overlays/painel antes do frame e encerrando recursos em todas as saídas.
- [x] 5.4 Aplicar limites de operação/pixels/tempo, revalidar sessão após esperas e testar recusa de permissão, cancelamento, timeout e bloqueio durante captura sem arquivo, clipboard ou prompt automático.

## 6. Leitura de QR e revisão autorizada

- [x] 6.1 Integrar Vision para QR e validar payloads pelo parser OTP existente, com limites de quantidade/comprimento e mensagens distintas para QR ausente, conteúdo inválido, formato não suportado e limites excedidos.
- [x] 6.2 Implementar prévia nativa por UUID/geração, escolha de um candidato em regiões com vários QRs e revisão na janela sem seed/URI/imagem; confirmar pelo commit existente, preservando duplicatas e contador HOTP.
- [x] 6.3 Criar imagens sintéticas reproduzíveis e testar TOTP/HOTP, Unicode, múltiplos QRs, payload hostil, falha de armazenamento e revogação durante decodificação/revisão/commit; provar descarte e ausência de escrita secreta fora do Keychain.

## 7. Regressão, integração e verificação nativa

- [x] 7.1 Migrar testes Vue/WebdriverIO para a janela `management` e preservar os fluxos claros/v3, senhas incorretas, enums numéricos, duplicatas, exportação/reimportação e resultados OTP contra a referência original.
- [x] 7.2 Executar typecheck, lint, formatação, testes frontend/Rust/interop, Clippy e E2E; registrar falhas/limites reais sem equiparar a automação WKWebView à cobertura AppKit.
- [ ] 7.3 Validar no `.app` isolado clique bloqueado, Touch ID/senha/cancelamento, ausência de prompts repetidos, painel após sucesso, busca/teclado/VoiceOver, Mais, clique externo e retomada em menos de 30 segundos; registrar evidências e pendências.
- [ ] 7.4 Validar captura real com permissão concedida/negada, cancelamento e revisão de QR sintético, incluindo segundo monitor; verificar bloqueio/suspensão com painel e janela abertos, sem usar o vault pessoal para o teste.

## 8. Documentação e builds isolados

- [x] 8.1 Atualizar README/AGENTS.md/guias de desenvolvimento, uso, compatibilidade e verificação nativa com as duas superfícies, permissão do scanner e fronteiras de teste; manter pendências da mudança original identificadas.
- [x] 8.2 Atualizar o inspetor de produção para a capability de gerenciamento e gerar/inspecionar `.app`/`.dmg` arm64/x86_64 em artefatos separados, preservando dados/instalação estável e distinguindo compilação de execução real em Intel/macOS 13. Após a regressão, novos bundles foram assinados com certificado local persistente, inspecionados e verificados por hdiutil. O preflight passa entre os builds com certificado; a passagem inicial do ad hoc exige a migração escolhida em 9.5.
- [x] 8.3 Validar a mudança OpenSpec, revisar o diff contra alterações indevidas no Keychain/LocalAuthentication/codec e registrar somente tarefas comprovadamente concluídas, sem publicar nova versão ou arquivar pendências manuais.

## 9. Continuidade do Keychain e migração escolhida

- [x] 9.1 Reproduzir a recusa entre dois builds ad hoc com itens sintéticos em Keychain descartável; provar acesso/atualização/rollback entre builds com certificado local e negação a outro identificador, sem conta Apple/trust settings/contas pessoais.
- [x] 9.2 Criar/reutilizar identidade local persistente com chave restrita ao codesign, integrar ao build, repetir a prova sintética com a identidade efetiva e inspecionar os bundles assinados.
- [x] 9.3 Implementar cofre separado, detecção do legado e orientação de migração usando o backup criptografado da UI estável; testar cancelamento, falha e transferência de ordem/favoritos/parâmetros/contadores atuais.
- [x] 9.4 Impedir UI própria do Keychain, retornar recusa distinta e revogar sessão sem retries; testar limpeza/no-read e validar LocalAuthentication no shell nativo com cofre sintético isolado.
- [ ] 9.5 Com candidato concreto validado, instalar e orientar importação na UI usando os backups existentes aceitos pelo usuário, que dispensou nova exportação; conferir o resultado pessoal sem inspecionar seus dados. Touch ID sem pedidos insistentes e captura/revisão QR já foram confirmados no harness nativo.

## Evidências em andamento

- Base `530bf1c` e bundles anteriores preservados; hashes em `.artifacts/native-quick-access/baseline/baseline.json`.
- Protótipo AppKit isolado passou em ancoragem, foco/entrada de teclado no campo nativo, busca com acentos, ação de cópia sintética e manutenção do painel durante o menu Mais. Relatório em `.artifacts/native-quick-access/native-probe.json`. Não utiliza Keychain nem biometria.
- Na extração inicial, `auth.rs`/`keychain.rs` foram preservados. Após reproduzir a recusa entre identidades, `keychain.rs` foi atualizado para impedir UI própria e distinguir acesso recusado, conforme 9.4; `auth.rs`, codecs e snapshots upstream não mudaram.
- Checks finais em 15/10/2026: build/typecheck, ESLint, Prettier, rustfmt, Clippy (incluindo `native-ui-probe`), 61 testes unitários Rust, interoperabilidade original nos dois sentidos, 17 testes Vue e quatro fluxos E2E de gerenciamento aprovados. Corridas de rota e perda de seleção detectadas pelos testes foram corrigidas e revalidadas.
- Vision leu dois QRs TOTP/HOTP sintéticos sem captura de tela. Permissão, timeout e revogação têm testes de lógica; não são evidência de captura real.
- Probe AppKit final aprovado: ancoragem, busca/teclado, Command-F, Mais, máscara de códigos, retomada de busca/rolagem/seleção/cursor antes de 30 s e descarte no limite. Relógio do harness ajustado; composição visual completa e VoiceOver permanecem manuais.
- `.app` e `.dmg` arm64/x86_64 gerados em `.artifacts/native-quick-access/target/`; assinatura ad hoc, versão/arquitetura, capability, assets frontend atuais e ausência de automação/Node/Chromium conferidos. DMGs verificados com `hdiutil verify`; checksums em `.artifacts/native-quick-access/SHA256SUMS`. Hashes dos bundles estáveis continuam iguais à baseline.
- Após pedido explícito, o build foi instalado para teste humano. O usuário relatou regressão de senhas insistentes: 7.3 falhou, e o candidato foi retirado. O estável anterior foi restaurado em `/Applications` e reaberto pelo caminho original; dados do cofre não foram alterados pelo rollback. 7.4 continua pendente.
- O usuário confirmou que o Touch ID voltou ao normal após o rollback. A restauração está validada; o candidato ad hoc permanece retirado. O novo candidato com certificado local foi gerado/inspecionado em 8.2; 7.3 mantém somente as verificações ainda não comprovadas.
- A identidade ad hoc estável é `cdhash 66788b…`; a do candidato, `cdhash c7e990…`. O candidato falha no requisito de assinatura do estável, apesar do mesmo bundle ID e dos fontes de Keychain/auth inalterados. Esta incompatibilidade foi comprovada por `codesign`, sem abrir itens do Keychain; sua ligação com o diálogo relatado permanece hipótese fundamentada, sem confirmação do título do diálogo.
- `scripts/verify-update-identity.py` recusa identidade incompatível e aceita builds com o certificado persistente nos dois sentidos. O probe real de Keychain passou também com a identidade efetiva: dez leituras, atualização, retorno ao primeiro build e recusa ao identificador estrangeiro. A passagem ad hoc→certificado permanece uma migração separada.
- Após a correção: 65 testes Rust, um de interoperabilidade, 18 testes Vue e quatro E2E passaram; bundles/DMGs arm64/Intel assinados, inspecionados e verificados em `.artifacts/native-quick-access/signed-target/`.
- O usuário confirmou no TopOTP Teste Nativo: “Touch ID funcionou e a lista ficou aberta sem repetir senha” e “A revisão do QR apareceu corretamente”. São provas nativas com contas públicas sintéticas no macOS arm64 atual; não comprovam VoiceOver, recusa/Escape reais, segundo monitor, suspensão ou execução Intel/macOS 13.
- O usuário confirmou possuir backups e dispensou nova exportação antes da instalação. O agente não verificou conteúdo/frescor dos arquivos pessoais; a importação deve ocorrer pela UI.

- Instalação do candidato com certificado persistente concluída em `/Applications/TopOTP.app`, após o usuário dispensar nova exportação e aceitar usar seus backups. Bundle estável preservado; nenhum dado antigo removido. Restauração pessoal é feita pelo usuário na UI, sem leitura de arquivos/senhas pelo agente; 9.5 permanece aberta para essa confirmação.
