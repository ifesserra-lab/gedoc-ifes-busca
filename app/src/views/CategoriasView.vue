<script setup lang="ts">
// View (US8) — CRUD de categorias com UTable + UModal + UForm. Nenhuma
// regra de negócio aqui: validação (R5) e persistência via IPC (ver
// stores/categorias.ts) vivem na store; a View só apresenta e coordena
// abertura/fechamento de modais (estado de UI local, não é regra de negócio).
import { onMounted, reactive, ref } from "vue";
import type { ColumnDef } from "@tanstack/vue-table";

import EmptyState from "@/components/base/EmptyState.vue";
import ErrorState from "@/components/base/ErrorState.vue";
import LoadingState from "@/components/base/LoadingState.vue";
import { type CategoriaItem, useCategoriasStore } from "@/stores/categorias";

const store = useCategoriasStore();

onMounted(() => store.carregar());

const columns: ColumnDef<CategoriaItem>[] = [
  { accessorKey: "nome", header: "Nome" },
  { accessorKey: "descricao", header: "Descrição" },
  { id: "acoes", header: "Ações" },
];

// --- Modal de criar/editar ---
const modalAberto = ref(false);
const indiceEditando = ref<number | null>(null);
const formState = reactive<CategoriaItem>({ nome: "", descricao: "" });

function abrirCriacao(): void {
  indiceEditando.value = null;
  formState.nome = "";
  formState.descricao = "";
  store.limparMensagens();
  modalAberto.value = true;
}

function abrirEdicao(indice: number): void {
  const item = store.itens[indice];
  if (!item) return;
  indiceEditando.value = indice;
  formState.nome = item.nome;
  formState.descricao = item.descricao;
  store.limparMensagens();
  modalAberto.value = true;
}

function fecharModal(): void {
  modalAberto.value = false;
}

/** Contrato do `:validate` do UForm: retorna erros por campo (R5). */
function validarFormulario(state: Partial<CategoriaItem>): Array<{ name: string; message: string }> {
  const problema = store.validarNome(state.nome ?? "", indiceEditando.value);
  return problema ? [{ name: "nome", message: problema }] : [];
}

async function aoSubmeter(evento: { data: CategoriaItem }): Promise<void> {
  const erro = await store.salvar(evento.data, indiceEditando.value);
  if (!erro) fecharModal();
}

// --- Confirmação de remoção ---
const indiceRemovendo = ref<number | null>(null);
const confirmarRemocaoAberto = ref(false);

function pedirRemocao(indice: number): void {
  indiceRemovendo.value = indice;
  confirmarRemocaoAberto.value = true;
}

function cancelarRemocao(): void {
  confirmarRemocaoAberto.value = false;
  indiceRemovendo.value = null;
}

async function confirmarRemocao(): Promise<void> {
  if (indiceRemovendo.value !== null) await store.remover(indiceRemovendo.value);
  cancelarRemocao();
}
</script>

