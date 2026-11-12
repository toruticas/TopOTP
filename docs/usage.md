# Uso e recuperação

Abra TopOTP e clique no ícone na barra de menus. **Desbloquear** abre a autenticação do macOS por Touch ID ou senha. O aplicativo inicia bloqueado; não há senha mestre própria nem conta Apple exigida.

O painel abre imediatamente abaixo do ícone. Clicar fora ou pressionar Escape o oculta. Reabrir em menos de 30 segundos retoma a tela, busca, rolagem, foco, seleções e campos não secretos, enquanto a sessão estiver válida. Senhas, chaves/URIs digitadas e códigos são limpos ao ocultar; códigos atuais retornam após revalidar a sessão. O estado é temporário em memória e descartado ao completar 30 segundos oculto, bloquear ou expirar a sessão.

Adicione conta manualmente (Base32, TOTP/HOTP e parâmetros) ou escolha URI `otpauth://`, revise a prévia e confirme. Edite por “•••”; deixar “Nova chave” vazia preserva a chave. Exclusão exige confirmação. Busca, estrelas e setas ajudam a organizar as contas. Rótulos são apresentados como texto.

A prévia informa tipo, algoritmo e dígitos, além do período TOTP ou próximo contador HOTP. Se uma exclusão retirar a conta da lista, mas a remoção do segredo ainda não puder ser confirmada no Keychain, o app apresenta um aviso. Bloqueie e desbloqueie para repetir a limpeza pendente; preserve os dados do aplicativo até ela ser concluída.

**Copiar** calcula o TOTP atual. HOTP só avança em **Gerar próximo**; copiar novamente não consome outro contador. O incremento é persistido antes de mostrar o código. Uma interrupção entre persistência e exibição pode pular um código; o contador nunca é revertido. Também não pode ser reduzido trocando o tipo da conta.

## Backups

**Importar** abre um seletor nativo. JSON claro e Authenticator v3 são processados no core. Backups com várias chaves pedem as senhas separadamente. Revise as contas/motivos de rejeição e escolha a seleção; importar um subconjunto exige reconhecimento explícito. Duplicatas preservam o existente, inclusive seu contador. Cancelamento/bloqueio apagam a prévia. O arquivo original não é alterado.

**Exportar** seleciona contas e usa v3 criptografado por padrão. Confirme a senha do backup, autentique novamente no macOS e escolha o destino. Guarde essa senha separadamente. Exportação clara exige reconhecimento e um arquivo novo; não sobrescreve um arquivo existente. Backups são gravados pelo core, com permissão 0600, sem JSON devolvido ao webview.

O envelope v3 mantém compatibilidade com a extensão, mas AES-CBC nesse formato não autentica integridade. Preserve o arquivo em local confiável. Não use esse envelope como formato de cofre local.

Steam/Battle.net/hex/GOST, backups criptografados antigos, QR/câmera e Google migration estão fora desta versão. Reexporte formatos antigos pela extensão atual. Limites: 5 MiB, 1000 contas, 32 chaves e orçamento de processamento limitado.

A exportação também verifica os limites de arquivo e de cada registro antes de gravar. Se o backup exceder o limite, tente exportar menos contas por vez. Nomes de conta/emissor muito longos podem exigir redução; não altere chaves secretas para reduzir o tamanho de um backup.

## Bloqueio, armazenamento e atualização

Bloquear/Sair removem a autorização e transientes. A sessão expira em 120 s sem interação, 300 s totais ou 30 s oculto. A cópia é limpa após 30 s ou bloqueio se a escrita ainda pertencer ao TopOTP; não apaga uma nova escrita de outro app.

Chaves, nomes e contadores ficam em valores do Keychain tradicional; o diretório de dados contém apenas referências opacas/ordem/fixação e journal de recuperação. Touch ID controla a sessão do app; a ACL do item não impõe biometria independentemente dele. Strings transitórias do JavaScript/frameworks não têm garantia de zeroização física, embora formulários sejam limpos e nada seja persistido no browser.

Desinstalar o `.app` não apaga o Keychain. Preserve também o manifesto ao fazer rollback; não exclua os dados do aplicativo para “corrigir” uma falha. Se uma escrita for interrompida, o journal recupera o conjunto anterior ou o novo inteiro no próximo desbloqueio. Schema desconhecido/referência ausente falha sem apagar contas confirmadas. Não edite contadores ou manifesto manualmente.

Builds locais têm assinatura ad hoc, sem notarização. Um arquivo baixado pode exigir a abertura autorizada pelo Finder/Privacidade e Segurança; a distribuição pública ainda não foi verificada. Ao trocar o binário, o Keychain pode pedir confirmação de confiança/senha por mudança da assinatura. Nunca selecione acesso geral para todos os aplicativos. Developer ID/notarização ficam como opção futura de distribuição.
