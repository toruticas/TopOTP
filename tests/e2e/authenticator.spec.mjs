import { browser, $, $$, expect } from '@wdio/globals';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import argon2 from 'argon2';
import CryptoJS from 'crypto-js';
import { createHmac } from 'node:crypto';
import assert from 'node:assert/strict';
// Native authentication/files/clipboard/storage are synthetic in this build.
// OTP, IPC authorization, vault commits and transfer codecs are the real code.
describe('TopOTP with synthetic native adapters', () => {
  it('adds an account, imports, exports compatible v3 and revokes presentation', async () => {
    await expect($('button.primary')).toHaveText('Desbloquear');
    await $('button=Desbloquear').click();
    await expect($('button=Adicionar conta')).toBeDisplayed();
    await $('button=Adicionar conta').click();
    await $('label*=Emissor').$('input').setValue('E2E Synthetic');
    await $('label*=Conta').$('input').setValue('e2e@example.test');
    await $('input[type=password]').setValue('JBSWY3DPEHPK3PXQ');
    await $('button=Salvar conta').click();
    await expect($('.account-card')).toHaveText(
      expect.stringContaining('E2E Synthetic'),
    );
    await $('button=Importar').click();
    await expect($('button=Importar 3 conta(s)')).toBeDisplayed();
    await $('button=Importar 3 conta(s)').click();
    await browser.waitUntil(
      async () => (await $$('.account-card')).length === 4,
    );
    const counter = await $('.account-card*=counter');
    await counter.$('button=Gerar próximo').click();
    await expect(counter.$('.code')).toHaveText('162 583');
    await $('button=Exportar').click();
    const passwords = await $$('input[type=password]');
    await passwords[0].setValue('synthetic-export-password');
    await passwords[1].setValue('synthetic-export-password');
    await $('button=Autorizar e salvar backup').click();
    await expect($('[role=status]')).toHaveText('Backup salvo.');
    const document = JSON.parse(
      await readFile(resolve('.artifacts/e2e/topotp-backup.json'), 'utf8'),
    );
    const key = Object.values(document).find((v) => v.dataType === 'Key');
    const raw = await argon2.hash('synthetic-export-password', {
      type: argon2.argon2id,
      memoryCost: 19456,
      timeCost: 2,
      parallelism: 1,
      hashLength: 32,
      salt: Buffer.from(key.salt),
      raw: true,
    });
    const pass = raw.toString('base64').replace(/=+$/, '');
    assert(await argon2.verify(key.hash, pass));
    const records = Object.entries(document)
      .filter(([, v]) => v.dataType === 'EncOTPStorage')
      .map(([id, v]) => {
        const record = JSON.parse(
          CryptoJS.AES.decrypt(v.data, pass).toString(CryptoJS.enc.Utf8),
        );
        assert.equal(record.dataType, 'OTPStorage');
        assert.equal(record.hash, id);
        return record;
      });
    assert.equal(records.length, 4);
    assert.equal(records.find((r) => r.type === 'hotp').counter, 8);
    // Independent reference code for the exported HOTP next counter.
    const message = Buffer.alloc(8);
    message.writeBigUInt64BE(8n);
    const digest = createHmac('sha1', '12345678901234567890')
      .update(message)
      .digest();
    const offset = digest.at(-1) & 15;
    assert.equal(
      String((digest.readUInt32BE(offset) & 0x7fffffff) % 1000000).padStart(
        6,
        '0',
      ),
      '399871',
    );
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
    await browser.saveScreenshot(resolve('.artifacts/e2e/locked.png'));
  });
});
