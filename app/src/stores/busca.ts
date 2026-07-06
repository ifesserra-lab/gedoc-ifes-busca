// ViewModel/estado (Pinia) — ponte entre BuscaView e o comando IPC
// `buscar_por_siape`. Mantém a View "fina": nenhuma regra de negócio aqui
// além da validação client-side (R10), que só antecipa feedback.

import { computed, ref } from "vue";
import { defineStore } from "pinia";

import { buscarPorSiape, mensagemDeErro, type ResultadoView } from "@/services/ipc";
import { MENSAGEM_SIAPE_INVALIDO, validarSiape } from "@/utils/siape";

export type EstadoBusca = "idle" | "loading" | "erro" | "resultado";

export const useBuscaStore = defineStore("busca", () => {
  const siape = ref("");
  const estado = ref<EstadoBusca>("idle");
  const erro = ref<string | null>(null);
  const resultado = ref<ResultadoView | null>(null);
  /** Estado de UI (filtro por chip) — vive na store, a View só apresenta. */
  const categoriaSelecionada = ref<string | null>(null);
  /**
   * US6 — liga classificação+resumo via IA (`modo: "llm"`). Default `false`:
   * o modo `keyword` é grátis e instantâneo; IA tem custo/latência (R9), por
   * isso é opt-in explícito do usuário, nunca ligado sozinho numa busca.
   */
  const usarIa = ref(false);

  const siapeValido = computed(() => validarSiape(siape.value));

  /** `resultado.total === 0` → estado "vazio" (US2/T021), distinto de "resultado". */
  const vazio = computed(() => estado.value === "resultado" && resultado.value?.total === 0);

  /** Grupo(s) a exibir considerando o chip selecionado ("Todas" quando null). */
  const gruposFiltrados = computed(() => {
    const grupos = resultado.value?.categorias ?? [];
    if (!categoriaSelecionada.value) return grupos;
    return grupos.filter((grupo) => grupo.categoria === categoriaSelecionada.value);
  });

  async function buscar(): Promise<void> {
    if (!siapeValido.value) {
      estado.value = "erro";
      erro.value = MENSAGEM_SIAPE_INVALIDO;
      resultado.value = null;
      return;
    }

    estado.value = "loading";
    erro.value = null;
    categoriaSelecionada.value = null;
    try {
      resultado.value = await buscarPorSiape({
        siape: siape.value,
        modo: usarIa.value ? "llm" : "keyword",
      });
      estado.value = "resultado";
    } catch (motivo) {
      estado.value = "erro";
      erro.value = mensagemDeErro(motivo);
    }
  }

  function selecionarCategoria(categoria: string | null): void {
    categoriaSelecionada.value = categoria;
  }

  function reiniciar(): void {
    siape.value = "";
    estado.value = "idle";
    erro.value = null;
    resultado.value = null;
    categoriaSelecionada.value = null;
  }

  return {
    siape,
    estado,
    erro,
    resultado,
    siapeValido,
    vazio,
    categoriaSelecionada,
    gruposFiltrados,
    usarIa,
    buscar,
    selecionarCategoria,
    reiniciar,
  };
});
