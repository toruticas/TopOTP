import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { loadReference } from './reference.mjs';

export async function verifyInOriginal(backup, password, expected) {
  const ref = loadReference();
  const decrypted = await ref.decryptBackupData(
    structuredClone(backup),
    password,
  );
  assert.equal(Object.keys(decrypted).length, expected.length);
  // This is the exact method called by FileImport.vue, with no conversion of
  // type/algorithm added by the harness before import.
  await ref.EntryStorage.import(new ref.Encryption('', ''), decrypted);
  const entries = await ref.EntryStorage.get();
  assert.equal(entries.length, expected.length);
  for (const [index, record] of expected.entries()) {
    const entry = entries[index];
    assert.equal(
      entry.type,
      ref.OTPType[record.kind],
      `${record.account}: type`,
    );
    assert.equal(
      entry.algorithm,
      ref.OTPAlgorithm[record.algorithm],
      `${record.account}: algorithm`,
    );
    for (const field of ['issuer', 'account', 'digits', 'period', 'pinned'])
      assert.equal(entry[field], record[field], `${record.account}: ${field}`);
    assert.equal(entry.index, index);
    assert.equal(entry.secret, record.secret);
    if (record.kind === 'hotp') {
      assert.equal(entry.counter, record.counter);
      entry.generate();
      assert.equal(entry.code, record.code);
      await entry.next();
      assert.equal(entry.code, record.code);
      assert.equal(entry.counter, record.counter + 1);
      const persisted = (await ref.EntryStorage.get()).find(
        (candidate) => candidate.hash === entry.hash,
      );
      assert.equal(persisted.counter, record.counter + 1);
      assert.equal(entry.code, record.code);
      // Return to the imported counter only in the isolated synthetic browser
      // store so the returned backup represents the original input unchanged.
      entry.counter = record.counter;
      await entry.update();
    } else {
      for (const vector of record.codes) {
        ref.setTime(vector.time);
        entry.generate();
        assert.equal(
          entry.code,
          vector.code,
          `${record.account}: ${vector.time}`,
        );
      }
    }
  }
  return { ref, entries };
}

export async function originalBackups(ref, entries, password) {
  const id = randomUUID();
  const salt = randomUUID();
  const derived = (await ref.derive(password, salt)).split('$')[5];
  const key = {
    dataType: 'Key',
    id,
    salt,
    hash: await ref.derive(derived, randomUUID()),
    version: 3,
  };
  const encryption = new ref.Encryption(derived, id);
  for (const entry of entries) entry.changeEncryption(encryption);
  const plain = ref.EntryStorage.getExport(entries, false);
  const encrypted = { ...ref.EntryStorage.getExport(entries, true), [id]: key };
  await ref.BrowserStorage.set(encrypted);
  const lockedEntries = await ref.EntryStorage.get();
  for (const entry of lockedEntries)
    entry.index = lockedEntries.length - 1 - entry.index;
  await ref.EntryStorage.set(lockedEntries);
  const reorderedEncrypted = await ref.EntryStorage.backupGetExport(
    encryption,
    true,
  );
  for (const [hash, entry] of Object.entries(reorderedEncrypted)) {
    if (entry.dataType === 'EncOTPStorage')
      assert.equal(entry.data, encrypted[hash].data);
  }
  return { plain, encrypted, reorderedEncrypted };
}
