import { mount } from '@vue/test-utils';
import { expect, it } from 'vitest';
import type { AccountSummary } from '../../lib/ipc';
import ExportForm from './ExportForm.vue';

const account: AccountSummary = {
  id: 'synthetic-account',
  issuer: 'Synthetic',
  account: 'alice',
  kind: 'totp',
  algorithm: 'SHA1',
  digits: 6,
  period: 30,
  nextCounter: '0',
  pinned: false,
};

it('requires a longer password only for new encrypted backups', async () => {
  const wrapper = mount(ExportForm, {
    props: { accounts: [account], busy: false },
  });
  const fields = wrapper.findAll('input[type=password]');
  for (const field of fields) await field.setValue('short');
  expect(wrapper.get('button.primary').attributes()).toHaveProperty('disabled');
  await wrapper.get('form').trigger('submit');
  expect(wrapper.emitted('export')).toBeUndefined();

  const password = 'synthetic-backup-password';
  for (const field of fields) await field.setValue(password);
  expect(wrapper.get('button.primary').attributes()).not.toHaveProperty(
    'disabled',
  );
  await wrapper.get('form').trigger('submit');
  expect(wrapper.emitted('export')).toEqual([
    [[account.id], true, password, password, false],
  ]);
  wrapper.unmount();
});
