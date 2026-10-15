import { mount } from '@vue/test-utils';
import { expect, it } from 'vitest';
import ImportForm from './ImportForm.vue';
import type { AccountSummary } from '../../lib/ipc';

const account: AccountSummary = {
  id: 'synthetic-one',
  issuer: 'Serviço sintético',
  account: 'alice@example.invalid',
  kind: 'totp',
  algorithm: 'SHA1',
  digits: 6,
  period: 30,
  nextCounter: null,
  pinned: false,
};
it('chooses exactly one supported QR while retaining rejected candidates as text', async () => {
  const wrapper = mount(ImportForm, {
    props: {
      busy: false,
      progress: '',
      preview: {
        origin: 'qr',
        id: 'synthetic-preview',
        keys: [],
        rows: [
          { sourceId: 'one', account, issue: null },
          {
            sourceId: 'two',
            account: {
              ...account,
              id: 'synthetic-two',
              account: '<script>hostile</script>',
              kind: 'hotp',
              nextCounter: '7',
            },
            issue: null,
          },
          { sourceId: 'bad', account: null, issue: 'UnsupportedFormat' },
          {
            sourceId: 'duplicate',
            account: { ...account, id: 'duplicate' },
            issue: 'DuplicateKept',
          },
        ],
      },
    },
  });
  expect(wrapper.find('input[type=password]').exists()).toBe(false);
  expect(wrapper.find('input[type=checkbox]').exists()).toBe(false);
  const radios = wrapper.findAll('input[type=radio]');
  expect((radios[0]!.element as HTMLInputElement).checked).toBe(true);
  expect((radios[2]!.element as HTMLInputElement).disabled).toBe(true);
  await radios[1]!.setValue(true);
  expect((radios[0]!.element as HTMLInputElement).checked).toBe(false);
  expect(wrapper.find('script').exists()).toBe(false);
  expect(wrapper.text()).toContain('Formato não suportado');
  await wrapper.get('button.primary').trigger('click');
  expect(wrapper.emitted('commit')).toEqual([[['synthetic-two'], true]]);
  wrapper.unmount();
});
