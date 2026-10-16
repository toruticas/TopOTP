# Uso e recuperação

O primeiro acesso segue o fluxo normal: clique no ícone e desbloqueie para abrir o painel nativo. Se não houver contas, a lista fica vazia e **Mais** continua disponível. Escolha **Nova entrada**, **Importar** ou **Escanear QR da tela** como primeira ação. Não há cadastro ou importação obrigatória, nem janela de migração automática.

Para trazer contas de outra versão, escolha **Mais → Importar**, selecione seu backup e informe a senha no app quando necessário. Revise as contas antes de confirmar. Backups claros e criptografados suportados usam o mesmo fluxo. A senha não deve ser enviada por chat. Seu cofre anterior permanece preservado; cancelar a importação permite escolher outra ação normalmente.

Para HOTP, exporte imediatamente antes da troca e não gere novos códigos no app anterior após exportar. O arquivo contém o próximo contador; os cofres antigo e novo não sincronizam alterações. Depois de usar HOTP no novo cofre, uma volta ao anterior precisa transferir também os contadores atuais.

Abra TopOTP e clique no ícone na barra de menus. Quando bloqueado, esse clique inicia uma única autenticação do macOS por Touch ID ou senha; a lista abre após aprovação. Cancelar encerra a tentativa. O aplicativo inicia bloqueado; não há senha mestre própria nem conta Apple exigida.

O painel AppKit abre abaixo do ícone e ajusta a altura à quantidade de contas exibidas, inclusive após filtrar. Poucas contas deixam o painel compacto; muitas mantêm o limite de altura anterior e usam rolagem interna. Clicar fora ou pressionar Escape o oculta. Reabrir em menos de 30 segundos retoma busca, rolagem, foco e seleção enquanto a sessão estiver válida. Códigos são limpos ao ocultar e retornam após revalidar a sessão. Esse estado vive somente em memória e é descartado ao completar 30 segundos oculto, bloquear ou expirar a sessão, mesmo que a janela de gerenciamento continue aberta.

A busca nativa filtra emissor/conta sem distinção de caso ou acentos. Fica desativada quando o cofre está vazio e volta a funcionar após adicionar contas; uma pesquisa sem resultados continua editável. Command-F retorna à busca; setas selecionam resultados, Return copia o código selecionado e Tab alcança os controles. O cadeado ao lado de **TopOTP**, acima da busca, bloqueia o cofre. **Mais** expande/recolhe no próprio painel as ações Nova entrada, Escanear QR da tela, Importar, Exportar e Sair. **Configurações** permanece como última opção, fora dessa seção e sempre visível. As ações não usam reticências. A expansão também é retomada ao reabrir em menos de 30 segundos na mesma sessão.

**Configurações** abre uma janela própria com Geral, Contas e os formulários de cadastro/transferência. Ela permanece aberta ao clicar fora ou trocar de aplicativo; Escape também não a fecha. Pode ser redimensionada e compartilha a mesma sessão do painel. Fechar pelo controle da janela a oculta e descarta formulários/prévias, sem encerrar o app. Minimizar limpa campos secretos e códigos, sem aplicar um prazo web para descartar a tela. Os prazos do cofre continuam vigentes: se a sessão expirar, a janela continua aberta com o desbloqueio disponível. Após desbloquear, Geral é retomada. Geral mostra versão e limites vigentes; ainda não há controles para mudar a política de sessão.

## Remover os dados do cofre

Em **Geral → Remover todos os dados**, leia a confirmação, digite `REMOVER` e escolha **Confirmar remoção**. O macOS solicita uma autorização adicional pelo mesmo fluxo de Touch ID/senha usado nas operações sensíveis. Cancelar preserva os dados.

A ação apaga permanentemente contas, chaves, contadores, ordem e favoritos do **cofre atual**, descarta prévias/códigos e limpa a cópia do clipboard se ela ainda pertencer ao TopOTP. A sessão é bloqueada; ao desbloquear novamente, o cofre estará vazio. Para restaurar as contas, importe um backup. Backups externos e o cofre separado de versões anteriores permanecem preservados, conforme indicado na confirmação.

Se a remoção falhar, o app informa que ela não foi concluída e bloqueia. Parte dos dados pode já ter sido excluída. O journal mantém a limpeza pendente para o próximo desbloqueio solicitado pelo usuário, sem retries de armazenamento ou pedidos de senha em segundo plano. Nenhum sucesso é declarado antes de excluir as chaves referenciadas e concluir a limpeza.

Adicione conta manualmente (Base32, TOTP/HOTP e parâmetros) ou escolha URI `otpauth://`, revise a prévia e confirme. Edite por “•••”; deixar “Nova chave” vazia preserva a chave. Exclusão exige confirmação. Busca, estrelas e setas ajudam a organizar as contas. Rótulos são apresentados como texto.

**Copiar** calcula o TOTP atual. HOTP só avança em **Gerar próximo**; copiar novamente não consome outro contador. O incremento é persistido antes de mostrar o código. Uma interrupção entre persistência e exibição pode pular um código; o contador nunca é revertido. Também não pode ser reduzido trocando o tipo da conta.

