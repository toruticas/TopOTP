import { mount } from '@vue/test-utils';
import { it, expect } from 'vitest';
import AccountForm from './AccountForm.vue';
it('sends the new seed once then clears the password field', async () => {
  const wrapper = mount(AccountForm, { props: { account: null, busy: false } });
  const input = wrapper.get('input[type="password"]');
  await input.setValue('JBSWY3DPEHPK3PXP');
  await wrapper.get('form').trigger('submit');
  expect(wrapper.emitted('save')?.[0]?.[0]).toMatchObject({
    secret: 'JBSWY3DPEHPK3PXP',
  });
  expect((input.element as HTMLInputElement).value).toBe('');
  wrapper.unmount();
});
