// ViewModel/estado (Pinia) — CRUD de categorias (R5: nome obrigatório e
// único; categoria "Outros" é reservada pelo domínio — ver
// src-tauri/src/domain/categoria.rs).
//
// Persistência real via IPC (US8, backend em src-tauri/src/commands/categorias.rs):
// `carregar()` chama `listar_categorias` no mount; `salvar()`/`remover()`
// persistem a lista completa via `salvar_categorias` (o backend substitui o
// arquivo inteiro — não há endpoint de "patch" de um item). A validação de
// nome (R5) roda primeiro no cliente (`validarNome`), para feedback
// instantâneo sem round-trip de IPC; o backend valida de novo (fonte de
// verdade) e uma rejeição vinda de lá também aparece em `erro`/`mensagemDeErro`.

import { computed, ref } from "vue";
import { defineStore } from "pinia";

import { type Categoria, listarCategorias, mensagemDeErro, salvarCategorias } from "@/services/ipc";

export interface CategoriaItem {
  nome: string;
  descricao: string;
}

export type EstadoCategorias = "idle" | "carregando" | "vazio" | "pronto" | "erro";

function paraItem(categoria: Categoria): CategoriaItem {
  return { nome: categoria.nome, descricao: categoria.descricao ?? "" };
}

function paraCategoria(item: CategoriaItem): Categoria {
  return { nome: item.nome.trim(), descricao: item.descricao.trim() || null };
}

export const useCategoriasStore = defineStore("categorias", () => {
  const itens = ref<CategoriaItem[]>([]);
  const estado = ref<EstadoCategorias>("idle");
  const erro = ref<string | null>(null);
  const mensagemSucesso = ref<string | null>(null);

  const vazio = computed(() => itens.value.length === 0);

  /** US8 — busca a lista persistida via IPC. */
  async function carregar(): Promise<void> {
    estado.value = "carregando";
    erro.value = null;
    try {
      const categorias = await listarCategorias();
      itens.value = categorias.map(paraItem);
      estado.value = vazio.value ? "vazio" : "pronto";
    } catch (motivo) {
      estado.value = "erro";
      erro.value = mensagemDeErro(motivo);
    }
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

  /** Persiste a lista completa via IPC; em sucesso, `itens` passa a refleti-la. */
  async function persistir(novaLista: CategoriaItem[]): Promise<string | null> {
    try {
      await salvarCategorias(novaLista.map(paraCategoria));
      itens.value = novaLista;
      erro.value = null;
      return null;
    } catch (motivo) {
      const mensagem = mensagemDeErro(motivo);
      erro.value = mensagem;
      return mensagem;
    }
  }

  /** Retorna `null` quando salvo com sucesso, ou a mensagem de erro. */
  async function salvar(categoria: CategoriaItem, indice: number | null): Promise<string | null> {
    const problema = validarNome(categoria.nome, indice);
    if (problema) {
      erro.value = problema;
      return problema;
    }

    const registro: CategoriaItem = { nome: categoria.nome.trim(), descricao: categoria.descricao.trim() };
    const novaLista = [...itens.value];
    if (indice === null) {
      novaLista.push(registro);
    } else {
      novaLista.splice(indice, 1, registro);
    }

    const falha = await persistir(novaLista);
    if (falha) return falha;

    mensagemSucesso.value = indice === null ? "Categoria criada." : "Categoria atualizada.";
    estado.value = "pronto";
    return null;
  }

  /** Retorna `null` quando removido com sucesso, ou a mensagem de erro. */
  async function remover(indice: number): Promise<string | null> {
    const novaLista = itens.value.filter((_, i) => i !== indice);

    const falha = await persistir(novaLista);
    if (falha) return falha;

    estado.value = vazio.value ? "vazio" : "pronto";
    mensagemSucesso.value = "Categoria removida.";
    return null;
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
