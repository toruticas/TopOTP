// Synthetic inputs; v3 entries are serialized by the pinned original routines.
import { mkdir, writeFile } from 'node:fs/promises';
import { createHmac } from 'node:crypto';
import CryptoJS from 'crypto-js';
import { loadReference } from '../tests/compatibility/reference.mjs';
const reference = loadReference();
const root = new URL('../tests/fixtures/', import.meta.url);
await mkdir(root, { recursive: true });
const plain = {
  '11111111-1111-4111-8111-111111111111': {
    dataType: 'OTPStorage',
    encrypted: false,
    secret: 'GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ',
    type: 'totp',
    hash: '11111111-1111-4111-8111-111111111111',
    index: 1,
    issuer: 'Sintético & <teste>',
    account: 'alice@example.test',
    algorithm: 'SHA256',
    digits: 8,
    period: 45,
    pinned: true,
  },
  '22222222-2222-4222-8222-222222222222': {
    encrypted: false,
    secret: 'GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ',
    type: 'hotp',
    hash: '22222222-2222-4222-8222-222222222222',
    index: 0,
    issuer: 'Synthetic',
    account: 'counter',
    counter: 7,
  },
  '33333333-3333-4333-8333-333333333333': {
    secret: 'JBSWY3DPEHPK3PXP',
    type: 'totp',
    issuer: 'Synthetic',
    account: 'defaults',
    hash: '33333333-3333-4333-8333-333333333333',
    index: 2,
  },
};
async function encryptedEntry(id, record, keyId, password, doc) {
  const salt = keyId;
  const derived = (await reference.derive(password, salt)).split('$')[5];
  const hash = await reference.derive(derived, 'synthetic-verifier-salt');
  doc[keyId] = { dataType: 'Key', id: keyId, salt, hash, version: 3 };
  const entry = new reference.OTPEntry(
    {
      ...record,
      type: reference.OTPType[record.type],
      algorithm: reference.OTPAlgorithm[record.algorithm ?? 'SHA1'],
    },
    new reference.Encryption(derived, keyId),
  );
  // Deterministic randomness only for this fixture generator; the original
  // export/AES implementation and inner encrypted:true marker are unchanged.
  const random = CryptoJS.lib.WordArray.random;
  try {
    CryptoJS.lib.WordArray.random = (size) => {
      if (size !== 8) throw new Error('Unexpected reference salt size');
      return CryptoJS.enc.Hex.parse('0102030405060708');
    };
    doc[id] = reference.EntryStorage.getExport([entry], true)[id];
  } finally {
    CryptoJS.lib.WordArray.random = random;
  }
}
const encrypted = {};
for (const [i, [id, record]] of Object.entries(plain).entries())
  await encryptedEntry(
    id,
    record,
    `aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa${i % 2}`,
    i % 2 ? 'synthetic-password-b' : 'synthetic-password-a',
    encrypted,
  );
await writeFile(
  new URL('authenticator-plain.json', root),
  JSON.stringify(plain, null, 2) + '\n',
);
await writeFile(
  new URL('authenticator-v3.json', root),
  JSON.stringify(encrypted, null, 2) + '\n',
);
// The original importer can persist numeric `type` values when importing a
// named enum. backupGetExport preserves those encrypted records verbatim.
// Exercise that real path, with normal and imported entries sharing one key.
reference.reset();
const importKeyId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const importPassword = 'synthetic-import-password';
const importDerived = (
  await reference.derive(importPassword, importKeyId)
).split('$')[5];
const importEncryption = new reference.Encryption(importDerived, importKeyId);
await reference.BrowserStorage.set({
  [importKeyId]: {
    dataType: 'Key',
    id: importKeyId,
    salt: importKeyId,
    hash: await reference.derive(importDerived, 'synthetic-verifier-salt'),
    version: 3,
  },
});
const originalRandom = CryptoJS.lib.WordArray.random;
try {
  CryptoJS.lib.WordArray.random = (size) => {
    if (size !== 8) throw new Error('Unexpected reference salt size');
    return CryptoJS.enc.Hex.parse('0102030405060708');
  };
  for (const record of Object.values(plain)) {
    await reference.EntryStorage.add(
      new reference.OTPEntry(
        {
          ...record,
          type: reference.OTPType[record.type],
          algorithm: reference.OTPAlgorithm[record.algorithm ?? 'SHA1'],
        },
        importEncryption,
      ),
    );
  }
  const importedId = '44444444-4444-4444-8444-444444444444';
  await reference.EntryStorage.import(importEncryption, {
    [importedId]: {
      dataType: 'OTPStorage',
      encrypted: false,
      hash: importedId,
      index: 3,
      type: 'totp',
      secret: 'MZXW6YTBOI',
      issuer: 'Synthetic',
      account: 'imported-numeric-kind',
    },
  });
  const importedBackup = await reference.EntryStorage.backupGetExport(
    importEncryption,
    true,
  );
  const importedRaw = importEncryption.decryptEncSecret({
    encData: importedBackup[importedId].data,
  });
  if (importedRaw?.type !== 1)
    throw new Error('Original importer no longer reproduces numeric type');
  await writeFile(
    new URL('authenticator-v3-imported.json', root),
    JSON.stringify(importedBackup, null, 2) + '\n',
  );
} finally {
  CryptoJS.lib.WordArray.random = originalRandom;
}
const variants = [];
for (const algorithm of ['sha1', 'sha256', 'sha512'])
  for (const counter of [0, 7, 123456]) {
    const message = Buffer.alloc(8);
    message.writeBigUInt64BE(BigInt(counter));
    const digest = createHmac(algorithm, '12345678901234567890')
      .update(message)
      .digest();
    const offset = digest.at(-1) & 15;
    const number = (digest.readUInt32BE(offset) & 0x7fffffff) % 1000000;
    variants.push({
      algorithm: algorithm.toUpperCase(),
      counter,
      code: String(number).padStart(6, '0'),
    });
  }
await writeFile(
  new URL('hotp-reference.json', root),
  JSON.stringify(variants, null, 2) + '\n',
);
console.log('Synthetic reference fixtures generated.');
