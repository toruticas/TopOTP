import { browser, $, $$, expect } from '@wdio/globals';
import { readFile, copyFile, mkdir, rm, stat } from 'node:fs/promises';
import { resolve } from 'node:path';
import assert from 'node:assert/strict';
import { loadReference } from '../compatibility/reference.mjs';
import { verifyInOriginal } from '../compatibility/roundtrip.mjs';

// Native authentication/dialogs/clipboard/storage are synthetic in this build.
// WKWebView, UI, IPC authorization, OTP, vault commits and codecs are real.
const artifacts = resolve('.artifacts/e2e');
const output = resolve(artifacts, 'topotp-backup.json');
const password = 'Senha sintética 🔐 com espaços';
const fixture = JSON.parse(
  await readFile(resolve('tests/fixtures/authenticator-plain.json'), 'utf8'),
);
const definitions = Object.values(fixture);
definitions.push({
  issuer: 'E2E Synthetic',
  account: 'e2e@example.test',
  secret: 'JBSWY3DPEHPK3PXQ',
  type: 'totp',
});

async function presentation() {
  return browser.executeAsync((done) => {
    const invoke = globalThis.__TAURI_INTERNALS__.invoke;
    (async () => {
      const { epoch } = await invoke('vault_status');
      return {
        accounts: await invoke('list_accounts', { epoch }),
        codes: await invoke('get_totp_codes', { epoch }),
      };
    })().then(done, (error) => done({ error: error.code }));
  });
}

async function clearSyntheticVault() {
  const result = await browser.executeAsync((done) => {
    const invoke = globalThis.__TAURI_INTERNALS__.invoke;
    (async () => {
      const { epoch } = await invoke('vault_status');
      const accounts = await invoke('list_accounts', { epoch });
      for (const account of accounts)
        await invoke('delete_account', { epoch, accountId: account.id });
      return 'cleared';
    })().then(done, (error) => done(error.code));
  });
  assert.equal(result, 'cleared');
  await $('button=Bloquear').click();
  await $('button=Desbloquear').click();
  await expect($('button=Adicionar conta')).toBeDisplayed();
  assert.equal((await $$('.account-card')).length, 0);
}

async function beginImport(source) {
  await copyFile(resolve(source), resolve(artifacts, 'import-source.json'));
  await $('button=Importar').click();
  await expect($('h2=Importar backup')).toBeDisplayed();
}

async function commit(count) {
  await $(`button=Importar ${count} conta(s)`).click();
  await browser.waitUntil(
    async () => (await $$('.account-card')).length === count,
  );
}

async function fillPasswords(values) {
  const fields = await $$('input[type=password]');
  assert.equal(fields.length, values.length);
  for (const [index, value] of values.entries())
    await fields[index].setValue(value);
  await $('button=Abrir backup').click();
}

function expectedRecords(accounts) {
  const ref = loadReference();
  return accounts.map((account) => {
    const original = definitions.find(
      (item) => item.account === account.account,
    );
    assert(original);
    const expected = {
      ...account,
      kind: original.type,
      algorithm: original.algorithm ?? 'SHA1',
      digits: original.digits ?? 6,
      period: original.period ?? 30,
      pinned: original.pinned ?? false,
      secret: original.secret,
      counter: Number(account.nextCounter),
    };
    for (const field of ['kind', 'algorithm', 'digits', 'period', 'pinned'])
      assert.equal(
        account[field],
        expected[field],
        `${account.account}: ${field}`,
      );
    const generate = () =>
      ref.KeyUtilities.generate(
        ref.OTPType[expected.kind],
        expected.secret,
        expected.counter,
        expected.period,
        expected.digits,
        ref.OTPAlgorithm[expected.algorithm],
        0,
      );
    expected.code = generate();
    expected.codes = [59, 1234567890].map((time) => {
      ref.setTime(time);
      return { time, code: generate() };
    });
    return expected;
  });
}

async function checkNativeCodes() {
  const data = await presentation();
  assert(!data.error);
  const records = expectedRecords(data.accounts);
  const ref = loadReference();
  for (const value of data.codes) {
    const record = records.find((item) => item.id === value.id);
    if (record.kind !== 'totp') continue;
    ref.setTime(value.validUntil - 1);
    assert.equal(
      value.code,
      ref.KeyUtilities.generate(
        ref.OTPType.totp,
        record.secret,
        0,
        record.period,
        record.digits,
        ref.OTPAlgorithm[record.algorithm],
        0,
      ),
    );
  }
  return records;
}

