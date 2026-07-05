<script setup lang="ts">
// Estado de "carregando" — reutilizado por todas as telas (Constituição XII:
// cinco estados sempre). Componente fino: só apresenta, sem regra de negócio.
withDefaults(
  defineProps<{
    /** Anunciado a leitores de tela (aria-label do status). */
    label?: string;
    /** Quantidade de linhas de skeleton a desenhar (aproxima o layout real). */
    linhas?: number;
  }>(),
  { label: "Carregando...", linhas: 3 },
);
</script>

<template>
  <div class="loading-state" role="status" aria-live="polite" :aria-label="label">
    <USkeleton v-for="n in linhas" :key="n" class="loading-state__linha" />
    <span class="sr-only">{{ label }}</span>
  </div>
</template>

<style scoped>
.loading-state {
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
}

.loading-state__linha {
  height: var(--sp-8);
  border-radius: var(--radius);
}

.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
</style>
