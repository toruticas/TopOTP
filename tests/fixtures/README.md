# Fixtures sintéticas

Geradas por `npm run fixtures`, exclusivamente com seeds públicos/sintéticos, pelos pacotes de desenvolvimento CryptoJS 4.2.0 e Argon2 0.45.1, usando o contrato observado no upstream Authenticator commit `9d9660bb73700b3e725800edc22836662d94afac` (MIT). [Fontes e formato](../../docs/upstream-compatibility.md).

Não há trechos de código/ícones upstream copiados. O gerador é original; a dependência CryptoJS reproduz o envelope usado pela extensão. Argon2 usa memória 19456 KiB, tempo 2, paralelismo 1, saída 32 bytes e Base64 sem padding como senha textual do envelope AES. O verificador usa outro Argon2 PHC.

- `authenticator-plain.json`: Unicode/rótulo hostil, TOTP SHA256/45 s/8 dígitos/pinning, HOTP próximo contador 7 e defaults omitidos.
- `authenticator-v3.json`: os mesmos registros, duas chaves/senhas sintéticas e salts determinísticos para reproduzir os arquivos.
- `hotp-reference.json`: HMAC/truncamento calculados por Node crypto para SHA1/256/512.

Senhas públicas: `synthetic-password-a`, `synthetic-password-b`. Não representam credenciais reais.

E2E exporta v3 no Rust; Node Argon2/CryptoJS verifica senha, descriptografa, valida `dataType/hash`, número de contas e contador HOTP seguinte 8. Testes Rust leem as fixtures geradas por JavaScript e comparam contas/códigos. Isso comprova interoperabilidade de codec nos dois sentidos; não comprova execução da extensão completa no navegador.
