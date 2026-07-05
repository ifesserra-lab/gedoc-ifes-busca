<script setup lang="ts">
// Estado de "erro" — reutilizado por todas as telas (Constituição XII).
// Mensagem útil (sem stack trace, alinhada ao `AppError`/mensagemDeErro em
// services/ipc.ts) + ação "tentar novamente" via evento (sem regra de
// negócio aqui: quem escuta `retry` decide o que refazer).
withDefaults(
  defineProps<{
    titulo?: string;
    mensagem: string;
    retryLabel?: string;
    /** Esconde o botão "tentar novamente" quando não fizer sentido. */
    permiteRetry?: boolean;
  }>(),
  { titulo: "Ocorreu um erro", retryLabel: "Tentar novamente", permiteRetry: true },
);

defineEmits<{ retry: [] }>();
</script>

<template>
  <UAlert
    role="alert"
    color="error"
    variant="soft"
    icon="i-lucide-alert-triangle"
    :title="titulo"
    :description="mensagem"
  >
    <template v-if="permiteRetry" #actions>
      <UButton color="error" variant="outline" size="sm" @click="$emit('retry')">
        {{ retryLabel }}
      </UButton>
    </template>
  </UAlert>
</template>
