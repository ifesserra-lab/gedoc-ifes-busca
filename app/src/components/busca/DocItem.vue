<script setup lang="ts">
// Um documento na lista de resultados da Busca (redesign aprovado: dot +
// título + meta [data mono + pill de categoria] + resumo + ação PDF — ver
// specs/002-ui-nuxt-minimalista/design-tokens.md). Apresentação pura;
// título longo trunca com reticências + tooltip nativo (`title`, edge
// case do spec). Download de PDF ainda depende de um comando IPC que o
// backend não expõe (TODO — ver relatório da feature 002).
import type { DocView } from "@/services/ipc";

defineProps<{
  doc: DocView;
  /** Categoria do grupo ao qual este documento pertence (pill de contexto). */
  categoria: string;
}>();
</script>

<template>
  <article class="doc-item">
    <span class="doc-item__dot" aria-hidden="true"></span>

    <div class="doc-item__corpo">
      <h3 class="doc-item__titulo" :title="doc.titulo">{{ doc.titulo }}</h3>

      <div class="doc-item__meta">
        <span v-if="doc.data" class="doc-item__data mono">{{ doc.data }}</span>
        <span class="doc-item__pill label-caps">{{ categoria }}</span>
      </div>

      <p v-if="doc.resumo" class="doc-item__resumo text-prosa">{{ doc.resumo }}</p>

      <div class="doc-item__acoes">
        <UTooltip text="Download de PDF ainda não disponível (backend em desenvolvimento).">
          <UButton
            icon="i-lucide-file-text"
            color="neutral"
            variant="outline"
            size="sm"
            class="alvo-minimo"
            aria-disabled="true"
          >
            PDF
          </UButton>
        </UTooltip>
      </div>
    </div>
  </article>
</template>

<style scoped>
.doc-item {
  display: flex;
  gap: var(--sp-3);
  padding: var(--sp-4) 0;
}

.doc-item:not(:last-child) {
  border-bottom: 1px solid var(--border);
}

.doc-item__dot {
  flex-shrink: 0;
  width: 8px;
  height: 8px;
  margin-top: 8px;
  border-radius: 50%;
  background-color: var(--faint);
}

.doc-item__corpo {
  flex: 1;
  min-width: 0;
}

.doc-item__titulo {
  font-size: var(--text-16);
  font-weight: 600;
  color: var(--ink);
  margin: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.doc-item__meta {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
  margin-top: var(--sp-1);
}

.doc-item__data {
  font-size: var(--text-13);
  color: var(--muted);
  white-space: nowrap;
}

.doc-item__pill {
  margin: 0;
  background-color: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 999px;
  padding: 2px var(--sp-2);
}

.doc-item__resumo {
  margin: var(--sp-2) 0 0;
  font-size: var(--text-14);
  color: var(--muted);
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.doc-item__acoes {
  margin-top: var(--sp-3);
  display: flex;
  justify-content: flex-end;
}
</style>
