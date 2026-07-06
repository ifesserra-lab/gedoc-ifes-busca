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

import { baixarZip, gerarRelatorio, mensagemDeErro, type ResultadoView } from "@/services/ipc";

const props = defineProps<{
  resultado: ResultadoView | null;
}>();

type Estado = "idle" | "processando" | "erro";

const estadoRelatorio = ref<Estado>("idle");
const erroRelatorio = ref<string | null>(null);
const estadoZip = ref<Estado>("idle");
const erroZip = ref<string | null>(null);

/** Só habilita as ações quando há ao menos 1 documento para exportar — um
 * `resultado.total > 0` de portal não basta: R2 pode descartar tudo e deixar
 * `categorias` sem nenhum item. */
const temItens = computed(
  () => props.resultado?.categorias.some((grupo) => grupo.itens.length > 0) ?? false,
);

async function baixarRelatorio(): Promise<void> {
  if (!props.resultado || !temItens.value || estadoRelatorio.value === "processando") return;

  estadoRelatorio.value = "processando";
  erroRelatorio.value = null;
  try {
    await gerarRelatorio(props.resultado);
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
      <UTooltip
        :text="
          erroRelatorio ??
          'Gera um relatório consolidado (HTML) com os resumos, agrupado por categoria.'
        "
      >
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

.relatorio-acoes__erro {
  margin: 0;
  font-size: var(--text-13);
  color: var(--danger);
  text-align: right;
}
</style>
