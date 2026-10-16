import { mount } from '@vue/test-utils';
import { describe, it, expect } from 'vitest';
import ResetVault from './ResetVault.vue';

describe('vault removal confirmation', () => {
  it('requires the exact confirmation and allows cancellation without removal', async () => {
    const wrapper = mount(ResetVault, { props: { busy: false } });
    await wrapper.get('button').trigger('click');
    expect(wrapper.text()).toContain('versões anteriores não são apagados');
    expect(
      wrapper.get('button[type=submit]').attributes('disabled'),
    ).toBeDefined();
    await wrapper.get('input').setValue('remover');
    await wrapper.get('form').trigger('submit');
    expect(wrapper.emitted('remove')).toBeUndefined();
    await wrapper.get('input').setValue('REMOVER');
    await wrapper.get('button[type=button]').trigger('click');
    expect(wrapper.find('form').exists()).toBe(false);
    expect(wrapper.emitted('remove')).toBeUndefined();
    await wrapper.get('button').trigger('click');
    expect((wrapper.get('input').element as HTMLInputElement).value).toBe('');
    await wrapper.get('input').setValue('REMOVER');
    await wrapper.get('form').trigger('submit');
    expect(wrapper.emitted('remove')).toEqual([['REMOVER']]);
    expect((wrapper.get('input').element as HTMLInputElement).value).toBe('');
    await wrapper.setProps({ busy: true });
    expect(
      wrapper.get('button[type=submit]').attributes('disabled'),
    ).toBeDefined();
  });
});
