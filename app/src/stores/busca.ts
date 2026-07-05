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

  const siapeValido = computed(() => validarSiape(siape.value));

  async function buscar(): Promise<void> {
    if (!siapeValido.value) {
      estado.value = "erro";
      erro.value = MENSAGEM_SIAPE_INVALIDO;
      resultado.value = null;
      return;
    }

    estado.value = "loading";
    erro.value = null;
    try {
      resultado.value = await buscarPorSiape({ siape: siape.value });
      estado.value = "resultado";
    } catch (motivo) {
      estado.value = "erro";
      erro.value = mensagemDeErro(motivo);
    }
  }

  function reiniciar(): void {
    siape.value = "";
    estado.value = "idle";
    erro.value = null;
    resultado.value = null;
  }

  return { siape, estado, erro, resultado, siapeValido, buscar, reiniciar };
});