## Backups

**Importar** abre um seletor nativo. JSON claro e Authenticator v3 são processados no core. Backups com várias chaves pedem as senhas separadamente. Revise as contas/motivos de rejeição e escolha a seleção; importar um subconjunto exige reconhecimento explícito. Duplicatas preservam o existente, inclusive seu contador. Cancelamento/bloqueio apagam a prévia. O arquivo original não é alterado.

**Exportar** seleciona contas e usa v3 criptografado por padrão. Confirme a senha do backup, autentique novamente no macOS e escolha o destino. Guarde essa senha separadamente. Exportação clara exige reconhecimento e um arquivo novo; não sobrescreve um arquivo existente. Backups são gravados pelo core, com permissão 0600, sem JSON devolvido ao webview.

O envelope v3 mantém compatibilidade com a extensão, mas AES-CBC nesse formato não autentica integridade. Preserve o arquivo em local confiável. Não use esse envelope como formato de cofre local.

Steam/Battle.net/hex/GOST, backups criptografados antigos, câmera e Google migration estão fora desta versão. Reexporte formatos antigos pela extensão atual. Limites de backup: 5 MiB, 1000 contas, 32 chaves e orçamento de processamento limitado.

## QR de uma área da tela

Escolha **Mais → Escanear QR da tela** com o cofre desbloqueado. Autorize a captura em Privacidade e Segurança → Gravação da Tela quando solicitado pelo macOS. Se o sistema exigir reiniciar após conceder acesso, reabra o app; recusa não inicia tentativas automáticas.

Arraste uma região sobre um QR TOTP/HOTP; Escape cancela. Cada seleção fica no monitor onde o arraste começou. A captura ocorre após remover a seleção e o painel; é processada localmente em memória, sem arquivo temporário, clipboard ou envio ao webview. Se houver vários QRs, escolha uma conta na revisão e confirme **Salvar conta**. A leitura sozinha não cadastra contas nem avança HOTP; duplicatas preservam o existente.

QR ausente, URI inválida, formato não suportado, permissão negada e limite excedido têm mensagens próprias. Limites: 16 milhões de pixels, 16 QRs, 8 KiB por URI e 15 s de processamento após selecionar. Bloqueio/expiração cancelam a operação e invalidam a revisão. As cópias internas dos frameworks Apple não oferecem garantia de zeroização física, embora seus recursos sejam liberados e payloads Rust controlados pelo app sejam zeroizados.

Esta interface pertence à mudança em desenvolvimento `native-macos-quick-access`; a beta publicada continua disponível como versão anterior. Consulte as [verificações pendentes](native-verification.md) antes de considerar o scanner validado em outro hardware.

## Bloqueio, armazenamento e atualização

Bloquear/Sair removem a autorização e transientes. A sessão expira em 120 s sem interação, 300 s totais ou 30 s com todas as superfícies ocultas. O painel tem seu próprio prazo de retomada de 30 s. A cópia é limpa após 30 s ou bloqueio se a escrita ainda pertencer ao TopOTP; não apaga uma nova escrita de outro app.

Chaves, nomes e contadores ficam em valores do Keychain tradicional; o diretório de dados contém apenas referências opacas/ordem/fixação e journal de recuperação. Na correção do modelo de atualização, um pequeno componente nativo fixo passa a guardar esses itens, enquanto a sessão e o Touch ID continuam no core do app. A ACL/partição do item não impõe biometria independentemente do código autorizado. Strings/cópias transitórias dos frameworks não têm garantia de zeroização física, embora buffers controlados sejam limpos e nada seja persistido no browser.

Desinstalar o `.app` não apaga o Keychain. Preserve também o manifesto ao fazer rollback; não exclua os dados do aplicativo para “corrigir” uma falha. Se uma escrita for interrompida, o journal recupera o conjunto anterior ou o novo inteiro no próximo desbloqueio. Schema desconhecido/referência ausente falha sem apagar contas confirmadas. Não edite contadores ou manifesto manualmente.

Novos builds locais usam certificado persistente e componente de armazenamento fixo, sem notarização. O certificado sozinho não basta para itens do Keychain login que exigem o hash do criador. A passagem do cofre direto ao componente fixo usa outro serviço/manifesto (`vault-broker-v1`), com importação escolhida pelo usuário e preservação integral do anterior. O cofre atual vazio não restringe as ações; não altere ACLs nem apague o cofre anterior para tentar destravar acesso. As próximas atualizações da interface precisam conservar a identidade e o componente; mudar o próprio componente ou arquitetura é uma migração explícita.

Acesso recusado ou transporte indisponível bloqueiam e interrompem as leituras, sem repetir pedidos de senha ou reconectar em segundo plano. A mensagem de recusa não diagnostica sozinha uma mudança de identidade. Um arquivo baixado pode exigir abertura autorizada pelo Finder/Privacidade e Segurança; distribuição pública e Developer ID/notarização permanecem separados dos testes locais. Builds ad hoc de CI não habilitam o cofre e servem somente à inspeção de compilação/empacotamento.
