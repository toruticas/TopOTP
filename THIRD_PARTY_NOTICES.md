# Código de terceiros

## Referência de testes: Authenticator Extension

Somente `tests/reference/authenticator/` contém código da [Authenticator-Extension/Authenticator](https://github.com/Authenticator-Extension/Authenticator), commit [`9d9660bb73700b3e725800edc22836662d94afac`](https://github.com/Authenticator-Extension/Authenticator/tree/9d9660bb73700b3e725800edc22836662d94afac). São cópias integrais, sem alterações, de `src/import.ts`, `src/argon.ts`, `src/models/{storage,otp,key-utilities,encryption}.ts` e `LICENSE`. [snapshot.json](tests/reference/authenticator/snapshot.json) registra SHA-256 de cada arquivo, conferido antes de executar a referência.

Esses módulos são usados pelo gerador de fixtures e pelos testes de interoperabilidade; não são incorporados ao aplicativo de produção. O harness adapta o ambiente do navegador e o binding Argon2, mantendo os corpos das funções originais. A licença também está preservada junto às cópias.

```text
MIT License

Copyright (c) 2017 Authenticator Extension

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Dependências do aplicativo

As licenças das dependências Rust/JavaScript empacotadas são geradas por `npm run licenses` em `src-tauri/resources/THIRD_PARTY_LICENSES.txt` e incluídas nos bundles. A licença do TopOTP permanece MIT.
