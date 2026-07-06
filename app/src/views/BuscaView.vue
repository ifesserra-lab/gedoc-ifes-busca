<script setup lang="ts">
// View (US1 — redesign aprovado: minimalista institucional, acento
// verde-pinho, dado em mono tabular; ver
// specs/002-ui-nuxt-minimalista/design-tokens.md). Nenhuma regra de negócio
// aqui: tudo delega à store (ViewModel) — cinco estados (Constituição XII):
// idle, loading, vazio, erro, sucesso.
import EmptyState from "@/components/base/EmptyState.vue";
import ErrorState from "@/components/base/ErrorState.vue";
import LoadingState from "@/components/base/LoadingState.vue";
import CategoriaChips from "@/components/busca/CategoriaChips.vue";
import DocItem from "@/components/busca/DocItem.vue";
import RelatorioAcoes from "@/components/busca/RelatorioAcoes.vue";
import { useBuscaStore } from "@/stores/busca";

const store = useBuscaStore();
</script>

<template>
  <section class="busca">
    <header class="busca__intro">
      <p class="busca__eyebrow label-caps">Consulta GeDoc</p>
      <h1 class="busca__titulo text-balance">Buscar por matrícula SIAPE</h1>

      <form class="busca__form" @submit.prevent="store.buscar()">
        <div class="busca__campo">
          <label class="busca__label" for="siape">Matrícula SIAPE</label>
          <UInput
            id="siape"
            v-model="store.siape"
            class="busca__input mono"
            inputmode="numeric"
            autocomplete="off"
            placeholder="Ex.: 1998547"
            size="xl"
            :color="store.estado === 'erro' ? 'error' : undefined"
            :aria-invalid="store.estado === 'erro'"
            :disabled="store.estado === 'loading'"
          />
        </div>
        <UButton
          type="submit"
          size="xl"
          class="busca__botao alvo-primario"
          :loading="store.estado === 'loading'"
          :disabled="store.estado === 'loading'"
        >
          {{ store.estado === "loading" ? "Buscando..." : "Buscar" }}
        </UButton>
      </form>
      <p class="busca__hint">Informe de 5 a 8 dígitos numéricos.</p>

      <USwitch
        id="usar-ia"
        v-model="store.usarIa"
        class="busca__toggle-ia alvo-minimo"
        :disabled="store.estado === 'loading'"
        label="Classificar e resumir com IA"
        description="Mais lento: envia cada documento a um serviço de IA para categorizar e gerar um resumo curto."
      />
    </header>

    <div class="busca__conteudo">
      <LoadingState v-if="store.estado === 'loading'" label="Buscando documentos..." :linhas="4" />

      <ErrorState
        v-else-if="store.estado === 'erro'"
        :mensagem="store.erro ?? 'Erro inesperado.'"
        @retry="store.buscar()"
      />

      <EmptyState
        v-else-if="store.vazio"
        titulo="Nenhum documento para este SIAPE"
        descricao="Verifique o número informado ou tente outra matrícula."
      />

      <div v-else-if="store.estado === 'resultado' && store.resultado" class="busca__resultado">
        <div class="busca__resumo">
          <p class="busca__resumo-texto">
            <span class="mono busca__resumo-numero">{{ store.resultado.total }}</span>
            documento(s) · SIAPE <span class="mono">{{ store.resultado.termo }}</span>
          </p>

          <RelatorioAcoes class="busca__resumo-acoes" :resultado="store.resultado" />
        </div>

        <CategoriaChips
          :grupos="store.resultado.categorias"
          :selecionada="store.categoriaSelecionada"
          @selecionar="store.selecionarCategoria"
        />

        <div class="busca__painel">
          <div v-for="(grupo, indice) in store.gruposFiltrados" :key="grupo.categoria" class="busca__grupo">
            <h2 class="busca__grupo-titulo label-caps" :class="{ 'busca__grupo-titulo--com-divisor': indice > 0 }">
              {{ grupo.categoria }} · <span class="mono">{{ grupo.qtd }}</span>
            </h2>
            <div class="busca__lista">
              <DocItem
                v-for="item in grupo.itens"
                :key="item.link"
                :doc="item"
                :categoria="grupo.categoria"
                :siape="store.resultado.termo"
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.busca {
  display: flex;
  flex-direction: column;
  gap: var(--sp-8);
  max-width: 960px;
  margin: 0 auto;
}

.busca__eyebrow {
  margin: 0 0 var(--sp-2);
}

.busca__titulo {
  font-size: var(--text-28);
  font-weight: 700;
  color: var(--ink);
  margin: 0 0 var(--sp-5);
}

.busca__form {
  display: flex;
  align-items: flex-end;
  gap: var(--sp-3);
}

.busca__campo {
  display: flex;
  flex-direction: column;
  gap: var(--sp-1);
  flex: 1;
  max-width: 320px;
}

.busca__label {
  font-size: var(--text-14);
  font-weight: 500;
  color: var(--ink);
}

.busca__botao {
  min-height: 48px;
}

.busca__hint {
  font-size: var(--text-13);
  color: var(--muted);
  margin: var(--sp-2) 0 0;
}

.busca__toggle-ia {
  align-items: flex-start;
  margin-top: var(--sp-4);
}

.busca__conteudo {
  min-height: 120px;
}

.busca__resultado {
  display: flex;
  flex-direction: column;
  gap: var(--sp-5);
}

.busca__resumo {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-4);
  flex-wrap: wrap;
}

.busca__resumo-texto {
  font-size: var(--text-14);
  color: var(--muted);
  margin: 0;
  display: flex;
  align-items: baseline;
  gap: var(--sp-2);
}

.busca__resumo-numero {
  font-size: var(--text-34);
  font-weight: 700;
  color: var(--ink);
}

.busca__painel {
  background-color: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-sm);
  padding: var(--sp-5) var(--sp-6);
  display: flex;
  flex-direction: column;
  gap: var(--sp-5);
}

.busca__grupo-titulo {
  margin: 0 0 var(--sp-3);
}

.busca__grupo-titulo--com-divisor {
  padding-top: var(--sp-5);
  border-top: 1px solid var(--border);
}

.busca__lista {
  display: flex;
  flex-direction: column;
}
</style>