<template>
  <section class="categorias">
    <header class="categorias__cabecalho">
      <div>
        <h1 class="categorias__titulo">Categorias</h1>
        <p class="categorias__dica">Categorias usadas para classificar os documentos encontrados na busca.</p>
      </div>
      <UButton icon="i-lucide-plus" size="lg" class="alvo-minimo" data-testid="nova-categoria" @click="abrirCriacao">
        Nova categoria
      </UButton>
    </header>

    <p v-if="store.mensagemSucesso" class="categorias__sucesso" role="status">
      {{ store.mensagemSucesso }}
    </p>

    <LoadingState v-if="store.estado === 'carregando'" label="Carregando categorias..." :linhas="3" />

    <ErrorState
      v-else-if="store.estado === 'erro' && store.erro"
      :mensagem="store.erro"
      @retry="store.carregar()"
    />

    <EmptyState
      v-else-if="store.vazio"
      titulo="Nenhuma categoria cadastrada"
      descricao="Crie a primeira categoria para começar a classificar os documentos."
    >
      <UButton icon="i-lucide-plus" class="alvo-minimo" @click="abrirCriacao">Nova categoria</UButton>
    </EmptyState>

    <div v-else class="categorias__painel">
      <UTable :data="store.itens" :columns="columns" class="categorias__tabela">
        <template #acoes-cell="{ row }">
          <div class="categorias__acoes-linha">
            <UButton
              icon="i-lucide-pencil"
              color="neutral"
              variant="ghost"
              size="sm"
              class="alvo-minimo"
              aria-label="Editar categoria"
              @click="abrirEdicao(row.index)"
            />
            <UButton
              icon="i-lucide-trash-2"
              color="error"
              variant="ghost"
              size="sm"
              class="alvo-minimo"
              aria-label="Remover categoria"
              @click="pedirRemocao(row.index)"
            />
          </div>
        </template>
      </UTable>
    </div>

    <UModal
      v-model:open="modalAberto"
      :title="indiceEditando === null ? 'Nova categoria' : 'Editar categoria'"
      description="Nome e descrição usados na classificação automática dos documentos."
    >
      <template #body>
        <UForm :state="formState" :validate="validarFormulario" class="categoria-form" @submit="aoSubmeter">
          <UFormField label="Nome" name="nome" required>
            <UInput v-model="formState.nome" placeholder="Ex.: Portaria" autofocus class="categoria-form__campo" />
          </UFormField>

          <UFormField label="Descrição" name="descricao">
            <UTextarea
              v-model="formState.descricao"
              placeholder="Descrição opcional"
              :rows="3"
              class="categoria-form__campo"
            />
          </UFormField>

          <p v-if="store.erro" class="categoria-form__erro" role="alert">{{ store.erro }}</p>

          <div class="categoria-form__acoes">
            <UButton type="button" color="neutral" variant="ghost" class="alvo-minimo" @click="fecharModal">Cancelar</UButton>
            <UButton type="submit" class="alvo-minimo">{{ indiceEditando === null ? "Criar" : "Salvar" }}</UButton>
          </div>
        </UForm>
      </template>
    </UModal>

    <UModal v-model:open="confirmarRemocaoAberto" title="Remover categoria">
      <template #body>
        <p>
          Remover a categoria "{{ indiceRemovendo !== null ? store.itens[indiceRemovendo]?.nome : "" }}"? Esta ação
          não pode ser desfeita.
        </p>
      </template>
      <template #footer>
        <UButton color="neutral" variant="ghost" class="alvo-minimo" @click="cancelarRemocao">Cancelar</UButton>
        <UButton color="error" class="alvo-minimo" @click="confirmarRemocao">Remover</UButton>
      </template>
    </UModal>
  </section>
</template>

<style scoped>
.categorias {
  display: flex;
  flex-direction: column;
  gap: var(--sp-4);
  max-width: 960px;
  margin: 0 auto;
}

.categorias__cabecalho {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--sp-4);
}

.categorias__titulo {
  font-size: var(--text-28);
  font-weight: 700;
  color: var(--ink);
  margin: 0 0 var(--sp-1);
  text-wrap: balance;
}

.categorias__dica {
  font-size: var(--text-14);
  color: var(--muted);
  margin: 0;
}

.categorias__sucesso {
  font-size: var(--text-14);
  color: var(--success);
  margin: 0;
}

.categorias__painel {
  background-color: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-sm);
  overflow: hidden;
}

.categorias__acoes-linha {
  display: flex;
  gap: var(--sp-1);
  justify-content: flex-end;
}

.categoria-form {
  display: flex;
  flex-direction: column;
  gap: var(--sp-4);
}

.categoria-form__campo {
  width: 100%;
}

.categoria-form__erro {
  font-size: var(--text-14);
  color: var(--danger);
  margin: 0;
}

.categoria-form__acoes {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-2);
}
</style>
