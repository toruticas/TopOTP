// Executes the pinned, unmodified upstream algorithms in an isolated browser
// environment. Only browser storage/UI, unavailable GOST, time and the Argon2
// browser binding are adapted; import/export/OTP function bodies are original.
import { readFileSync } from 'node:fs';
import { createHash, randomUUID } from 'node:crypto';
import { createContext, Script } from 'node:vm';
import ts from 'typescript';
import CryptoJS from 'crypto-js';
import argon2 from 'argon2';
import assert from 'node:assert/strict';

const root = new URL('../reference/authenticator/', import.meta.url);
const snapshot = JSON.parse(readFileSync(new URL('snapshot.json', root)));

export function loadReference(unixSeconds = 1234567890) {
  const sources = new Map();
  for (const [file, hash] of Object.entries(snapshot.sha256)) {
    const bytes = readFileSync(new URL(file, root));
    assert.equal(createHash('sha256').update(bytes).digest('hex'), hash, file);
    sources.set(file, bytes.toString());
  }
  let stored = {};
  let milliseconds = unixSeconds * 1000;
  const listeners = [];
  const sandboxListeners = [];
  let loadingSandbox = false;
  const settings = {
    StorageLocation: { Local: 'local', Sync: 'sync' },
    UserSettings: {
      items: { storageLocation: 'local', offset: 0 },
      async updateItems() {},
      commitItems() {},
    },
  };
  const storage = {
    async get(callback) {
      const data = structuredClone(stored);
      if (callback) callback(data);
      return data;
    },
    async set(data) {
      Object.assign(stored, structuredClone(data));
    },
    async remove(keys) {
      for (const key of [keys].flat()) delete stored[key];
    },
    async clear() {
      stored = {};
    },
  };
  const window = {
    addEventListener(type, listener) {
      assert.equal(type, 'message');
      (loadingSandbox ? sandboxListeners : listeners).push(listener);
    },
  };
  const source = {
    postMessage(data) {
      // Deliver sandbox responses to the import module, as the browser does.
      for (const listener of [...listeners]) listener({ data });
    },
  };
  const iframe = {
    contentWindow: {
      postMessage(data) {
        for (const listener of [...sandboxListeners])
          listener({ data, source, origin: 'https://synthetic.test' });
      },
    },
  };
  const argonBinding = {
    ArgonType: { Argon2id: argon2.argon2id },
    async hash(options) {
      return {
        encoded: await argon2.hash(options.pass, {
          type: options.type,
          salt: Buffer.from(options.salt),
          memoryCost: options.mem,
          timeCost: options.time,
          parallelism: options.parallelism,
          hashLength: options.hashLen,
          version: 19,
        }),
      };
    },
    async verify(options) {
      if (!(await argon2.verify(options.encoded, options.pass)))
        throw new Error('Invalid synthetic password');
    },
  };
  class ReferenceDate extends Date {
    constructor(...args) {
      super(...(args.length ? args : [milliseconds]));
    }
    static now() {
      return milliseconds;
    }
  }
  const context = createContext({
    crypto: { randomUUID },
    chrome: { storage: { local: storage, sync: storage }, runtime: {} },
    window,
    document: {
      getElementById(id) {
        assert.equal(id, 'argon-sandbox');
        return iframe;
      },
    },
    Date: ReferenceDate,
    setTimeout,
    console: { warn() {}, error() {}, log() {} },
  });
  const cache = new Map();
  function load(file) {
    if (cache.has(file)) return cache.get(file).exports;
    const original = sources.get(file);
    assert(original, `Unpinned module: ${file}`);
    const parsed = ts.createSourceFile(file, original, ts.ScriptTarget.Latest);
    // import.ts also launches the Vue import page. Avoid UI bootstrap and its
    // imports while retaining the exact original function bodies under test.
    const uiImports = new Set([
      'vue',
      './components/Import.vue',
      './components/common/index',
      './store/i18n',
      './models/migration',
    ]);
    const code = parsed.statements
      .filter((statement) => {
        if (file !== 'src/import.ts') return true;
        if (ts.isImportDeclaration(statement))
          return !uiImports.has(statement.moduleSpecifier.text);
        return !(
          ts.isExpressionStatement(statement) &&
          statement.expression.getText(parsed) === 'init()'
        );
      })
      .map((statement) => statement.getFullText(parsed))
      .join('\n');
    const compiled = ts.transpileModule(code, {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2022,
        esModuleInterop: true,
      },
    }).outputText;
    const module = { exports: {} };
    cache.set(file, module);
    function require(specifier) {
      if (specifier === 'crypto-js') return CryptoJS;
      if (specifier === 'argon2-browser') return argonBinding;
      if (specifier === './settings') return settings;
      if (specifier === 'node-gost-crypto')
        return {
          gostEngine: {
            getGostDigest() {
              throw new Error('GOST is outside the supported reference scope');
            },
          },
        };
      assert(specifier.startsWith('./'), `Unexpected import: ${specifier}`);
      return load(
        new URL(`${specifier}.ts`, `file:///${file}`).pathname.slice(1),
      );
    }
    loadingSandbox = file === 'src/argon.ts';
    new Script(`(function(require, module, exports) {${compiled}\n})`, {
      filename: file,
    }).runInContext(context)(require, module, module.exports);
    loadingSandbox = false;
    return module.exports;
  }
  load('src/argon.ts');
  return {
    ...load('src/models/storage.ts'),
    ...load('src/models/otp.ts'),
    ...load('src/models/encryption.ts'),
    ...load('src/import.ts'),
    ...load('src/models/key-utilities.ts'),
    setTime(seconds) {
      milliseconds = seconds * 1000;
    },
    reset() {
      stored = {};
    },
    snapshot: () => structuredClone(stored),
    async derive(password, salt) {
      // Use the actual Argon class/options from the snapshot via its sandbox.
      return new Promise((resolve) => {
        const reply = {
          postMessage(data) {
            resolve(data.response);
          },
        };
        for (const listener of [...sandboxListeners])
          listener({
            data: { action: 'hash', value: password, salt },
            source: reply,
            origin: 'https://synthetic.test',
          });
      });
    },
  };
}