async function exportBackup(encrypted) {
  await rm(output, { force: true });
  const before = (await presentation()).accounts;
  await $('button=Exportar').click();
  if (encrypted) {
    const fields = await $$('input[type=password]');
    await fields[0].setValue(password);
    await fields[1].setValue('does-not-match');
    await expect($('button=Autorizar e salvar backup')).toBeDisabled();
    await fields[1].setValue(password);
  } else {
    // Embedded WKWebView's option-click does not dispatch change reliably.
    // Use the DOM change event that the real select sends to Vue.
    await browser.execute(
      (select) => {
        select.selectedIndex = 1;
        select.dispatchEvent(new Event('change', { bubbles: true }));
      },
      await $('select'),
    );
    await expect($('label.warning')).toBeDisplayed();
    await expect($('button=Autorizar e salvar backup')).toBeDisabled();
    await $('label.warning input').click();
  }
  await $('button=Autorizar e salvar backup').click();
  await expect($('[role=status]')).toHaveText('Backup salvo.');
  assert.equal((await stat(output)).mode & 0o777, 0o600);
  const after = (await presentation()).accounts;
  assert.deepEqual(after, before, 'Export must not consume HOTP counters');
  const backup = JSON.parse(await readFile(output, 'utf8'));
  await verifyInOriginal(
    backup,
    encrypted ? password : null,
    expectedRecords(after),
  );
  await copyFile(
    output,
    resolve(artifacts, `${encrypted ? 'encrypted' : 'plain'}-export.json`),
  );
}

describe('TopOTP transfer interoperability with synthetic native adapters', () => {
  before(async () => {
    await mkdir(artifacts, { recursive: true });
    await expect($('button=Desbloquear')).toBeDisplayed();
    await $('button=Desbloquear').click();
    await expect($('button=Adicionar conta')).toBeDisplayed();
  });

  it('imports plain JSON and exports both formats accepted by original import', async () => {
    await $('button=Adicionar conta').click();
    await $('label*=Emissor').$('input').setValue('E2E Synthetic');
    await $('label*=Conta').$('input').setValue('e2e@example.test');
    await $('input[type=password]').setValue('JBSWY3DPEHPK3PXQ');
    await $('button=Salvar conta').click();
    await beginImport('tests/fixtures/authenticator-plain.json');
    assert.equal((await $$('input[type=password]')).length, 0);
    await $('button=Importar 3 conta(s)').click();
    await browser.waitUntil(
      async () => (await $$('.account-card')).length === 4,
    );
    await checkNativeCodes();
    const counter = await $('.account-card*=counter');
    await counter.$('button=Gerar próximo').click();
    await expect(counter.$('.code')).toHaveText('162 583');
    assert.equal(
      (await presentation()).accounts.find((a) => a.kind === 'hotp')
        .nextCounter,
      '8',
    );
    await exportBackup(true);
    await exportBackup(false);
  });

  it('imports original v3 with multiple keys, rejects wrong passwords and cancels', async () => {
    await clearSyntheticVault();
    await beginImport('tests/fixtures/authenticator-v3.json');
    await expect($('button=Importar 0 conta(s)')).toBeDisabled();
    await fillPasswords(['wrong-password', 'wrong-password']);
    await expect($('[role=alert]')).toHaveText('Senha do backup incorreta.');
    for (const field of await $$('input[type=password]'))
      assert.equal(await field.getValue(), '');
    await $('button=Cancelar').click();
    assert.equal((await presentation()).accounts.length, 0);
    await beginImport('tests/fixtures/authenticator-v3.json');
    await fillPasswords(['synthetic-password-a', 'synthetic-password-b']);
    await expect($('button=Importar 3 conta(s)')).toBeEnabled();
    await commit(3);
    await checkNativeCodes();
    assert.equal(
      (await presentation()).accounts.find((a) => a.kind === 'hotp')
        .nextCounter,
      '7',
    );
  });

  it('reimports its plain/v3 exports and keeps a newer HOTP counter on duplicates', async () => {
    for (const encrypted of [false, true]) {
      await clearSyntheticVault();
      await beginImport(
        resolve(artifacts, `${encrypted ? 'encrypted' : 'plain'}-export.json`),
      );
      if (encrypted) await fillPasswords([password]);
      await expect($('button=Importar 4 conta(s)')).toBeEnabled();
      await commit(4);
      await checkNativeCodes();
      const counter = await $('.account-card*=counter');
      await counter.$('button=Gerar próximo').click();
      await expect(counter.$('.code')).toHaveText('399 871');
      await beginImport(
        resolve(artifacts, `${encrypted ? 'encrypted' : 'plain'}-export.json`),
      );
      if (encrypted) await fillPasswords([password]);
      await expect($('button=Importar 0 conta(s)')).toBeDisabled();
      assert.equal((await $$('small*=Duplicata: existente mantida')).length, 4);
      await $('button=Cancelar').click();
      assert.equal(
        (await presentation()).accounts.find((a) => a.kind === 'hotp')
          .nextCounter,
        '9',
      );
    }
    await $('button=Bloquear').click();
    await expect($('button=Desbloquear')).toBeDisplayed();
    assert.equal((await $$('.account-card')).length, 0);
    const rejected = await browser.executeAsync((done) => {
      globalThis.__TAURI_INTERNALS__
        .invoke('list_accounts', { epoch: '1', authenticated: true })
        .then(
          () => done({ accepted: true }),
          (error) => done({ code: error.code }),
        );
    });
    assert.equal(rejected.code, 'Locked');
    const denied = await browser.executeAsync((done) => {
      globalThis.__TAURI_INTERNALS__.invoke('plugin:dialog|open', {}).then(
        () => done('allowed'),
        (error) => done(String(error)),
      );
    });
    assert.match(denied, /not allowed|denied/);
    await browser.saveScreenshot(resolve(artifacts, 'locked.png'));
  });
});
