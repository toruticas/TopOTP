// Independent reference using the upstream algorithms, no upstream code copied.
import { mkdir, writeFile } from 'node:fs/promises';
import { createHmac } from 'node:crypto';
import argon2 from 'argon2';
import CryptoJS from 'crypto-js';
const root = new URL('../tests/fixtures/', import.meta.url);
await mkdir(root, { recursive: true });
const options = {
  type: argon2.argon2id,
  memoryCost: 19456,
  timeCost: 2,
  parallelism: 1,
  hashLength: 32,
  version: 19,
};
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
  const raw = await argon2.hash(password, {
    ...options,
    salt: Buffer.from(salt),
    raw: true,
  });
  const derived = raw.toString('base64').replace(/=+$/, '');
  const hash = await argon2.hash(derived, {
    ...options,
    salt: Buffer.from('synthetic-verifier-salt'),
  });
  doc[keyId] = { dataType: 'Key', id: keyId, salt, hash, version: 3 };
  doc[id] = {
    dataType: 'EncOTPStorage',
    keyId,
    data: CryptoJS.AES.encrypt(
      JSON.stringify({ ...record, dataType: 'OTPStorage', encrypted: false }),
      derived,
      { salt: CryptoJS.enc.Hex.parse('0102030405060708') },
    ).toString(),
    index: record.index,
  };
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
