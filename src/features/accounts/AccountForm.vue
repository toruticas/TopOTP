<script setup lang="ts">
import { reactive, ref, onBeforeUnmount, onMounted } from 'vue';
import type { AccountInput, AccountSummary } from '../../lib/ipc';
const props = defineProps<{ account: AccountSummary | null; busy: boolean }>();
const emit = defineEmits<{
  save: [input: AccountInput];
  uri: [uri: string];
  cancel: [];
  delete: [];
}>();
const mode = ref<'manual' | 'uri'>('manual');
const uri = ref('');
const confirmingDelete = ref(false);
const formElement = ref<HTMLFormElement>();
onMounted(() =>
  formElement.value
    ?.querySelector<HTMLInputElement>('input,select,textarea')
    ?.focus(),
);
const form = reactive<AccountInput>({
  issuer: props.account?.issuer ?? '',
  account: props.account?.account ?? '',
  kind: props.account?.kind ?? 'totp',
  algorithm: props.account?.algorithm ?? 'SHA1',
  digits: props.account?.digits ?? 6,
  period: props.account?.period ?? 30,
  nextCounter: props.account?.nextCounter ?? '0',
  secret: '',
});
function submit() {
  if (mode.value === 'uri') {
    const value = uri.value;
    uri.value = '';
    emit('uri', value);
  } else {
    const value = { ...form, secret: form.secret || null };
    form.secret = '';
    emit('save', value);
  }
}
onBeforeUnmount(() => {
  form.secret = '';
  uri.value = '';
});
</script>
<template>
  <form ref="formElement" class="form-stack" @submit.prevent="submit">
    <div class="section-title">
      <h2>{{ account ? 'Editar conta' : 'Adicionar conta' }}</h2>
      <button type="button" :disabled="busy" @click="emit('cancel')">
        Voltar
      </button>
    </div>
    <label v-if="!account"
      >Como adicionar<select v-model="mode">
        <option value="manual">Chave manual</option>
        <option value="uri">URI otpauth://</option>
      </select></label
    >
    <template v-if="mode === 'manual'">
      <label
        >Emissor<input
          v-model="form.issuer"
          maxlength="1024"
          autocomplete="off"
      /></label>
      <label
        >Conta<input
          v-model="form.account"
          maxlength="1024"
          autocomplete="off"
          required
      /></label>
      <label
        >{{ account ? 'Nova chave (opcional)' : 'Chave Base32'
        }}<input
          v-model="form.secret"
          type="password"
          maxlength="1024"
          autocomplete="off"
          :required="!account"
          spellcheck="false"
      /></label>
      <div class="form-row">
        <label
          >Tipo<select v-model="form.kind">
            <option value="totp">TOTP · tempo</option>
            <option value="hotp">HOTP · contador</option>
          </select></label
        ><label
          >Algoritmo<select v-model="form.algorithm">
            <option>SHA1</option>
            <option>SHA256</option>
            <option>SHA512</option>
          </select></label
        >
      </div>
      <div class="form-row">
        <label
          >Dígitos<input
            v-model.number="form.digits"
            type="number"
            min="6"
            max="8"
            required /></label
        ><label v-if="form.kind === 'totp'"
          >Período (s)<input
            v-model.number="form.period"
            type="number"
            min="1"
            max="86400"
            required /></label
        ><label v-else
          >Próximo contador<input
            v-model="form.nextCounter"
            inputmode="numeric"
            pattern="[0-9]+"
            required
        /></label>
      </div>
      <p v-if="account?.kind === 'hotp'" class="hint">
        O contador pode avançar; valores consumidos não podem ser reutilizados.
      </p>
    </template>
    <label v-else
      >URI de configuração<textarea
        v-model="uri"
        maxlength="8192"
        required
        autocomplete="off"
        spellcheck="false"
        placeholder="otpauth://totp/…"
      />
    </label>
    <button class="primary" type="submit" :disabled="busy">
      {{
        busy
          ? 'Processando…'
          : mode === 'uri'
            ? 'Revisar conta'
            : 'Salvar conta'
      }}
    </button>
    <template v-if="account"
      ><button
        v-if="!confirmingDelete"
        type="button"
        class="danger"
        :disabled="busy"
        @click="confirmingDelete = true"
      >
        Excluir conta…
      </button>
      <div v-else class="warning">
        <p>Excluir esta conta do cofre? Guarde um backup antes.</p>
        <button
          type="button"
          class="danger"
          :disabled="busy"
          @click="emit('delete')"
        >
          Confirmar exclusão</button
        ><button type="button" @click="confirmingDelete = false">
          Manter conta
        </button>
      </div></template
    >
  </form>
</template>
