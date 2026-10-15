# Uso e recuperação

Se esta versão encontrar um cofre da beta anterior, ela abre **Traga suas contas para esta versão** depois do desbloqueio. Use o backup criptografado exportado pela versão estável, selecione o arquivo no app, informe sua senha e revise todas as contas antes de confirmar. A senha não deve ser enviada por chat. Seu cofre anterior permanece preservado.

Para HOTP, exporte imediatamente antes da troca e não gere novos códigos no app anterior após exportar. O arquivo contém o próximo contador; os cofres antigo e novo não sincronizam alterações. Depois de usar HOTP no novo cofre, uma volta ao anterior precisa transferir também os contadores atuais.

Abra TopOTP e clique no ícone na barra de menus. Quando bloqueado, esse clique inicia uma única autenticação do macOS por Touch ID ou senha; a lista abre após aprovação. Cancelar encerra a tentativa. O aplicativo inicia bloqueado; não há senha mestre própria nem conta Apple exigida.

O painel AppKit abre abaixo do ícone. Clicar fora ou pressionar Escape o oculta. Reabrir em menos de 30 segundos retoma busca, rolagem, foco e seleção enquanto a sessão estiver válida. Códigos são limpos ao ocultar e retornam após revalidar a sessão. Esse estado vive somente em memória e é descartado ao completar 30 segundos oculto, bloquear ou expirar a sessão, mesmo que a janela de gerenciamento continue aberta.

A busca nativa filtra emissor/conta sem distinção de caso ou acentos. Command-F retorna à busca; setas selecionam resultados, Return copia o código selecionado e Tab alcança os controles. **Mais** abre Bloquear, Nova entrada, Escanear QR da tela, Importar, Exportar, Configurações e Sair.

**Configurações** abre uma janela própria com Geral, Contas e os formulários de cadastro/transferência. Ela permanece aberta ao clicar fora, pode ser redimensionada e compartilha a mesma sessão do painel. Fechar oculta a janela e descarta formulários/prévias, sem encerrar o app. Minimizar limpa campos secretos. Geral mostra versão e limites vigentes; ainda não há controles para mudar a política de sessão.

Adicione conta manualmente (Base32, TOTP/HOTP e parâmetros) ou escolha URI `otpauth://`, revise a prévia e confirme. Edite por “•••”; deixar “Nova chave” vazia preserva a chave. Exclusão exige confirmação. Busca, estrelas e setas ajudam a organizar as contas. Rótulos são apresentados como texto.

**Copiar** calcula o TOTP atual. HOTP só avança em **Gerar próximo**; copiar novamente não consome outro contador. O incremento é persistido antes de mostrar o código. Uma interrupção entre persistência e exibição pode pular um código; o contador nunca é revertido. Também não pode ser reduzido trocando o tipo da conta.

## Backups

**Importar** abre um seletor nativo. JSON claro e Authenticator v3 são processados no core. Backups com várias chaves pedem as senhas separadamente. Revise as contas/motivos de rejeição e escolha a seleção; importar um subconjunto exige reconhecimento explícito. Duplicatas preservam o existente, inclusive seu contador. Cancelamento/bloqueio apagam a prévia. O arquivo original não é alterado.

**Exportar** seleciona contas e usa v3 criptografado por padrão. Confirme a senha do backup, autentique novamente no macOS e escolha o destino. Guarde essa senha separadamente. Exportação clara exige reconhecimento e um arquivo novo; não sobrescreve um arquivo existente. Backups são gravados pelo core, com permissão 0600, sem JSON devolvido ao webview.

O envelope v3 mantém compatibilidade com a extensão, mas AES-CBC nesse formato não autentica integridade. Preserve o arquivo em local confiável. Não use esse envelope como formato de cofre local.

Steam/Battle.net/hex/GOST, backups criptografados antigos, câmera e Google migration estão fora desta versão. Reexporte formatos antigos pela extensão atual. Limites de backup: 5 MiB, 1000 contas, 32 chaves e orçamento de processamento limitado.

## QR de uma área da tela

Escolha **Mais → Escanear QR da tela…** com o cofre desbloqueado. Autorize a captura em Privacidade e Segurança → Gravação da Tela quando solicitado pelo macOS. Se o sistema exigir reiniciar após conceder acesso, reabra o app; recusa não inicia tentativas automáticas.

Arraste uma região sobre um QR TOTP/HOTP; Escape cancela. Cada seleção fica no monitor onde o arraste começou. A captura ocorre após remover a seleção e o painel; é processada localmente em memória, sem arquivo temporário, clipboard ou envio ao webview. Se houver vários QRs, escolha uma conta na revisão e confirme **Salvar conta**. A leitura sozinha não cadastra contas nem avança HOTP; duplicatas preservam o existente.

QR ausente, URI inválida, formato não suportado, permissão negada e limite excedido têm mensagens próprias. Limites: 16 milhões de pixels, 16 QRs, 8 KiB por URI e 15 s de processamento após selecionar. Bloqueio/expiração cancelam a operação e invalidam a revisão. As cópias internas dos frameworks Apple não oferecem garantia de zeroização física, embora seus recursos sejam liberados e payloads Rust controlados pelo app sejam zeroizados.

Esta interface pertence à mudança em desenvolvimento `native-macos-quick-access`; a beta publicada continua disponível como versão anterior. Consulte as [verificações pendentes](native-verification.md) antes de considerar o scanner validado em outro hardware.

## Bloqueio, armazenamento e atualização

Bloquear/Sair removem a autorização e transientes. A sessão expira em 120 s sem interação, 300 s totais ou 30 s com todas as superfícies ocultas. O painel tem seu próprio prazo de retomada de 30 s. A cópia é limpa após 30 s ou bloqueio se a escrita ainda pertencer ao TopOTP; não apaga uma nova escrita de outro app.

Chaves, nomes e contadores ficam em valores do Keychain tradicional; o diretório de dados contém apenas referências opacas/ordem/fixação e journal de recuperação. Touch ID controla a sessão do app; a ACL do item não impõe biometria independentemente dele. Strings transitórias do JavaScript/frameworks não têm garantia de zeroização física, embora formulários sejam limpos e nada seja persistido no browser.

Desinstalar o `.app` não apaga o Keychain. Preserve também o manifesto ao fazer rollback; não exclua os dados do aplicativo para “corrigir” uma falha. Se uma escrita for interrompida, o journal recupera o conjunto anterior ou o novo inteiro no próximo desbloqueio. Schema desconhecido/referência ausente falha sem apagar contas confirmadas. Não edite contadores ou manifesto manualmente.

Novos builds locais usam um certificado persistente, sem notarização. A primeira passagem da beta ad hoc usa a migração por backup descrita acima; atualizações seguintes precisam manter a mesma identidade. Acesso recusado ao Keychain bloqueia a sessão e interrompe as leituras, sem repetir pedidos de senha. Não amplie a confiança dos itens para contornar uma recusa. Um arquivo baixado pode exigir abertura autorizada pelo Finder/Privacidade e Segurança; distribuição pública e Developer ID/notarização permanecem separados dos testes locais.
