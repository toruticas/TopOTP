<script setup lang="ts">
import { ref } from 'vue';
defineProps<{ busy: boolean }>();
const emit = defineEmits<{ remove: [confirmation: string] }>();
const confirming = ref(false);
const confirmation = ref('');
function clearSensitive() {
  confirming.value = false;
  confirmation.value = '';
}
function submit() {
  if (confirmation.value !== 'REMOVER') return;
  const value = confirmation.value;
  confirmation.value = '';
  emit('remove', value);
}
defineExpose({ clearSensitive });
</script>

<template>
  <section class="reset-vault" aria-labelledby="reset-heading">
    <div class="section-title">
      <h3 id="reset-heading">Dados do cofre</h3>
      <button
        v-if="!confirming"
        class="danger"
        :disabled="busy"
        @click="confirming = true"
      >
        Remover todos os dados
      </button>
    </div>
    <p>
      Apague permanentemente todas as contas, chaves, contadores e favoritos
      deste cofre. Para recuperá-los, será necessário importar um backup.
    </p>
    <p class="hint">
      Backups salvos fora do app e o cofre preservado de versões anteriores não
      são apagados.
    </p>
    <form v-if="confirming" class="form-stack warning" @submit.prevent="submit">
      <h3>Remover todos os dados?</h3>
      <p>Esta ação não pode ser desfeita. O macOS pedirá sua autorização.</p>
      <label>
        Digite REMOVER para confirmar
        <input
          v-model="confirmation"
          autocomplete="off"
          spellcheck="false"
          :disabled="busy"
        />
      </label>
      <div class="form-row">
        <button type="button" :disabled="busy" @click="clearSensitive">
          Cancelar
        </button>
        <button
          type="submit"
          class="danger"
          :disabled="busy || confirmation !== 'REMOVER'"
        >
          {{ busy ? 'Removendo…' : 'Confirmar remoção' }}
        </button>
      </div>
    </form>
  </section>
</template>

<style scoped>
.reset-vault {
  border-top: 1px solid #364254;
  margin-top: 28px;
  padding-top: 12px;
}
.warning h3,
.warning p {
  margin: 0;
}
</style>
