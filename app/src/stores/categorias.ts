// ViewModel/estado (Pinia) — CRUD de categorias (R5: nome obrigatório e
// único; categoria "Outros" é reservada pelo domínio — ver
// src-tauri/src/domain/categoria.rs).
//
// TODO(IPC): o backend ainda não expõe `listar_categorias`/`salvar_categorias`
// (persistência em `config/categoria.json`, Constituição IV) — só o modelo
// `Categoria` e os erros `CategoriaSemNome`/`NomeDuplicado` já existem em
// Rust (TODO de US8). Enquanto o comando não existe, esta store mantém as
// categorias em memória (sessão) com a MESMA validação que o backend fará,
// para a tela já funcionar de ponta a ponta. Quando o IPC existir, trocar
// `carregar`/`salvar`/`remover` para chamar `services/ipc.ts` (pedir ao
// agente `tauri-mvc-expert` o comando — ver contracts/ui-components.md,
// seção "Não-metas").

import { computed, ref } from "vue";
import { defineStore } from "pinia";

export interface CategoriaItem {
  nome: string;
  descricao: string;
}

export type EstadoCategorias = "idle" | "vazio" | "pronto" | "erro";

export const useCategoriasStore = defineStore("categorias", () => {
  const itens = ref<CategoriaItem[]>([]);
  const estado = ref<EstadoCategorias>("idle");
  const erro = ref<string | null>(null);
  const mensagemSucesso = ref<string | null>(null);

  const vazio = computed(() => itens.value.length === 0);

  function carregar(): void {
    estado.value = vazio.value ? "vazio" : "pronto";
  }

  /** R5 — nome obrigatório e único (case-insensitive), espelha `AppError`. */
  function validarNome(nome: string, ignorarIndice: number | null): string | null {
    const limpo = nome.trim();
    if (!limpo) return "Informe um nome para a categoria.";
    const duplicado = itens.value.some(
      (item, indice) => indice !== ignorarIndice && item.nome.trim().toLowerCase() === limpo.toLowerCase(),
    );
    if (duplicado) return `Categoria já existe: '${limpo}'`;
    return null;
  }

  /** Retorna `null` quando salvo com sucesso, ou a mensagem de erro. */
  function salvar(categoria: CategoriaItem, indice: number | null): string | null {
    const problema = validarNome(categoria.nome, indice);
    if (problema) {
      erro.value = problema;
      return problema;
    }

    const registro: CategoriaItem = { nome: categoria.nome.trim(), descricao: categoria.descricao.trim() };
    if (indice === null) {
      itens.value.push(registro);
    } else {
      itens.value.splice(indice, 1, registro);
    }

    erro.value = null;
    mensagemSucesso.value = indice === null ? "Categoria criada." : "Categoria atualizada.";
    estado.value = "pronto";
    return null;
  }

  function remover(indice: number): void {
    itens.value.splice(indice, 1);
    estado.value = vazio.value ? "vazio" : "pronto";
    mensagemSucesso.value = "Categoria removida.";
  }

  function limparMensagens(): void {
    erro.value = null;
    mensagemSucesso.value = null;
  }

  return {
    itens,
    estado,
    erro,
    mensagemSucesso,
    vazio,
    carregar,
    validarNome,
    salvar,
    remover,
    limparMensagens,
  };
});
