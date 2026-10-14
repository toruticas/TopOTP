import { mount, flushPromises } from '@vue/test-utils';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import App from './App.vue';
import AccountForm from '../features/accounts/AccountForm.vue';
import ImportForm from '../features/transfer/ImportForm.vue';
import ExportForm from '../features/transfer/ExportForm.vue';
import { api } from '../lib/ipc';
import type { AccountSummary, VaultStatus } from '../lib/ipc';
const listeners = vi.hoisted(() => new Map<string, () => void>());
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (name: string, callback: () => void) => {
    listeners.set(name, callback);
    return () => listeners.delete(name);
  }),
}));
vi.mock('../lib/ipc', async (importOriginal) => {
  const original = await importOriginal<typeof import('../lib/ipc')>();
  return {
    ...original,
    api: Object.fromEntries(
      Object.keys(original.api).map((key) => [
        key,
        vi.fn(async () => undefined),
      ]),
    ),
  };
});
const account: AccountSummary = {
  id: 'synthetic',
  issuer: '<img src=x onerror=alert(1)>',
  account: 'alice',
  kind: 'hotp',
  algorithm: 'SHA1',
  digits: 6,
  period: 30,
  nextCounter: '7',
  pinned: false,
};
let wrapper: ReturnType<typeof mount> | undefined;
beforeEach(() => {
  vi.clearAllMocks();
  listeners.clear();
  vi.mocked(api.status).mockResolvedValue({ phase: 'locked', epoch: '1' });
  vi.mocked(api.list).mockResolvedValue([account]);
  vi.mocked(api.codes).mockResolvedValue([]);
});
afterEach(() => {
  wrapper?.unmount();
  wrapper = undefined;
  vi.useRealTimers();
});
describe('vault presentation boundaries', () => {
  it('starts locked without loading accounts or rendering credentials', async () => {
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    expect(wrapper.text()).toContain('Desbloquear');
    expect(api.list).not.toHaveBeenCalled();
    expect(wrapper.text()).not.toContain('alice');
  });
  it('discards a late authentication success after a native lock event', async () => {
    let resolve!: (s: VaultStatus) => void;
    vi.mocked(api.unlock).mockReturnValue(
      new Promise((r) => {
        resolve = r;
      }),
    );
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    await wrapper.get('button.primary').trigger('click');
    listeners.get('vault-changed')?.();
    await flushPromises();
    resolve({ phase: 'unlocked', epoch: '2' });
    await flushPromises();
    expect(wrapper.text()).not.toContain('alice');
    expect(api.list).not.toHaveBeenCalled();
  });
  it('renders hostile labels as text and never consumes HOTP on render', async () => {
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '2' });
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    expect(wrapper.text()).toContain(account.issuer);
    expect(wrapper.find('img[src="x"]').exists()).toBe(false);
    expect(api.next).not.toHaveBeenCalled();
    const button = wrapper
      .findAll('button')
      .find((b) => b.text() === 'Gerar próximo');
    await button!.trigger('click');
    await flushPromises();
    expect(api.next).toHaveBeenCalledOnce();
  });
  it('observes native authentication finishing after a hide and quick reopen', async () => {
    vi.useFakeTimers({
      toFake: [
        'setTimeout',
        'clearTimeout',
        'setInterval',
        'clearInterval',
        'performance',
      ],
    });
    let finish!: (status: VaultStatus) => void;
    vi.mocked(api.unlock).mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    await wrapper.get('button.primary').trigger('click');
    listeners.get('panel-hidden')?.();
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocking', epoch: '2' });
    listeners.get('panel-shown')?.();
    await flushPromises();
    expect(wrapper.text()).toContain('Autenticando');
    finish({ phase: 'unlocked', epoch: '2' });
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '2' });
    await flushPromises();
    await vi.advanceTimersByTimeAsync(1000);
    expect(wrapper.text()).toContain('alice');
    expect(wrapper.text()).not.toContain('Autenticando');
  });
  it('retains search on quick reopen but masks the hidden panel', async () => {
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '2' });
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    await wrapper.get('input[type=search]').setValue('alice');
    listeners.get('panel-hidden')?.();
    await flushPromises();
    expect(wrapper.get('main').isVisible()).toBe(false);
    expect(api.next).not.toHaveBeenCalled();
    listeners.get('panel-shown')?.();
    await flushPromises();
    expect(wrapper.get('main').isVisible()).toBe(true);
    expect(
      (wrapper.get('input[type=search]').element as HTMLInputElement).value,
    ).toBe('alice');
    expect(wrapper.text()).toContain('alice');
  });
  it('preserves an account draft, scroll and focus at 29.999 seconds, clearing its seed', async () => {
    vi.useFakeTimers({
      toFake: [
        'setTimeout',
        'clearTimeout',
        'setInterval',
        'clearInterval',
        'performance',
      ],
    });
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '2' });
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    await wrapper
      .findAll('button')
      .find((b) => b.text() === '+ Conta')!
      .trigger('click');
    const form = wrapper.findComponent(AccountForm);
    const instance = form.element;
    const inputs = form.findAll('input');
    await inputs[0]!.setValue('Draft issuer');
    await inputs[1]!.setValue('Draft account');
    await form.get('input[type=password]').setValue('JBSWY3DPEHPK3PXP');
    (inputs[1]!.element as HTMLInputElement).focus();
    const scroll = document.scrollingElement ?? document.documentElement;
    scroll.scrollTop = 160;
    listeners.get('panel-hidden')?.();
    await flushPromises();
    expect(
      (form.get('input[type=password]').element as HTMLInputElement).value,
    ).toBe('');
    scroll.scrollTop = 0;
    await vi.advanceTimersByTimeAsync(29_999);
    listeners.get('panel-shown')?.();
    await flushPromises();
    expect(wrapper.findComponent(AccountForm).element).toBe(instance);
    expect((inputs[0]!.element as HTMLInputElement).value).toBe('Draft issuer');
    expect((inputs[1]!.element as HTMLInputElement).value).toBe(
      'Draft account',
    );
    expect(scroll.scrollTop).toBe(160);
    expect(document.activeElement).toBe(inputs[1]!.element);
  });
  it('expires the hidden draft at 30 seconds without extending it on repeated hides', async () => {
    vi.useFakeTimers({
      toFake: [
        'setTimeout',
        'clearTimeout',
        'setInterval',
        'clearInterval',
        'performance',
      ],
    });
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '2' });
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    await wrapper
      .findAll('button')
      .find((b) => b.text() === '+ Conta')!
      .trigger('click');
    listeners.get('panel-hidden')?.();
    await vi.advanceTimersByTimeAsync(20_000);
    listeners.get('panel-hidden')?.();
    await vi.advanceTimersByTimeAsync(10_000);
    expect(wrapper.findComponent(AccountForm).exists()).toBe(false);
    vi.mocked(api.status).mockResolvedValue({ phase: 'locked', epoch: '3' });
    listeners.get('panel-shown')?.();
    await flushPromises();
    expect(wrapper.text()).toContain('Desbloquear');
    expect(wrapper.text()).not.toContain('alice');
  });
  it('drops a retained draft immediately when the native session is revoked', async () => {
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '2' });
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    await wrapper
      .findAll('button')
      .find((b) => b.text() === '+ Conta')!
      .trigger('click');
    listeners.get('panel-hidden')?.();
    await flushPromises();
    vi.mocked(api.status).mockResolvedValue({ phase: 'locked', epoch: '3' });
    listeners.get('vault-changed')?.();
    await flushPromises();
    expect(wrapper.findComponent(AccountForm).exists()).toBe(false);
    listeners.get('panel-shown')?.();
    await flushPromises();
    expect(wrapper.text()).toContain('Desbloquear');
  });
  it('retains export selections and format but clears backup passwords on hide', async () => {
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '2' });
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    await wrapper
      .findAll('button')
      .find((b) => b.text() === 'Exportar')!
      .trigger('click');
    const form = wrapper.findComponent(ExportForm);
    await form.get('input[type=checkbox]').setValue(false);
    for (const field of form.findAll('input[type=password]'))
      await field.setValue('synthetic-password');
    listeners.get('panel-hidden')?.();
    await flushPromises();
    listeners.get('panel-shown')?.();
    await flushPromises();
    expect(wrapper.findComponent(ExportForm).element).toBe(form.element);
    expect(
      (form.get('input[type=checkbox]').element as HTMLInputElement).checked,
    ).toBe(false);
    for (const field of form.findAll('input[type=password]'))
      expect((field.element as HTMLInputElement).value).toBe('');
  });
  it('retains an import preview and its selection while clearing entered passwords', async () => {
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '2' });
    vi.mocked(api.import).mockResolvedValue({
      origin: 'backup',
      id: 'synthetic-preview',
      keys: [{ id: 'synthetic-key' }],
      rows: [{ sourceId: 'synthetic-source', account, issue: null }],
    });
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    await wrapper
      .findAll('button')
      .find((b) => b.text() === 'Importar')!
      .trigger('click');
    await flushPromises();
    const form = wrapper.findComponent(ImportForm);
    await form.get('input[type=password]').setValue('synthetic-password');
    await form.findAll('input[type=checkbox]')[0]!.setValue(false);
    listeners.get('panel-hidden')?.();
    await flushPromises();
    listeners.get('panel-shown')?.();
    await flushPromises();
    expect(wrapper.findComponent(ImportForm).element).toBe(form.element);
    expect(
      (form.get('input[type=password]').element as HTMLInputElement).value,
    ).toBe('');
    expect(form.text()).toContain('Importar 0 conta(s)');
    expect(api.cancel).not.toHaveBeenCalled();
  });
  it('waits for current authorization before revealing a restored panel', async () => {
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '2' });
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    listeners.get('panel-hidden')?.();
    await flushPromises();
    let resolve!: (status: VaultStatus) => void;
    vi.mocked(api.status).mockReturnValue(
      new Promise((r) => {
        resolve = r;
      }),
    );
    listeners.get('panel-shown')?.();
    await flushPromises();
    expect(wrapper.get('main').isVisible()).toBe(false);
    resolve({ phase: 'locked', epoch: '3' });
    await flushPromises();
    expect(wrapper.get('main').isVisible()).toBe(true);
    expect(wrapper.text()).not.toContain('alice');
    expect(wrapper.text()).toContain('Desbloquear');
  });
});
