import { mount, flushPromises } from '@vue/test-utils';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import App from './App.vue';
import AccountForm from '../features/accounts/AccountForm.vue';
import ImportForm from '../features/transfer/ImportForm.vue';
import ExportForm from '../features/transfer/ExportForm.vue';
import { api } from '../lib/ipc';
import type { AccountSummary, VaultStatus } from '../lib/ipc';
const listeners = vi.hoisted(
  () => new Map<string, (event?: { payload?: unknown }) => void>(),
);
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
  vi.mocked(api.managementReady).mockResolvedValue(null);
  vi.mocked(api.pendingPreview).mockResolvedValue(null);
});
afterEach(() => {
  wrapper?.unmount();
  wrapper = undefined;
  vi.useRealTimers();
});
describe('vault presentation boundaries', () => {
  it('keeps settings on blur, Escape, repeated show and session reauthentication', async () => {
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '2' });
    vi.mocked(api.managementReady).mockResolvedValueOnce({
      route: 'general',
      epoch: '2',
    });
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    await wrapper.get('main').trigger('blur');
    window.dispatchEvent(new Event('blur'));
    await wrapper.get('main').trigger('keydown', { key: 'Escape' });
    listeners.get('panel-shown')?.();
    await flushPromises();
    expect(wrapper.get('main').isVisible()).toBe(true);
    expect(wrapper.get('h2').text()).toBe('Configurações');
    expect(api.hide).not.toHaveBeenCalled();
    expect(api.unlock).not.toHaveBeenCalled();
    vi.mocked(api.status).mockResolvedValue({ phase: 'locked', epoch: '3' });
    listeners.get('vault-changed')?.();
    await flushPromises();
    expect(wrapper.get('main').isVisible()).toBe(true);
    expect(wrapper.text()).toContain('Desbloquear');
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '4' });
    vi.mocked(api.unlock).mockResolvedValue({ phase: 'unlocked', epoch: '4' });
    await wrapper.get('button.primary').trigger('click');
    await flushPromises();
    expect(wrapper.get('h2').text()).toBe('Configurações');
  });
  it('removes through settings and reflects revocation before the command finishes', async () => {
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '2' });
    vi.mocked(api.managementReady).mockResolvedValueOnce({
      route: 'general',
      epoch: '2',
    });
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    vi.mocked(api.removeAll).mockImplementation(async () => {
      vi.mocked(api.status).mockResolvedValue({ phase: 'locked', epoch: '3' });
      listeners.get('vault-changed')?.();
      await flushPromises();
      return { phase: 'locked', epoch: '3' };
    });
    await wrapper.get('.reset-vault button').trigger('click');
    await wrapper.get('.reset-vault input').setValue('REMOVER');
    await wrapper.get('.reset-vault form').trigger('submit');
    await flushPromises();
    expect(api.removeAll).toHaveBeenCalledExactlyOnceWith('2', 'REMOVER');
    expect(wrapper.text()).toContain(
      'Todos os dados deste cofre foram removidos.',
    );
    expect(wrapper.text()).not.toContain(account.account);
    expect(wrapper.get('main').isVisible()).toBe(true);
  });
  it('keeps cancellation harmless and reports a partial removal failure after revocation', async () => {
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '2' });
    vi.mocked(api.managementReady).mockResolvedValueOnce({
      route: 'general',
      epoch: '2',
    });
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    const submit = async () => {
      if (!wrapper!.find('.reset-vault input').exists())
        await wrapper!.get('.reset-vault button').trigger('click');
      await wrapper!.get('.reset-vault input').setValue('REMOVER');
      await wrapper!.get('.reset-vault form').trigger('submit');
      await flushPromises();
    };
    vi.mocked(api.removeAll).mockRejectedValueOnce({ code: 'Cancelled' });
    await submit();
    expect(wrapper.get('[role=alert]').text()).toBe('Operação cancelada.');
    expect(wrapper.get('h2').text()).toBe('Configurações');
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Contas')!
      .trigger('click');
    expect(wrapper.find('.account-card').text()).toContain(account.account);
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Geral')!
      .trigger('click');
    vi.mocked(api.removeAll).mockImplementationOnce(async () => {
      vi.mocked(api.status).mockResolvedValue({ phase: 'locked', epoch: '3' });
      listeners.get('vault-changed')?.();
      await flushPromises();
      throw { code: 'StorageUnavailable' };
    });
    await submit();
    expect(wrapper.get('[role=alert]').text()).toContain(
      'remoção não foi concluída',
    );
    expect(wrapper.text()).not.toContain(
      'Todos os dados deste cofre foram removidos.',
    );
  });
  it('offers backup migration instead of an empty vault and resumes it after cancellation', async () => {
    const migration: VaultStatus = {
      phase: 'unlocked',
      epoch: '2',
      legacyMigrationRequired: true,
    };
    vi.mocked(api.status).mockResolvedValue(migration);
    vi.mocked(api.list).mockResolvedValue([]);
    vi.mocked(api.managementReady).mockResolvedValueOnce({
      route: 'migration',
      epoch: '2',
    });
    vi.mocked(api.import).mockRejectedValueOnce({ code: 'Cancelled' });
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    expect(wrapper.get('h2').text()).toBe('Traga suas contas para esta versão');
    expect(wrapper.text()).toContain(
      'não gere novos códigos na versão anterior',
    );
    expect(wrapper.text()).not.toContain('Seu primeiro código');
    expect(wrapper.find('nav').exists()).toBe(false);
    await wrapper.get('button.primary').trigger('click');
    await flushPromises();
    expect(wrapper.get('h2').text()).toBe('Traga suas contas para esta versão');
    vi.mocked(api.import).mockResolvedValue({
      origin: 'backup',
      id: 'migration-preview',
      keys: [],
      rows: [{ sourceId: 'source', account, issue: null }],
    });
    await wrapper.get('button.primary').trigger('click');
    await flushPromises();
    expect(wrapper.findComponent(ImportForm).exists()).toBe(true);
    wrapper.findComponent(ImportForm).vm.$emit('cancel');
    await flushPromises();
    expect(wrapper.get('h2').text()).toBe('Traga suas contas para esta versão');
    expect(api.unlock).not.toHaveBeenCalled();
  });
  it('applies the latest native route after an earlier route finishes loading', async () => {
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '2' });
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    let finish!: (
      value: Awaited<ReturnType<typeof api.pendingPreview>>,
    ) => void;
    vi.mocked(api.pendingPreview).mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    vi.mocked(api.managementReady).mockResolvedValueOnce({
      route: 'qr-review',
      epoch: '2',
    });
    listeners.get('management-route')?.();
    await flushPromises();
    vi.mocked(api.managementReady).mockResolvedValueOnce({
      route: 'general',
      epoch: '2',
    });
    listeners.get('management-route')?.();
    await flushPromises();
    finish({
      origin: 'qr',
      id: 'earlier',
      keys: [],
      rows: [{ sourceId: 'source', account, issue: null }],
    });
    await flushPromises();
    expect(wrapper.get('h2').text()).toBe('Configurações');
  });
  it('keeps an early native route until initial authorization has loaded', async () => {
    let finish!: (value: VaultStatus) => void;
    vi.mocked(api.status).mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    vi.mocked(api.managementReady).mockResolvedValueOnce({
      route: 'general',
      epoch: '2',
    });
    listeners.get('management-route')?.();
    await flushPromises();
    expect(wrapper.text()).not.toContain('Configurações');
    finish({ phase: 'unlocked', epoch: '2' });
    await flushPromises();
    expect(wrapper.get('h2').text()).toBe('Configurações');
    expect(wrapper.text()).toContain('TopOTP 0.0.1-beta.1');
    listeners.get('vault-changed')?.({
      payload: { phase: 'unlocked', epoch: '2' },
    });
    await flushPromises();
    expect(wrapper.get('h2').text()).toBe('Configurações');
    expect(api.unlock).not.toHaveBeenCalled();
  });
  it('routes to a pending QR review using metadata only', async () => {
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '2' });
    vi.mocked(api.managementReady).mockResolvedValueOnce({
      route: 'qr-review',
      epoch: '2',
    });
    vi.mocked(api.pendingPreview).mockResolvedValue({
      origin: 'qr',
      id: 'synthetic-preview',
      keys: [],
      rows: [{ sourceId: 'source', account, issue: null }],
    });
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    expect(api.pendingPreview).toHaveBeenCalledWith('2');
    expect(wrapper.findComponent(ImportForm).exists()).toBe(true);
    expect(wrapper.find('input[type=password]').exists()).toBe(false);
    expect(wrapper.text()).toContain('Revisar conta');
    expect(wrapper.find('img[src="x"]').exists()).toBe(false);
    await wrapper.get('button.primary').trigger('click');
    await flushPromises();
    expect(api.commit).toHaveBeenCalledWith(
      '2',
      'synthetic-preview',
      ['synthetic'],
      true,
    );
  });
  it('discards a QR response after revocation and clears drafts when closed', async () => {
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '2' });
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();
    await wrapper
      .findAll('button')
      .find((button) => button.text() === 'Nova entrada')!
      .trigger('click');
    await wrapper.get('input[type=password]').setValue('SYNTHETICSEED');
    listeners.get('management-closed')?.();
    await flushPromises();
    expect(wrapper.findComponent(AccountForm).exists()).toBe(false);
    listeners.get('panel-shown')?.();
    await flushPromises();
    let finish!: (
      value: Awaited<ReturnType<typeof api.pendingPreview>>,
    ) => void;
    vi.mocked(api.pendingPreview).mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    vi.mocked(api.managementReady).mockResolvedValueOnce({
      route: 'qr-review',
      epoch: '2',
    });
    listeners.get('management-route')?.();
    await flushPromises();
    vi.mocked(api.status).mockResolvedValue({ phase: 'locked', epoch: '3' });
    listeners.get('vault-changed')?.({
      payload: { phase: 'locked', epoch: '3' },
    });
    finish({
      origin: 'qr',
      id: 'late',
      keys: [],
      rows: [{ sourceId: 'source', account, issue: null }],
    });
    await flushPromises();
    expect(wrapper.findComponent(ImportForm).exists()).toBe(false);
    expect(wrapper.text()).not.toContain(account.account);
    expect(wrapper.text()).toContain('Desbloquear');
  });
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
  it('uses native authorization on restore instead of a web 30 second deadline', async () => {
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
    expect(wrapper.findComponent(AccountForm).exists()).toBe(true);
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
