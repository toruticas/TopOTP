import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import ImportForm from './ImportForm.vue';
import type { AccountSummary, ImportPreview } from '../../lib/ipc';

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

function renderPreview(
  origin: string,
  overrides: Partial<AccountSummary> = {},
) {
  const preview: ImportPreview = {
    origin,
    id: 'synthetic-preview',
    keys: [],
    rows: [
      {
        sourceId: 'synthetic-source',
        account: { ...account, ...overrides },
        issue: null,
      },
    ],
  };
  return mount(ImportForm, {
    props: { preview, busy: false, progress: '' },
  });
}

describe('import parameter review', () => {
  it('shows the TOTP URI defaults before confirming the account', () => {
    const wrapper = renderPreview('uri');
    expect(wrapper.text()).toContain('Synthetic · alice');
    expect(wrapper.text()).toContain('TOTP · SHA1');
    expect(wrapper.text()).toContain('6 dígitos');
    expect(wrapper.text()).toContain('Período: 30 s');
    expect(wrapper.text()).not.toContain('Próximo contador');
    expect(wrapper.emitted('commit')).toBeUndefined();
    wrapper.unmount();
  });

  it.each([7, 8])(
    'shows %i digits and the custom TOTP backup period',
    (digits) => {
      const wrapper = renderPreview('backup', {
        digits,
        algorithm: 'SHA512',
        period: 45,
      });
      expect(wrapper.text()).toContain('TOTP · SHA512');
      expect(wrapper.text()).toContain(`${digits} dígitos`);
      expect(wrapper.text()).toContain('Período: 45 s');
      wrapper.unmount();
    },
  );

  it('shows the complete HOTP counter without presenting a time period', () => {
    const counter = '9007199254740991';
    const wrapper = renderPreview('backup', {
      kind: 'hotp',
      digits: 8,
      nextCounter: counter,
      period: 86400,
    });
    expect(wrapper.text()).toContain('HOTP · SHA1');
    expect(wrapper.text()).toContain('8 dígitos');
    expect(wrapper.text()).toContain(`Próximo contador: ${counter}`);
    expect(wrapper.text()).not.toContain('Período');
    expect(wrapper.text()).not.toContain('86400');
    wrapper.unmount();
  });
});
