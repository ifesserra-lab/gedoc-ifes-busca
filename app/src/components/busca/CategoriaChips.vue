<script setup lang="ts">
// Chips de categoria (contagem) — filtro de resultado na Busca. Componente
// de apresentação puro: recebe os grupos já calculados e emite a escolha;
// quem decide o que fazer com a seleção é a View/store (sem regra de
// negócio aqui).
import type { CategoriaGrupo } from "@/services/ipc";

const props = defineProps<{
  grupos: CategoriaGrupo[];
  /** `null`/`undefined` = "Todas". */
  selecionada?: string | null;
}>();

const emit = defineEmits<{ selecionar: [categoria: string | null] }>();

const total = () => props.grupos.reduce((soma, grupo) => soma + grupo.qtd, 0);
</script>

<template>
  <div class="categoria-chips" role="group" aria-label="Filtrar documentos por categoria">
    <button
      type="button"
      class="categoria-chips__item"
      :aria-pressed="!selecionada"
      @click="emit('selecionar', null)"
    >
      <UBadge :color="!selecionada ? 'primary' : 'neutral'" :variant="!selecionada ? 'solid' : 'subtle'">
        Todas ({{ total() }})
      </UBadge>
    </button>

    <button
      v-for="grupo in grupos"
      :key="grupo.categoria"
      type="button"
      class="categoria-chips__item"
      :aria-pressed="selecionada === grupo.categoria"
      @click="emit('selecionar', grupo.categoria)"
    >
      <UBadge
        :color="selecionada === grupo.categoria ? 'primary' : 'neutral'"
        :variant="selecionada === grupo.categoria ? 'solid' : 'subtle'"
      >
        {{ grupo.categoria }} ({{ grupo.qtd }})
      </UBadge>
    </button>
  </div>
</template>

<style scoped>
.categoria-chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--sp-2);
}

.categoria-chips__item {
  background: none;
  border: none;
  padding: var(--sp-1);
  margin: calc(var(--sp-1) * -1);
  cursor: pointer;
  border-radius: var(--radius);
}

.categoria-chips__item:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: 2px;
}
</style>
