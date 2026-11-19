<script setup lang="ts">
import { computed, ref, onBeforeUnmount } from 'vue';
import type { AccountSummary } from '../../lib/ipc';
const props = defineProps<{ accounts: AccountSummary[]; busy: boolean }>();
const emit = defineEmits<{
  export: [
    ids: string[],
    encrypted: boolean,
    password: string,
    confirmation: string,
    acknowledge: boolean,
  ];
  cancel: [];
}>();
const selected = ref(props.accounts.map((a) => a.id));
const encrypted = ref(true);
const password = ref('');
const confirmation = ref('');
const acknowledge = ref(false);
const minBackupPasswordChars = 16;
const passwordValid = computed(
  () =>
    Array.from(password.value.trim()).length >= minBackupPasswordChars &&
    new TextEncoder().encode(password.value).length <= 1024 &&
    password.value === confirmation.value,
);
const canSubmit = computed(
  () =>
    !props.busy &&
    selected.value.length > 0 &&
    (encrypted.value ? passwordValid.value : acknowledge.value),
);
function submit() {
  if (!canSubmit.value) return;
  const p = password.value;
  const c = confirmation.value;
  password.value = '';
  confirmation.value = '';
  emit('export', selected.value, encrypted.value, p, c, acknowledge.value);
}
function clearSensitive() {
  password.value = '';
  confirmation.value = '';
}
defineExpose({ clearSensitive });
onBeforeUnmount(clearSensitive);
</script>
<template>
  <form class="form-stack" @submit.prevent="submit">
    <div class="section-title">
      <h2>Exportar backup</h2>
      <button type="button" :disabled="busy" @click="emit('cancel')">
        Cancelar
      </button>
    </div>
    <label v-for="a in accounts" :key="a.id" class="selection"
      ><input v-model="selected" type="checkbox" :value="a.id" /><span
        >{{ a.issuer }} · {{ a.account }}</span
      ></label
    >
    <label
      >Formato<select v-model="encrypted">
        <option :value="true">Criptografado · Authenticator v3</option>
        <option :value="false">JSON em texto claro</option>
      </select></label
    >
    <template v-if="encrypted"
      ><label
        >Senha do backup<input
          v-model="password"
          type="password"
          maxlength="1024"
          required
          autocomplete="new-password" /></label
      ><label
        >Confirmar senha<input
          v-model="confirmation"
          type="password"
          maxlength="1024"
          required
          autocomplete="new-password"
      /></label>
      <p class="hint">
        Use uma frase-senha única com pelo menos {{ minBackupPasswordChars }}
        caracteres, de preferência várias palavras aleatórias. Guarde-a
        separadamente: ela será necessária para importar este backup.
      </p></template
    >
    <label v-else class="selection warning"
      ><input v-model="acknowledge" type="checkbox" required /><span
        >Entendo que este arquivo conterá as chaves secretas sem criptografia.
        Escolherei um arquivo novo e um local protegido.</span
      ></label
    >
    <p class="hint">
      O macOS pedirá Touch ID ou senha novamente antes da exportação.
    </p>
    <button class="primary" :disabled="!canSubmit">
      {{ busy ? 'Exportando…' : 'Autorizar e salvar backup' }}
    </button>
  </form>
</template>
