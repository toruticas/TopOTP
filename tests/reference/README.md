# Implementação original usada nos testes

`authenticator/` preserva seis módulos TypeScript e a licença do upstream fixado em `snapshot.json`. Não reformate nem edite esses arquivos: o harness verifica seus SHA-256. Atualizações exigem revisar a compatibilidade, renovar a proveniência e executar toda a suíte. Consulte [atribuição/licença](../../THIRD_PARTY_NOTICES.md).

`tests/compatibility/reference.mjs` transpila os módulos originais em um contexto VM isolado. Adapta `chrome.storage` em memória, configurações locais, relógio, comunicação entre página e sandbox Argon2 e a API `argon2-browser` para o binding Node. GOST é rejeitado por estar fora do escopo. Remove apenas o bootstrap/imports da interface Vue de `import.ts`; suas funções `decryptBackupData` e `findAndUnlockKey` permanecem intactas.

O teste chama `EntryStorage.import` exatamente como `FileImport.vue`, sem converter enums previamente. Depois lê `EntryStorage.get`, compara parâmetros e códigos, consome/persiste HOTP na referência e gera backups com `EntryStorage.getExport` e `Encryption`. O Rust reimporta esses resultados. A extensão completa não é instalada/aberta no navegador por esse harness.

Executar: `rtk proxy npm run test:interop` (após `npm ci`). Também integra `npm run test:rust` e a CI. Não acessa Keychain nem credenciais reais; todos os inputs são sintéticos.
