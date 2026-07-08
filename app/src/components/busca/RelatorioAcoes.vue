<script setup lang="ts">
// US7 — ações do cabeçalho-resumo da Busca: "Baixar relatório" (HTML
// consolidado com os resumos agrupados por categoria — ver decisão de PDF em
// `services::relatorio` no backend: HTML self-contained aberto no app
// padrão, sem depender de Chrome; "Imprimir → Salvar como PDF" fica a cargo
// do usuário) e "Baixar ZIP" (todos os PDFs já baixados, US4, para o SIAPE
// atual). Extraído da BuscaView num componente próprio para as mudanças
// desta US ficarem localizadas aqui, sem tocar o resto da tela. Nenhuma
// regra de negócio: as duas chamadas de IPC já validam/geram tudo no
// backend; este componente só administra o estado visual idle/processando/
// erro de cada ação, independentemente uma da outra.
import { computed, ref } from "vue";
import { useToast } from "@nuxt/ui/composables";

import { baixarZip, gerarRelatorio, mensagemDeErro, type ResultadoView } from "@/services/ipc";
import { useBuscaStore } from "@/stores/busca";

const props = defineProps<{
  resultado: ResultadoView | null;
}>();

type Estado = "idle" | "processando" | "erro";

const estadoRelatorio = ref<Estado>("idle");
const erroRelatorio = ref<string | null>(null);
const estadoZip = ref<Estado>("idle");
const erroZip = ref<string | null>(null);

const store = useBuscaStore();
const toast = useToast();

/** US #22 — progresso e ação de baixar todos os PDFs (delegados à store). */
const progresso = computed(() => store.downloadProgresso);
const baixandoTodos = computed(() => store.baixandoTodos);

async function baixarTodosPdfs(): Promise<void> {
  if (!props.resultado || !temItens.value || baixandoTodos.value) return;
  const docs = props.resultado.categorias.flatMap((grupo) => grupo.itens);
  const { ok, falhas } = await store.baixarTodos(docs, props.resultado.termo);
  if (falhas === 0) {
    toast.add({ title: `${ok} PDF(s) baixado(s).`, color: "success", icon: "i-lucide-check" });
  } else {
    toast.add({
      title: `${ok} baixado(s), ${falhas} com falha.`,
      color: "warning",
      icon: "i-lucide-triangle-alert",
    });
  }
}

/** Só habilita as ações quando há ao menos 1 documento para exportar — um
 * `resultado.total > 0` de portal não basta: R2 pode descartar tudo e deixar
 * `categorias` sem nenhum item. */
const temItens = computed(
  () => props.resultado?.categorias.some((grupo) => grupo.itens.length > 0) ?? false,
);

/** US7 — o relatório consolida os RESUMOS da IA. Se a busca atual não foi no
 * modo IA, clicar EXECUTA a IA (re-busca em modo llm) antes de gerar. */
const dicaRelatorio = computed(() =>
  store.resultadoComIa
    ? "Gera um relatório consolidado (HTML) com os resumos, agrupado por categoria."
    : "Executa a IA (resume os documentos) e gera o relatório consolidado — pode levar um tempo.",
);

async function baixarRelatorio(): Promise<void> {
  if (!props.resultado || !temItens.value || estadoRelatorio.value === "processando") return;

  estadoRelatorio.value = "processando";
  erroRelatorio.value = null;
  try {
    // O relatório consolida os resumos da IA. Se a busca atual não foi no modo
    // IA, roda a IA agora (re-busca em modo llm) antes de gerar — assim o
    // relatório sai com os resumos.
    if (!store.resultadoComIa) {
      const antes = store.usarIa;
      store.usarIa = true;
      await store.buscar();
      store.usarIa = antes;
      if (store.estado !== "resultado" || !store.resultado) {
        estadoRelatorio.value = "erro";
        erroRelatorio.value = store.erro ?? "Falha ao executar a IA para o relatório.";
        return;
      }
    }
    await gerarRelatorio(store.resultado ?? props.resultado);
    estadoRelatorio.value = "idle";
  } catch (motivo) {
    estadoRelatorio.value = "erro";
    erroRelatorio.value = mensagemDeErro(motivo);
  }
}

async function baixarDocumentosZip(): Promise<void> {
  if (!props.resultado || !temItens.value || estadoZip.value === "processando") return;

  estadoZip.value = "processando";
  erroZip.value = null;
  try {
    await baixarZip(props.resultado.termo);
    estadoZip.value = "idle";
  } catch (motivo) {
    estadoZip.value = "erro";
    erroZip.value = mensagemDeErro(motivo);
  }
}
</script>

<template>
  <div class="relatorio-acoes">
    <div class="relatorio-acoes__botoes">
      <UTooltip :text="erroRelatorio ?? dicaRelatorio">
        <UButton
          icon="i-lucide-file-text"
          :color="estadoRelatorio === 'erro' ? 'error' : 'neutral'"
          variant="ghost"
          size="sm"
          class="alvo-minimo"
          :loading="estadoRelatorio === 'processando'"
          :disabled="!temItens || estadoRelatorio === 'processando'"
          @click="baixarRelatorio"
        >
          {{ estadoRelatorio === "processando" ? "Gerando..." : "Baixar relatório" }}
        </UButton>
      </UTooltip>

      <UTooltip text="Baixa o PDF de todos os documentos listados (pula os já baixados).">
        <UButton
          icon="i-lucide-folder-down"
          color="neutral"
          variant="ghost"
          size="sm"
          class="alvo-minimo"
          :loading="baixandoTodos"
          :disabled="!temItens || baixandoTodos"
          @click="baixarTodosPdfs"
        >
          {{ baixandoTodos ? "Baixando..." : "Baixar todos os PDFs" }}
        </UButton>
      </UTooltip>

      <UTooltip :text="erroZip ?? 'Baixa um ZIP com os PDFs já baixados deste SIAPE.'">
        <UButton
          icon="i-lucide-download"
          :color="estadoZip === 'erro' ? 'error' : 'neutral'"
          variant="ghost"
          size="sm"
          class="alvo-minimo"
          :loading="estadoZip === 'processando'"
          :disabled="!temItens || estadoZip === 'processando'"
          @click="baixarDocumentosZip"
        >
          {{ estadoZip === "processando" ? "Baixando..." : "Baixar ZIP" }}
        </UButton>
      </UTooltip>
    </div>

    <div
      v-if="progresso"
      class="relatorio-acoes__progresso"
      role="status"
      aria-live="polite"
    >
      <span class="relatorio-acoes__progresso-rotulo mono">
        Baixando {{ progresso.atual }} de {{ progresso.total }}
      </span>
      <UProgress :model-value="progresso.atual" :max="progresso.total" size="sm" />
    </div>

    <p v-if="erroRelatorio || erroZip" role="alert" class="relatorio-acoes__erro">
      {{ erroRelatorio ?? erroZip }}
    </p>
  </div>
</template>

<style scoped>
.relatorio-acoes {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: var(--sp-1);
}

.relatorio-acoes__botoes {
  display: flex;
  gap: var(--sp-1);
}

.relatorio-acoes__progresso {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: var(--sp-1);
  width: 100%;
  min-width: 220px;
}

.relatorio-acoes__progresso-rotulo {
  font-size: var(--text-13);
  color: var(--muted);
}

.relatorio-acoes__progresso :deep([role="progressbar"]),
.relatorio-acoes__progresso > * {
  width: 100%;
}

.relatorio-acoes__erro {
  margin: 0;
  font-size: var(--text-13);
  color: var(--danger);
  text-align: right;
}
</style>
