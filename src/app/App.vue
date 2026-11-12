<script setup lang="ts">
import { computed, nextTick, onMounted, onBeforeUnmount, ref } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  api,
  errorMessage,
  type AccountInput,
  type AccountSummary,
  type Code,
  type ImportPreview,
  type VaultStatus,
  type TransferProgress,
} from '../lib/ipc';
import AccountForm from '../features/accounts/AccountForm.vue';
import ImportForm from '../features/transfer/ImportForm.vue';
import ExportForm from '../features/transfer/ExportForm.vue';
const status = ref<VaultStatus>({ phase: 'locked', epoch: '0' });
const accounts = ref<AccountSummary[]>([]);
const codes = ref<Code[]>([]);
const search = ref('');
const view = ref<'list' | 'account' | 'import' | 'export'>('list');
const editing = ref<AccountSummary | null>(null);
const preview = ref<ImportPreview | null>(null);
const busy = ref(false);
const error = ref('');
const notice = ref('');
const progress = ref('');
const visible = ref(true);
const restoring = ref(false);
const panelElement = ref<HTMLElement>();
const accountForm = ref<InstanceType<typeof AccountForm>>();
const importForm = ref<InstanceType<typeof ImportForm>>();
const exportForm = ref<InstanceType<typeof ExportForm>>();
const now = ref(Date.now() / 1000);
const resumeWindowMs = 30_000;
let hiddenAt: number | null = null;
let hiddenEpoch: string | null = null;
let resumeTimer: ReturnType<typeof setTimeout> | undefined;
let savedScroll = { top: 0, left: 0 };
let savedFocus: HTMLElement | null = null;
let visibilityGeneration = 0;
let generation = 0;
let polling = false;
let interval: ReturnType<typeof setInterval> | undefined;
const unlisten: UnlistenFn[] = [];
let disposed = false;
const filtered = computed(() =>
  accounts.value
    .filter((a) =>
      `${a.issuer} ${a.account}`
        .toLocaleLowerCase()
        .includes(search.value.toLocaleLowerCase()),
    )
    .sort((a, b) => Number(b.pinned) - Number(a.pinned)),
);
function clear() {
  if (resumeTimer) clearTimeout(resumeTimer);
  resumeTimer = undefined;
  hiddenAt = null;
  hiddenEpoch = null;
  savedScroll = { top: 0, left: 0 };
  savedFocus = null;
  generation++;
  accounts.value = [];
  codes.value = [];
  search.value = '';
  editing.value = null;
  preview.value = null;
  view.value = 'list';
  busy.value = false;
  error.value = '';
  notice.value = '';
  progress.value = '';
}
function canResume() {
  return (
    hiddenAt !== null &&
    performance.now() - hiddenAt < resumeWindowMs &&
    hiddenEpoch === status.value.epoch &&
    status.value.phase === 'unlocked'
  );
}
function hidePresentation() {
  visibilityGeneration++;
  if (!visible.value) return;
  const scroll = document.scrollingElement ?? document.documentElement;
  savedScroll = { top: scroll.scrollTop, left: scroll.scrollLeft };
  savedFocus =
    document.activeElement instanceof HTMLElement &&
    panelElement.value?.contains(document.activeElement)
      ? document.activeElement
      : null;
  accountForm.value?.clearSensitive();
  importForm.value?.clearSensitive();
  exportForm.value?.clearSensitive();
  visible.value = false;
  codes.value = [];
  if (status.value.phase !== 'unlocked') {
    clear();
    return;
  }
  hiddenAt = performance.now();
  hiddenEpoch = status.value.epoch;
  resumeTimer = setTimeout(() => {
    clear();
    status.value.phase = 'locked';
  }, resumeWindowMs);
}
async function showPresentation() {
  const request = ++visibilityGeneration;
  const resume = canResume();
  const scroll = resume ? savedScroll : { top: 0, left: 0 };
  const focus = resume ? savedFocus : null;
  const epoch = hiddenEpoch;
  if (resumeTimer) clearTimeout(resumeTimer);
  resumeTimer = undefined;
  restoring.value = true;
  visible.value = true;
  if (!resume) clear();
  try {
    await sync();
    if (request !== visibilityGeneration || disposed) return;
    const restored =
      resume &&
      status.value.phase === 'unlocked' &&
      status.value.epoch === epoch;
    hiddenAt = null;
    hiddenEpoch = null;
    savedFocus = null;
    restoring.value = false;
    await nextTick();
    if (request !== visibilityGeneration || disposed) return;
    const element = document.scrollingElement ?? document.documentElement;
    element.scrollTop = restored ? scroll.top : 0;
    element.scrollLeft = restored ? scroll.left : 0;
    if (restored && focus?.isConnected) focus.focus({ preventScroll: true });
  } catch {
    if (request !== visibilityGeneration || disposed) return;
    clear();
    status.value.phase = 'locked';
    restoring.value = false;
  }
}
async function refresh(ticket = generation) {
  const epoch = status.value.epoch;
  const [list, values] = await Promise.all([api.list(epoch), api.codes(epoch)]);
  if (
    ticket === generation &&
    (visible.value || canResume()) &&
    epoch === status.value.epoch
  ) {
    accounts.value = list;
    if (visible.value) codes.value = values;
  }
}
async function sync() {
  const ticket = generation;
  const value = await api.status();
  if (ticket !== generation || !visible.value) return;
  if (value.epoch !== status.value.epoch || value.phase === 'locked') clear();
  status.value = value;
  if (value.phase === 'unlocked') await refresh();
}
async function action(work: (current: () => boolean) => Promise<void>) {
  if (busy.value) return;
  const ticket = generation;
  const current = () => ticket === generation && (visible.value || canResume());
  busy.value = true;
  error.value = '';
  notice.value = '';
  progress.value = '';
  try {
    await work(current);
  } catch (e) {
    if (current()) {
      error.value = errorMessage(e);
      if (
        typeof e === 'object' &&
        e !== null &&
        'code' in e &&
        e.code === 'Locked'
      ) {
        clear();
        status.value = await api.status();
      }
    }
  } finally {
    if (current()) busy.value = false;
  }
}
async function unlock() {
  await action(async (current) => {
    status.value.phase = 'unlocking';
    try {
      const value = await api.unlock();
      if (current()) {
        status.value = value;
        await refresh();
      }
    } catch (e) {
      if (current()) status.value.phase = 'locked';
      throw e;
    }
  });
}
async function lock() {
  clear();
  status.value.phase = 'locked';
  await api.lock();
  status.value = await api.status();
}
function interact() {
  if (status.value.phase === 'unlocked' && visible.value)
    void api.interact(status.value.epoch).catch(() => {});
}
function edit(account: AccountSummary | null) {
  editing.value = account;
  view.value = 'account';
}
async function save(input: AccountInput) {
  await action(async (current) => {
    await api.save(status.value.epoch, editing.value?.id ?? null, input);
    if (current()) {
      view.value = 'list';
      editing.value = null;
      await refresh();
      notice.value = 'Conta salva.';
    }
  });
}
async function saveUri(uri: string) {
  await action(async (current) => {
    const value = await api.uri(status.value.epoch, uri);
    if (current()) {
      preview.value = value;
      view.value = 'import';
    }
  });
}
async function remove() {
  if (!editing.value) return;
  const id = editing.value.id;
  await action(async (current) => {
    let pendingDeletion: unknown;
    try {
      await api.delete(status.value.epoch, id);
    } catch (e) {
      if (
        typeof e !== 'object' ||
        e === null ||
        !('code' in e) ||
        e.code !== 'DeletionPending'
      )
        throw e;
      pendingDeletion = e;
    }
    if (current()) {
      view.value = 'list';
      editing.value = null;
      await refresh();
      if (pendingDeletion) throw pendingDeletion;
    }
  });
}
async function next(id: string) {
  await action(async (current) => {
    await api.next(status.value.epoch, id);
    if (current()) await refresh();
  });
}
async function copy(id: string) {
  await action(async (current) => {
    await api.copy(status.value.epoch, id);
    if (current()) notice.value = 'Código copiado. Limpeza automática em 30 s.';
  });
}
async function pin(account: AccountSummary) {
  await action(async (current) => {
    const pinned = accounts.value
      .filter((a) => a.pinned && a.id !== account.id)
      .map((a) => a.id);
    if (!account.pinned) pinned.push(account.id);
    await api.order(
      status.value.epoch,
      accounts.value.map((a) => a.id),
      pinned,
    );
    if (current()) await refresh();
  });
}
async function move(id: string, delta: number) {
  const ids = accounts.value.map((a) => a.id);
  const from = ids.indexOf(id);
  const to = from + delta;
  if (to < 0 || to >= ids.length) return;
  [ids[from], ids[to]] = [ids[to]!, ids[from]!];
  await action(async (current) => {
    await api.order(
      status.value.epoch,
      ids,
      accounts.value.filter((a) => a.pinned).map((a) => a.id),
    );
    if (current()) await refresh();
  });
}
async function beginImport() {
  await action(async (current) => {
    const value = await api.import(status.value.epoch);
    if (current()) {
      preview.value = value;
      view.value = 'import';
    }
  });
}
async function passwords(values: Record<string, string>) {
  if (!preview.value) return;
  await action(async (current) => {
    const value = await api.passwords(
      status.value.epoch,
      preview.value!.id,
      values,
    );
    for (const key in values) values[key] = '';
    if (current()) preview.value = value;
  });
}
async function commit(ids: string[], acknowledge: boolean) {
  if (!preview.value) return;
  await action(async (current) => {
    await api.commit(status.value.epoch, preview.value!.id, ids, acknowledge);
    if (current()) {
      preview.value = null;
      view.value = 'list';
      await refresh();
      notice.value = 'Contas importadas.';
    }
  });
}
async function cancelImport() {
  const epoch = status.value.epoch;
  generation++;
  preview.value = null;
  view.value = 'list';
  busy.value = false;
  try {
    await api.cancel(epoch);
    await refresh();
  } catch (e) {
    error.value = errorMessage(e);
  }
}
async function exportBackup(
  ids: string[],
  encrypted: boolean,
  password: string,
  confirmation: string,
  acknowledge: boolean,
) {
  await action(async (current) => {
    await api.export(
      status.value.epoch,
      ids,
      encrypted,
      password,
      confirmation,
      acknowledge,
    );
    if (current()) {
      view.value = 'list';
      notice.value = 'Backup salvo.';
    }
  });
}
function codeFor(id: string) {
  return codes.value.find((c) => c.id === id);
}
function remaining(id: string) {
  const code = codeFor(id);
  return code?.validUntil
    ? Math.max(0, Math.ceil(code.validUntil - now.value))
    : null;
}
function grouped(id: string) {
  const code = codeFor(id)?.code;
  return code
    ? `${code.slice(0, Math.ceil(code.length / 2))} ${code.slice(Math.ceil(code.length / 2))}`
    : '— — —';
}
async function tick() {
  now.value = Date.now() / 1000;
  if (polling || busy.value || !visible.value) return;
  polling = true;
  const ticket = generation;
  try {
    const s = await api.status();
    if (ticket !== generation) return;
    if (s.epoch !== status.value.epoch || s.phase !== status.value.phase) {
      clear();
      status.value = s;
      if (s.phase === 'unlocked') await refresh();
    } else if (s.phase === 'unlocked' && view.value === 'list') {
      const values = await api.codes(s.epoch);
      if (ticket === generation && visible.value) codes.value = values;
    }
  } catch {
    clear();
    status.value.phase = 'locked';
  } finally {
    polling = false;
  }
}
onMounted(async () => {
  const offProgress = await listen<TransferProgress>(
    'transfer-progress',
    (event) => {
      const p = event.payload;
      if (
        visible.value &&
        p.epoch === status.value.epoch &&
        p.previewId === preview.value?.id
      ) {
        progress.value = `Chaves processadas: ${p.completed} de ${p.total}`;
      }
    },
  );
  if (disposed) offProgress();
  else unlisten.push(offProgress);

  for (const [name, callback] of [
    [
      'vault-changed',
      () => {
        clear();
        void sync().catch(() => {
          clear();
          status.value.phase = 'locked';
        });
      },
    ],
    ['panel-hidden', hidePresentation],
    ['panel-shown', () => void showPresentation()],
  ] as const) {
    const off = await listen(name, callback);
    if (disposed) off();
    else unlisten.push(off);
  }
  try {
    await sync();
  } catch {
    error.value = 'Abra o aplicativo TopOTP para acessar o cofre.';
  }
  interval = setInterval(() => void tick(), 1000);
});
onBeforeUnmount(() => {
  disposed = true;
  clear();
  unlisten.forEach((off) => off());
  if (interval) clearInterval(interval);
});
</script>
<template>
  <main
    v-show="visible && !restoring"
    ref="panelElement"
    class="panel"
    @pointerdown="interact"
    @keydown="interact"
    @keydown.esc.prevent="api.hide()"
  >
    <header>
      <div class="brand">
        <img src="../assets/icon.svg" width="28" height="28" alt="" />
        <h1>TopOTP</h1>
      </div>
      <button
        v-if="status.phase === 'unlocked'"
        class="compact"
        aria-label="Bloquear cofre"
        @click="lock"
      >
        Bloquear</button
      ><span v-else class="hint">2FA no seu Mac</span>
    </header>
    <p v-if="error" class="message error" role="alert">{{ error }}</p>
    <p v-if="notice" class="message" role="status">{{ notice }}</p>
    <section
      v-if="status.phase !== 'unlocked'"
      class="locked"
      aria-labelledby="vault-heading"
    >
      <div class="vault-mark" aria-hidden="true">◇</div>
      <h2 id="vault-heading">Seu cofre, protegido</h2>
      <p>Seus códigos, a um toque de distância.</p>
      <button
        class="primary"
        :disabled="busy || status.phase === 'unlocking'"
        @click="unlock"
      >
        {{ status.phase === 'unlocking' ? 'Autenticando…' : 'Desbloquear' }}
      </button>
      <p class="hint">Touch ID ou senha do macOS</p>
      <p class="privacy">Chaves guardadas no Keychain. Sempre local.</p>
    </section>
    <template v-else-if="view === 'list'">
      <label class="search"
        ><span class="sr-only">Buscar contas</span
        ><input
          v-model="search"
          type="search"
          placeholder="Buscar contas…"
          autocomplete="off"
      /></label>
      <section v-if="!accounts.length" class="empty">
        <h2>Seu primeiro código</h2>
        <p>Adicione uma chave ou importe seu backup do Authenticator.</p>
        <button class="primary" :disabled="busy" @click="edit(null)">
          Adicionar conta
        </button>
      </section>
      <p v-else-if="!filtered.length" class="hint">Nenhuma conta encontrada.</p>
      <article
        v-for="account in filtered"
        :key="account.id"
        class="account-card"
      >
        <div class="account-top">
          <div class="account-label">
            <strong>{{ account.issuer || account.account }}</strong
            ><span>{{
              account.issuer ? account.account : account.kind.toUpperCase()
            }}</span>
          </div>
          <button
            class="icon-button"
            :aria-label="`${account.pinned ? 'Desafixar' : 'Fixar'} ${account.account}`"
            :aria-pressed="account.pinned"
            :disabled="busy"
            @click="pin(account)"
          >
            {{ account.pinned ? '★' : '☆' }}</button
          ><button
            class="icon-button"
            :aria-label="`Editar ${account.account}`"
            :disabled="busy"
            @click="edit(account)"
          >
            •••
          </button>
        </div>
        <div class="code-row">
          <span
            class="code"
            :aria-label="codeFor(account.id)?.code ?? 'Gere um código'"
            >{{ grouped(account.id) }}</span
          ><span
            v-if="account.kind === 'totp'"
            class="countdown"
            :aria-label="`${remaining(account.id) ?? 0} segundos restantes`"
            >{{ remaining(account.id) ?? 0 }} s</span
          ><span v-else class="hint">HOTP</span>
        </div>
        <div class="account-actions">
          <button
            :disabled="busy || !codeFor(account.id)"
            @click="copy(account.id)"
          >
            Copiar</button
          ><button
            v-if="account.kind === 'hotp'"
            :disabled="busy"
            @click="next(account.id)"
          >
            Gerar próximo
          </button>
          <div class="order-actions">
            <button
              class="icon-button"
              :disabled="busy || accounts[0]?.id === account.id"
              :aria-label="`Mover ${account.account} para cima`"
              @click="move(account.id, -1)"
            >
              ↑</button
            ><button
              class="icon-button"
              :disabled="busy || accounts.at(-1)?.id === account.id"
              :aria-label="`Mover ${account.account} para baixo`"
              @click="move(account.id, 1)"
            >
              ↓
            </button>
          </div>
        </div>
      </article>
      <footer>
        <button v-if="accounts.length" :disabled="busy" @click="edit(null)">
          + Conta</button
        ><button :disabled="busy" @click="beginImport">Importar</button
        ><button :disabled="busy || !accounts.length" @click="view = 'export'">
          Exportar
        </button>
      </footer>
    </template>
    <AccountForm
      v-else-if="view === 'account'"
      ref="accountForm"
      :account="editing"
      :busy="busy"
      @save="save"
      @uri="saveUri"
      @cancel="
        view = 'list';
        editing = null;
      "
      @delete="remove"
    />
    <ImportForm
      v-else-if="view === 'import' && preview"
      ref="importForm"
      :key="JSON.stringify(preview.rows.map((r) => [r.sourceId, r.issue]))"
      :preview="preview"
      :progress="progress"
      :busy="busy"
      @passwords="passwords"
      @commit="commit"
      @cancel="cancelImport"
    />
    <ExportForm
      v-else-if="view === 'export'"
      ref="exportForm"
      :accounts="accounts"
      :busy="busy"
      @export="exportBackup"
      @cancel="view = 'list'"
    />
  </main>
</template>
