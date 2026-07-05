<script setup lang="ts">
// Um documento na lista de resultados da Busca. Apresentação pura (título,
// data, resumo, ação de PDF); truncamento de título longo com reticências +
// tooltip (edge case do spec). Download de PDF ainda depende de um comando
// IPC que o backend não expõe (TODO — ver relatório da feature 002).
import type { DocView } from "@/services/ipc";

defineProps<{ doc: DocView }>();
</script>

<template>
  <UCard class="doc-item" :ui="{ body: 'doc-item__corpo' }">
    <div class="doc-item__cabecalho">
      <h3 class="doc-item__titulo" :title="doc.titulo">{{ doc.titulo }}</h3>
      <span v-if="doc.data" class="doc-item__data">{{ doc.data }}</span>
    </div>

    <p v-if="doc.resumo" class="doc-item__resumo">{{ doc.resumo }}</p>

    <div class="doc-item__acoes">
      <span title="Download de PDF ainda não disponível (backend em desenvolvimento).">
        <UButton icon="i-lucide-file-text" color="neutral" variant="outline" size="sm" disabled>
          PDF
        </UButton>
      </span>
    </div>
  </UCard>
</template>

<style scoped>
.doc-item__cabecalho {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: var(--sp-3);
}

.doc-item__titulo {
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text);
  margin: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 48ch;
}

.doc-item__data {
  font-size: var(--text-xs);
  color: var(--muted);
  white-space: nowrap;
}

.doc-item__resumo {
  margin: var(--sp-2) 0 0;
  font-size: var(--text-sm);
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
