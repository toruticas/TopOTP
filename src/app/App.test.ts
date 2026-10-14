import { mount, flushPromises } from '@vue/test-utils';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import App from './App.vue';
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
});
describe('vault presentation boundaries', () => {
  it('starts locked without loading accounts or rendering credentials', async () => {
    wrapper = mount(App);
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
    wrapper = mount(App);
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
    wrapper = mount(App);
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
  it('clears accounts and pending forms when the native panel is hidden', async () => {
    vi.mocked(api.status).mockResolvedValue({ phase: 'unlocked', epoch: '2' });
    wrapper = mount(App);
    await flushPromises();
    expect(wrapper.text()).toContain('alice');
    listeners.get('panel-hidden')?.();
    await flushPromises();
    expect(wrapper.text()).not.toContain('alice');
  });
});
