<script setup lang="ts">
// Estado "vazio" — reutilizado por todas as telas (Constituição XII). Recebe
// título/descrição por props; a ação de próximo passo vem via slot (quem usa
// decide o quê fazer, este componente não tem regra de negócio).
withDefaults(
  defineProps<{
    titulo: string;
    descricao?: string;
    /** Nome de ícone Iconify local (offline), ex.: "i-lucide-inbox". */
    icon?: string;
  }>(),
  { icon: "i-lucide-inbox" },
);
</script>

<template>
  <div class="empty-state" role="status">
    <UIcon :name="icon" class="empty-state__icone" aria-hidden="true" />
    <p class="empty-state__titulo">{{ titulo }}</p>
    <p v-if="descricao" class="empty-state__descricao">{{ descricao }}</p>
    <div v-if="$slots.default" class="empty-state__acao">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: var(--sp-2);
  padding: var(--sp-8) var(--sp-4);
  color: var(--muted);
}

.empty-state__icone {
  width: 32px;
  height: 32px;
  color: var(--muted);
}

.empty-state__titulo {
  font-size: var(--text-20);
  color: var(--ink);
  font-weight: 600;
  margin: 0;
  text-wrap: balance;
}

.empty-state__descricao {
  font-size: var(--text-14);
  max-width: 32ch;
  margin: 0;
}

.empty-state__acao {
  margin-top: var(--sp-2);
}
</style>
