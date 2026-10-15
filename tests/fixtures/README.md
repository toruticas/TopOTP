# Fixtures sintéticas

Geradas por `npm run fixtures`, exclusivamente com seeds públicos/sintéticos, pelos pacotes de desenvolvimento CryptoJS 4.2.0 e Argon2 0.45.1, usando o contrato observado no upstream Authenticator commit `9d9660bb73700b3e725800edc22836662d94afac` (MIT). [Fontes e formato](../../docs/upstream-compatibility.md).

O gerador define contas sintéticas e executa `OTPEntry`, `EntryStorage.getExport`, `EntryStorage.import`, `EntryStorage.backupGetExport`, `Encryption` e o sandbox Argon2 originais de `tests/reference/`, com licença/checksums preservados. Apenas a aleatoriedade dos salts AES é fixada para reproduzir os arquivos; o aplicativo usa salts aleatórios. Argon2 usa memória 19456 KiB, tempo 2, paralelismo 1, saída 32 bytes e Base64 sem padding como senha textual do envelope AES. O verificador usa outro Argon2 PHC.

- `authenticator-plain.json`: Unicode/rótulo hostil, TOTP SHA256/45 s/8 dígitos/pinning, HOTP próximo contador 7 e defaults omitidos.
- `authenticator-v3.json`: os mesmos registros, duas chaves/senhas sintéticas e salts determinísticos, exportados pelas rotinas originais. O registro interno contém `encrypted: true`, como no upstream.
- `authenticator-v3-imported.json`: as três contas anteriores e um TOTP sintético adicional, com uma única chave; `EntryStorage.import` e `backupGetExport` originais produzem tipos em texto e número no mesmo backup. Reproduz uma importação antes parcial, sem usar o arquivo pessoal recebido.
- `hotp-reference.json`: HMAC/truncamento calculados por Node crypto para SHA1/256/512.

Senhas públicas: `synthetic-password-a`, `synthetic-password-b`. Não representam credenciais reais.

A fixture `authenticator-v3-imported.json` usa a senha pública `synthetic-import-password`. O TOTP adicional tem o seed público de demonstração `MZXW6YTBOI` (`foobar`).

`npm run test:interop` valida ida e volta entre codecs Rust e importador/exportador originais nos dois formatos, SHA1/256/512, 6–8 dígitos, períodos, Unicode, ordem/pinning e códigos/contador HOTP. E2E importa essas fixtures pelo app, exporta claro/v3, passa os arquivos ao importador original e reimporta no TopOTP. Não comprova execução da extensão completa no navegador nem autenticação biométrica real.
