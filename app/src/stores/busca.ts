// ViewModel/estado (Pinia) — ponte entre BuscaView e o comando IPC
// `buscar_por_siape`. Mantém a View "fina": nenhuma regra de negócio aqui
// além da validação client-side (R10), que só antecipa feedback.

import { computed, ref } from "vue";
import { defineStore } from "pinia";

import {
  baixarDocumento,
  buscarPorSiape,
  type DocView,
  mensagemDeErro,
  type ResultadoView,
} from "@/services/ipc";
import { MENSAGEM_SIAPE_INVALIDO, validarSiape } from "@/utils/siape";

export type EstadoBusca = "idle" | "loading" | "erro" | "resultado";

/** Progresso do download em lote (US #22): documento atual / total. */
export interface ProgressoDownload {
  atual: number;
  total: number;
}

/** Resumo do download em lote: quantos baixaram e quantos falharam. */
export interface ResumoDownload {
  ok: number;
  falhas: number;
}

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
  /**
   * US7 — indica se o `resultado` atual foi produzido no modo IA (`llm`).
   * O relatório consolida os RESUMOS da IA, então só faz sentido (e só é
   * habilitado na tela) quando a busca foi feita com IA. Marcado no fim de
   * cada busca com o `usarIa` daquela busca — não muda se o toggle for
   * alternado depois.
   */
  const resultadoComIa = ref(false);
  /**
   * US #22 — progresso do "Baixar todos os PDFs". `null` quando não há
   * download em lote em andamento; caso contrário, `{ atual, total }` para a
   * barra de progresso. Estado de UI, vive na store (a View só apresenta).
   */
  const downloadProgresso = ref<ProgressoDownload | null>(null);

  const siapeValido = computed(() => validarSiape(siape.value));

  /** True enquanto um download em lote está em andamento (desabilita a ação). */
  const baixandoTodos = computed(() => downloadProgresso.value !== null);

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
      const comIa = usarIa.value;
      resultado.value = await buscarPorSiape({
        siape: siape.value,
        modo: comIa ? "llm" : "keyword",
      });
      resultadoComIa.value = comIa;
      estado.value = "resultado";
    } catch (motivo) {
      estado.value = "erro";
      erro.value = mensagemDeErro(motivo);
    }
  }

  /**
   * US #22 — baixa o PDF de cada documento de `docs`, um a um, reusando o
   * comando `baixar_documento` (idempotente: pula os já baixados). Atualiza
   * `downloadProgresso` a cada documento para a barra de progresso. R11: a
   * falha em um documento não aborta o lote — é contada e o processo segue.
   * Recebe `docs`/`siape` explicitamente (o chamador passa os itens exibidos),
   * mantendo a store como única dona do estado de progresso. No-op (0/0) sem
   * documentos ou SIAPE.
   */
  async function baixarTodos(docs: DocView[], siapeDoc: string): Promise<ResumoDownload> {
    if (!siapeDoc || docs.length === 0) return { ok: 0, falhas: 0 };

    let ok = 0;
    let falhas = 0;
    downloadProgresso.value = { atual: 0, total: docs.length };
    try {
      for (const doc of docs) {
        try {
          await baixarDocumento({ siape: siapeDoc, link: doc.link, titulo: doc.titulo, data: doc.data });
          ok += 1;
        } catch {
          falhas += 1; // R11: um documento com erro não derruba o lote
        }
        downloadProgresso.value = { atual: ok + falhas, total: docs.length };
      }
    } finally {
      downloadProgresso.value = null;
    }
    return { ok, falhas };
  }

  function selecionarCategoria(categoria: string | null): void {
    categoriaSelecionada.value = categoria;
  }

  function reiniciar(): void {
    siape.value = "";
    estado.value = "idle";
    erro.value = null;
    resultado.value = null;
    resultadoComIa.value = false;
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
    resultadoComIa,
    downloadProgresso,
    baixandoTodos,
    buscar,
    baixarTodos,
    selecionarCategoria,
    reiniciar,
  };
});
