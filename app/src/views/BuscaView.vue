<script setup lang="ts">
// View (US1 — recomposta com Nuxt UI + tokens; US3 preserva o contrato de
// campo SIAPE + estados). Nenhuma regra de negócio aqui: tudo delega à
// store (ViewModel) — cinco estados (Constituição XII): idle, loading,
// vazio, erro, sucesso.
import EmptyState from "@/components/base/EmptyState.vue";
import ErrorState from "@/components/base/ErrorState.vue";
import LoadingState from "@/components/base/LoadingState.vue";
import CategoriaChips from "@/components/busca/CategoriaChips.vue";
import DocItem from "@/components/busca/DocItem.vue";
import { useBuscaStore } from "@/stores/busca";

const store = useBuscaStore();
</script>

<template>
  <section class="busca">
    <header class="busca__intro">
      <h1 class="busca__titulo">Consulta por SIAPE</h1>
      <p class="busca__dica">
        Informe a matrícula SIAPE (5 a 8 dígitos numéricos) para listar os documentos do GeDoc.
      </p>
    </header>

    <form class="busca__form" @submit.prevent="store.buscar()">
      <div class="busca__campo">
        <label class="busca__label" for="siape">Matrícula SIAPE</label>
        <UInput
          id="siape"
          v-model="store.siape"
          inputmode="numeric"
          autocomplete="off"
          placeholder="Ex.: 1998547"
          size="lg"
          :color="store.estado === 'erro' ? 'error' : undefined"
          :aria-invalid="store.estado === 'erro'"
          :disabled="store.estado === 'loading'"
        />
      </div>
      <UButton
        type="submit"
        size="lg"
        class="busca__botao"
        :loading="store.estado === 'loading'"
        :disabled="store.estado === 'loading'"
      >
        {{ store.estado === "loading" ? "Buscando..." : "Buscar" }}
      </UButton>
    </form>

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
        <p class="busca__total">
          <strong>{{ store.resultado.total }}</strong> documento(s) para o SIAPE
          <strong>{{ store.resultado.termo }}</strong>
        </p>

        <CategoriaChips
          :grupos="store.resultado.categorias"
          :selecionada="store.categoriaSelecionada"
          @selecionar="store.selecionarCategoria"
        />

        <div class="busca__acoes-globais">
          <span title="Geração de PDF do resumo ainda não disponível (backend em desenvolvimento).">
            <UButton icon="i-lucide-file-text" color="neutral" variant="outline" size="sm" disabled>
              PDF do resumo
            </UButton>
          </span>
          <span title="Exportação em ZIP ainda não disponível (backend em desenvolvimento).">
            <UButton icon="i-lucide-download" color="neutral" variant="outline" size="sm" disabled>
              Baixar ZIP
            </UButton>
          </span>
        </div>

        <div v-for="grupo in store.gruposFiltrados" :key="grupo.categoria" class="busca__grupo">
          <h2 class="busca__grupo-titulo">{{ grupo.categoria }} ({{ grupo.qtd }})</h2>
          <div class="busca__lista">
            <DocItem v-for="item in grupo.itens" :key="item.link" :doc="item" />
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
  gap: var(--sp-6);
  max-width: 960px;
  margin: 0 auto;
}

.busca__titulo {
  font-size: var(--text-xl);
  font-weight: 700;
  color: var(--text);
  margin: 0 0 var(--sp-1);
}

.busca__dica {
  font-size: var(--text-sm);
  color: var(--muted);
  margin: 0;
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
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--text);
}

.busca__botao {
  min-height: 40px;
}

.busca__conteudo {
  min-height: 120px;
}

.busca__resultado {
  display: flex;
  flex-direction: column;
  gap: var(--sp-4);
}

.busca__total {
  font-size: var(--text-sm);
  color: var(--muted);
  margin: 0;
}

.busca__acoes-globais {
  display: flex;
  gap: var(--sp-2);
}

.busca__grupo-titulo {
  font-size: var(--text-md);
  font-weight: 600;
  color: var(--text);
  margin: 0 0 var(--sp-2);
}

.busca__lista {
  display: flex;
  flex-direction: column;
  gap: var(--sp-3);
}
</style>
