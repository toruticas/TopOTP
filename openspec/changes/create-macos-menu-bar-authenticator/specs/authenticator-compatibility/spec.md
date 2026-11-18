## Purpose

Permitir transportar contas TOTP/HOTP entre TopOTP e Authenticator Extension com contratos explícitos de formatos, validação de dados e proteção dos segredos durante importação e exportação.

## ADDED Requirements

### Requirement: In-app backup workflow

O aplicativo MUST oferecer Importar backup e Exportar backup em sua própria interface, reproduzindo o fluxo de uso da extensão: seleção nativa de arquivo/destino, entrada de senhas quando necessária, revisão/seleção de contas, confirmação e resultado compreensível. Os fluxos MUST ser utilizáveis sem comandos de terminal, scripts ou manipulação manual do armazenamento. A senha do backup MUST ser tratada separadamente da autenticação de acesso ao cofre.

#### Scenario: User imports through the app
- **WHEN** o usuário escolhe Importar backup no painel
- **THEN** consegue selecionar o arquivo, informar a senha quando necessária, revisar as contas e confirmar sua importação na interface

#### Scenario: User exports through the app
- **WHEN** o usuário escolhe Exportar backup no painel
- **THEN** consegue selecionar contas/formato, informar e confirmar a senha de backup, autenticar-se nativamente e escolher o destino pela interface

### Requirement: Standard OTP URI input

O app MUST aceitar cadastro por `otpauth://totp/` e `otpauth://hotp/`, preservando emissor, conta e parâmetros suportados. O seed MUST ser obrigatório, HOTP MUST exigir contador explícito, e parâmetros ausentes de TOTP MUST receber os defaults documentados. URIs inválidas, algoritmos desconhecidos e tipos fora do escopo MUST ser rejeitados sem alterar o cofre.

#### Scenario: Valid TOTP URI
- **WHEN** uma URI TOTP válida contém conta, emissor e seed Base32 sem parâmetros opcionais
- **THEN** a prévia apresenta conta/emissor e os defaults SHA-1, 6 dígitos e 30 segundos, sem repetir o seed

#### Scenario: HOTP URI without counter
- **WHEN** uma URI HOTP não contém contador
- **THEN** ela é rejeitada com erro específico e não cria conta

### Requirement: Authenticator JSON interoperability

O app MUST importar objetos JSON de registros `OTPStorage` não criptografados e `EncOTPStorage` associados a `Key` versão 3 do Authenticator Extension de referência. MUST preservar tipo, algoritmo, dígitos, período, próximo contador HOTP, emissor, conta, ordem e fixação quando presentes e válidos, e aplicar defaults documentados quando omitidos. Arquivos v3 com vários identificadores de chave MUST ser suportados com as senhas necessárias fornecidas explicitamente pelo usuário.

#### Scenario: Plain backup with omitted defaults
- **WHEN** um backup não criptografado contém TOTP sem algoritmo, dígitos ou período
- **THEN** a conta importada gera os mesmos códigos que a extensão de referência usando SHA-1, 6 dígitos e 30 segundos

#### Scenario: Encrypted v3 backup
- **WHEN** um backup v3 válido e as senhas de suas chaves são fornecidos
- **THEN** suas entradas suportadas podem ser validadas e importadas sem enviar seeds ou conteúdo descriptografado à camada de apresentação

#### Scenario: Mixed JSON enum representations
- **WHEN** um backup claro ou v3 mistura nomes, strings numéricas e números JSON para tipos TOTP/HOTP e algoritmos SHA1/256/512, como pode ocorrer após importação na extensão original
- **THEN** todas as entradas suportadas são reconhecidas com os mesmos parâmetros e códigos, sem interpretar números desconhecidos, fracionários ou negativos como defaults válidos

#### Scenario: Wrong password or missing key
- **WHEN** a senha não desbloqueia uma chave ou um `keyId` não possui registro `Key` correspondente
- **THEN** o app apresenta erro de importação e não persiste as entradas afetadas nem assume que a importação inteira foi bem-sucedida

### Requirement: Explicit import review and commit

Importar MUST exigir sessão válida, prévia sem seeds e confirmação das entradas selecionadas. A prévia MUST identificar válidas, duplicadas, inválidas e não suportadas, sem descarte silencioso. Duplicatas MUST preservar a conta existente por padrão e MUST NOT reduzir o contador HOTP. Importar um subconjunto suportado MUST exigir escolha explícita; o conjunto confirmado MUST ser persistido atomicamente para a lista visível.

#### Scenario: Mixed supported and unsupported entries
- **WHEN** o backup contém TOTP válido e uma conta Steam
- **THEN** a prévia informa ambos os resultados e somente o TOTP pode ser importado mediante confirmação explícita do subconjunto

#### Scenario: Cancelled preview
- **WHEN** o usuário cancela a prévia ou bloqueia o cofre antes da confirmação
- **THEN** o cofre permanece inalterado e dados transitórios da importação são descartados

