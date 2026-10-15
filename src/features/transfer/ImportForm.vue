<script setup lang="ts">
import { reactive, ref, onBeforeUnmount } from 'vue';
import type { ImportPreview } from '../../lib/ipc';
const props = defineProps<{
  preview: ImportPreview;
  busy: boolean;
  progress: string;
}>();
const emit = defineEmits<{
  passwords: [values: Record<string, string>];
  commit: [ids: string[], acknowledge: boolean];
  cancel: [];
}>();
const passwords = reactive<Record<string, string>>({});
const selected = ref(
  props.preview.rows
    .filter((r) => r.account && !r.issue)
    .map((r) => r.account!.id)
    .slice(0, props.preview.origin === 'qr' ? 1 : undefined),
);
const acknowledge = ref(false);
const issues: Record<string, string> = {
  PasswordRequired: 'Precisa de senha',
  MissingKey: 'Chave de backup ausente',
  DuplicateKept: 'Duplicata: existente mantida',
  UnsupportedFormat: 'Formato não suportado',
  LegacyEncryptedFormat: 'Criptografia antiga: reexporte pela extensão',
  UnsupportedOrInvalidRecord: 'Tipo ou registro não suportado',
  InvalidParameters: 'Parâmetros inválidos',
  InvalidEncryptedRecord: 'Registro criptografado inválido',
};
function sendPasswords() {
  const values = { ...passwords };
  for (const key in passwords) passwords[key] = '';
  emit('passwords', values);
}
function clearSensitive() {
  for (const key in passwords) passwords[key] = '';
}
defineExpose({ clearSensitive });
onBeforeUnmount(clearSensitive);
</script>
<template>
  <section class="form-stack">
    <div class="section-title">
      <h2>
        {{ preview.origin !== 'backup' ? 'Revisar conta' : 'Importar backup' }}
      </h2>
      <button @click="emit('cancel')">Cancelar</button>
    </div>
    <form
      v-if="preview.keys.length"
      class="form-stack"
      @submit.prevent="sendPasswords"
    >
      <p>Informe a senha de cada chave do backup.</p>
      <label v-for="(key, index) in preview.keys" :key="key.id"
        >Senha da chave {{ index + 1
        }}<input
          v-model="passwords[key.id]"
          type="password"
          maxlength="1024"
          required
          autocomplete="off" /></label
      ><button class="primary" :disabled="busy">
        {{ busy ? 'Descriptografando…' : 'Abrir backup' }}
      </button>
    </form>
    <p v-if="busy && progress" role="status">{{ progress }}</p>
    <p>{{ preview.rows.length }} registro(s). Revise antes de importar.</p>
    <label
      v-for="(row, index) in preview.rows"
      :key="row.sourceId"
      class="selection"
      ><input
        v-if="row.account && preview.origin === 'qr'"
        type="radio"
        :name="preview.id"
        :value="row.account.id"
        :checked="selected.includes(row.account.id)"
        :disabled="!!row.issue || busy"
        @change="selected = [row.account.id]"
      /><input
        v-else-if="row.account"
        v-model="selected"
        type="checkbox"
        :value="row.account.id"
        :disabled="!!row.issue || busy"
      /><span
        >{{
          row.account
            ? `${row.account.issuer} · ${row.account.account}`
            : `Registro ${index + 1}`
        }}<small v-if="row.issue">{{
          issues[row.issue] ?? 'Registro não importável'
        }}</small
        ><small v-else
          >{{ row.account?.kind.toUpperCase() }} ·
          {{ row.account?.algorithm }}</small
        ><small v-if="preview.origin === 'qr' && row.account && !row.issue"
          >{{ row.account.digits }} dígitos ·
          {{
            row.account.kind === 'totp'
              ? `${row.account.period} s`
              : `Próximo contador: ${row.account.nextCounter}`
          }}</small
        ></span
      ></label
    >
    <label
      v-if="preview.origin !== 'qr' && selected.length !== preview.rows.length"
      class="selection"
      ><input v-model="acknowledge" type="checkbox" /><span
        >Importar apenas a seleção e ignorar os demais registros.</span
      ></label
    >
    <button
      class="primary"
      :disabled="
        busy ||
        !selected.length ||
        (preview.origin !== 'qr' &&
          selected.length !== preview.rows.length &&
          !acknowledge)
      "
      @click="emit('commit', selected, preview.origin === 'qr' || acknowledge)"
    >
      {{
        busy
          ? 'Salvando…'
          : preview.origin === 'qr'
            ? 'Salvar conta'
            : `Importar ${selected.length} conta(s)`
      }}
    </button>
  </section>
</template>
