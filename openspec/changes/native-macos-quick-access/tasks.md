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
- [x] 9.5 Com candidato concreto validado, instalar e orientar importação na UI usando os backups existentes aceitos pelo usuário, que dispensou nova exportação; conferir o resultado pessoal sem inspecionar seus dados. Touch ID sem pedidos insistentes e captura/revisão QR já foram confirmados no harness nativo.

## 10. Limpeza do cofre e ciclo de vida da janela

- [x] 10.1 Adicionar remoção do cofre atual com confirmação textual, autorização nativa fresca, journal recuperável e revogação de estado; informar explicitamente o escopo e falhas parciais.
- [x] 10.2 Manter a janela de gerenciamento aberta ao perder foco, retirar o fechamento por Escape e o prazo web de retomada; preservar bloqueio e limpeza sensível sem fechar a janela.
- [x] 10.3 Testar confirmação/cancelamento, sessão antiga, falha parcial e limpeza completa com dados sintéticos; verificar janela nativa sem foco e executar checks relevantes.
- [x] 10.4 Documentar o comportamento e gerar candidato com a mesma assinatura, verificando compatibilidade antes de atualizar a instalação sem acionar a remoção do cofre pessoal.

## 11. Partições do Keychain login e dono estável

- [x] 11.1 Diagnosticar sem ler valores de credenciais: assinatura igual, Keychain desbloqueado, nenhum journal e partições presas ao CDHash anterior; restaurar o bundle compatível sem alterar o cofre.
- [x] 11.2 Reproduzir a diferença entre chaveiro descartável e login com itens sintéticos; exigir continuidade do dono da partição, além dos requisitos de assinatura, no preflight.
- [x] 11.3 Implementar helper nativo fixo, canal privado herdado com validação dos processos, namespaces derivados do cliente e protocolo limitado; manter autorização no core Rust e LocalAuthentication existente.
- [x] 11.4 Fixar helper por arquitetura no estado local de build, impedir regeneração silenciosa, preservar CDHash no empacotamento e recusar builds incompatíveis.
- [x] 11.5 Integrar o Store, mensagens corretas e cofre separado sem modificar os itens anteriores; preservar cadastro, HOTP, transferências, remoção e comportamento da janela.
- [x] 11.6 Testar troca de builds no login real com dados sintéticos, isolamento de cliente/certificado, pin incorreto do helper, resposta excessiva, encerramento, timeout e falhas/revogação; executar as suítes pertinentes.
- [x] 11.7 Gerar candidato isolado, registrar provas/limites e conduzir transferência pela UI a partir da versão restaurada; só declarar a restauração pessoal após confirmação do usuário.

## 12. Primeiro acesso sem etapa obrigatória

- [x] 12.1 Remover detecção/estado/rota que forçam migração e manter o painel nativo vazio como apresentação normal após autenticar; preservar os cofres e ações de Mais.
- [x] 12.2 Cobrir cofre vazio, escolha explícita de cadastro/importação/QR, cancelamento e menu nativo com dados sintéticos; executar checks pertinentes sem alterar o helper fixo ou o ator de autenticação.
- [x] 12.3 Atualizar documentação, gerar/inspecionar candidato com o mesmo helper e validar a atualização antes de instalar, preservando a versão confirmada pelo usuário.

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

- Complemento 10 validado: 72 testes Rust, um interop, 22 Vue e seis E2E aprovados, incluindo remoção confirmada de contas sintéticas e janela ativa que permanece visível após o Finder tomar foco. Build arm64 assinado com a identidade existente, inspecionado e DMG verificado em `.artifacts/settings-update/`. Preflight bidirecional aprovado; instalação anterior preservada, atualização instalada e processo confirmado. Recibo em `.artifacts/settings-update/installation.json`; cofre pessoal não foi apagado pelo agente. `auth.rs` e `keychain.rs` inalterados neste complemento.

- **Reavaliação de 16/10/2026:** o preflight anterior verificava requisitos designados, mas não o dono exigido pela partição do login. O build `d274a02b…` do complemento 10 foi retirado após a recusa relatada; `5a46add6…`, CDHash `017bfbef…`, foi restaurado sem alterar dados. Os metadados dos itens antigos exigem esse CDHash; o certificado igual não basta. O probe descartável de 9.1/9.2 não qualificava o login padrão. O usuário confirmou a restauração: “Sim, o acesso voltou ao normal”.
- Complemento 11: `scripts/vault-helper-upgrade-probe.py` reproduziu a recusa direta `-25293` no login padrão com mesmo DR e provou leitura/atualização/rollback pelo helper fixo entre clientes Rust diferentes, isolamento de namespace, recusa de pai sem assinatura autorizada e pin incorreto, com cleanup sintético. Nenhum valor pessoal/ACL/trust setting foi lido ou alterado. Relatório `.artifacts/vault-helper-upgrade-probe/report.json`.
- Verificação do complemento 11: 76 testes Rust, um interop, 22 Vue e cinco fluxos E2E com Store/helper/Keychain reais aprovados; auth/clipboard/diálogos E2E sintéticos. O teste de foco foi explicitamente pulado por `com.apple.loginwindow`, sem transformar o desktop bloqueado em prova de foco. Typecheck, ESLint, Prettier, rustfmt e Clippy com `e2e,native-probe` passaram; logs finais em `.artifacts/keychain-broker/final-check-*.log`.
- Candidato arm64 inspecionado e DMG verificado em `.artifacts/keychain-broker/target/`; helper conserva CDHash `72e01246…`. Uma cópia com pai reempacotado/reassinado e helper igual passou no novo preflight nos dois sentidos; direto → helper retorna migração obrigatória. No app **Teste**, o usuário confirmou “Sim, abriu com Touch ID sem erro”; o manifesto sintético contém duas referências.
- Após a confirmação nativa, candidato instalado em `/Applications/TopOTP.app`, com assinatura/pins/inspeção aprovados no destino e versão anterior preservada. Recibo `.artifacts/keychain-broker/installation.json`; metadados dos cofres anteriores inalterados. O usuário confirmou desbloqueio e importação pessoal pela UI: “Sim, desbloqueou e importou normalmente”. Nenhum arquivo, senha ou valor de conta pessoal foi lido pelo agente. 9.5 e 11.7 concluídas; 7.3/7.4 mantêm verificações manuais ainda não demonstradas, sem arquivamento automático.

- Complemento 12: o primeiro acesso passa pelo painel nativo normal, sem detecção/flag/rota de migração; importação, cadastro e QR continuam escolhas explícitas. Passaram 76 testes Rust, um interop, 23 Vue e seis E2E com helper/Keychain reais e auth/clipboard/diálogos sintéticos. O probe AppKit exibiu a lista vazia e comprovou que Nova entrada, Importar e Escanear estão habilitados e disparam seus callbacks. Relatórios em `.artifacts/first-use/`. Helper fixo, adaptadores de Keychain/LocalAuthentication e dados pessoais não foram modificados.
- Build do complemento 12 instalado em `/Applications/TopOTP.app`, SHA-256 `52117f90…`, após inspeção e preflight normal aprovados, com o mesmo helper `72e01246…` e sem migração. App/DMG e checksum em `.artifacts/first-use/`; versão confirmada anterior preservada no rollback. Manifestos/journals opacos permaneceram iguais durante a instalação. Recibo `installation.json`; testes nativos novos usaram somente dados sintéticos, sem reset ou leitura das contas pessoais.