#### Scenario: Duplicate HOTP with older counter
- **WHEN** uma conta HOTP existente é reencontrada com contador inferior em um backup
- **THEN** a conta atual é preservada e a importação não reduz seu próximo contador

### Requirement: OTP identity for duplicate detection

A identidade HOTP MUST ignorar período e rótulos quando seed, tipo, algoritmo e dígitos coincidem, inclusive com identificador de origem diferente. O período MUST continuar diferenciando identidades TOTP.

#### Scenario: Reidentified HOTP with a different period
- **WHEN** uma cópia HOTP com a mesma seed, algoritmo e dígitos possui outro identificador e período, e um contador inferior ao existente
- **THEN** ela continua sinalizada como duplicata e não cria outra conta capaz de reutilizar o contador antigo

#### Scenario: Equivalent HOTP entries in one backup
- **WHEN** duas entradas HOTP de um mesmo arquivo diferem somente em origem, rótulos, período ou contador
- **THEN** apenas a primeira entrada é elegível e a segunda é sinalizada como duplicata; confirmar o subconjunto exige reconhecimento explícito

### Requirement: Duplicate revalidation at import commit

O commit MUST revalidar duplicatas contra o cofre atual e o conjunto da prévia dentro da operação autorizada. Falhas de leitura ou validação MUST preservar a prévia sem persistir a seleção.

#### Scenario: Vault changes after the preview
- **WHEN** uma conta equivalente é persistida após a prévia e antes da confirmação
- **THEN** o commit recusa a seleção duplicada, preserva a conta atual e mantém a prévia sem importar parcialmente

#### Scenario: Duplicate revalidation cannot read the vault
- **WHEN** o armazenamento falha durante a revalidação de duplicatas na confirmação
- **THEN** nada é importado e a mesma prévia permanece disponível para nova tentativa após a recuperação

### Requirement: Bounded and versioned format handling

O app MUST rejeitar backups acima de 5 MiB, mais de 1000 entradas OTP, mais de 32 registros de chave, campos de identificação/seed acima de 1024 bytes, payload cifrado por registro acima de 16 KiB ou parâmetros criptográficos acima dos limites documentados antes de executar trabalho não limitado. Versões criptografadas anteriores ao v3 e versões desconhecidas MUST gerar orientação para reexportar pela extensão em formato suportado, sem fallback criptográfico silencioso.

#### Scenario: Oversized or excessive-cost backup
- **WHEN** um arquivo ultrapassa os limites ou solicita parâmetros de derivação fora do envelope suportado
- **THEN** a operação é rejeitada antes de alocar memória ou processar derivações acima dos limites

#### Scenario: Unsupported encrypted format
- **WHEN** o backup usa `OldKey`, registro OTP com `encrypted: true` sem envelope v3, ou versão de chave desconhecida
- **THEN** o app identifica o formato não suportado e explica a necessidade de reexportá-lo, sem importar como texto claro

### Requirement: Explicit compatible export

O app MUST exportar a seleção em JSON compatível com a extensão de referência, preservando parâmetros suportados. O padrão MUST ser v3 criptografado com senha fornecida e confirmada na operação, nunca persistida. Exportar MUST exigir nova autenticação nativa e destino escolhido pelo usuário; o backend MUST gravar e retornar só status. Texto claro MUST exigir escolha separada e reconhecimento explícito de que o arquivo conterá seeds.

#### Scenario: Encrypted round trip
- **WHEN** contas TOTP/HOTP são exportadas com senha em formato v3 e o arquivo é importado pela extensão de referência
- **THEN** a extensão reconhece suas contas, gera TOTP equivalente e usa o mesmo próximo contador HOTP, sem que a exportação consuma contadores

#### Scenario: Fresh authentication denied
- **WHEN** a nova autenticação para exportar é cancelada ou negada
- **THEN** nenhum arquivo contendo contas ou seeds é gravado, mesmo que a sessão anterior estivesse válida

#### Scenario: Plaintext export without acknowledgement
- **WHEN** o usuário escolhe texto claro e não confirma a exposição dos seeds
- **THEN** a exportação é bloqueada

#### Scenario: Destination write fails
- **WHEN** a gravação falha, o usuário cancela a seleção do destino ou a sessão é revogada antes do commit do arquivo
- **THEN** o app não reporta sucesso nem deixa um backup incompleto apresentado como concluído; nenhum arquivo temporário em texto claro é produzido

### Requirement: Documented support boundary

O projeto MUST documentar o repositório/commit de referência, formatos suportados e exclusões. Qualquer código ou asset reaproveitado MUST preservar licença e atribuição. Compatibilidade MUST ser demonstrada com contas sintéticas e casos reproduzíveis, sem credenciais reais do usuário.

#### Scenario: Upstream reference is changed
- **WHEN** a implementação passa a usar outro commit ou amplia um formato
- **THEN** documentação de compatibilidade e casos reproduzíveis são atualizados antes de declarar esse suporte
