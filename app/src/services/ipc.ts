// Wrapper tipado de `invoke()` — única porta de entrada para o IPC Tauri.
// Nenhum `.vue`/store deve chamar `invoke` diretamente (ver
// contracts/ipc-commands.md).

import { invoke } from "@tauri-apps/api/core";

export interface BuscarPorSiapeInput {
  siape: string;
  repositorio?: "0" | "1" | "2";
}

export interface DocView {
  titulo: string;
  data: string | null;
  link: string;
  arquivo: string | null;
  resumo: string | null;
}

export interface CategoriaGrupo {
  categoria: string;
  qtd: number;
  itens: DocView[];
}

export interface ResultadoView {
  termo: string;
  total: number;
  categorias: CategoriaGrupo[];
  tem_pdf: boolean;
}

/** Espelha `AppError` (thiserror + serde, `#[serde(tag = "tipo", content = "mensagem")]`). */
export type AppErrorPayload =
  | { tipo: "SiapeInvalido"; mensagem: { termo: string } }
  | { tipo: "FalhaPortal"; mensagem: { motivo: string } }
  | { tipo: "FalhaIA"; mensagem: { motivo: string } }
  | { tipo: "CategoriaSemNome"; mensagem: null }
  | { tipo: "NomeDuplicado"; mensagem: { nome: string } }
  | { tipo: "NaoImplementado"; mensagem: string };

function ehAppErrorPayload(valor: unknown): valor is AppErrorPayload {
  return typeof valor === "object" && valor !== null && "tipo" in valor;
}

/** Converte o erro (rejeição de `invoke`) numa mensagem amigável, sem stack trace. */
export function mensagemDeErro(erro: unknown): string {
  if (!ehAppErrorPayload(erro)) {
    return "Erro inesperado ao comunicar com o backend.";
  }
  switch (erro.tipo) {
    case "SiapeInvalido":
      return `SIAPE inválido: '${erro.mensagem.termo}'. Informe de 5 a 8 dígitos numéricos.`;
    case "FalhaPortal":
      return `Falha ao comunicar com o portal GeDoc: ${erro.mensagem.motivo}`;
    case "FalhaIA":
      return `Falha no serviço de IA: ${erro.mensagem.motivo}`;
    case "CategoriaSemNome":
      return "Categoria sem nome.";
    case "NomeDuplicado":
      return `Categoria já existe: '${erro.mensagem.nome}'`;
    case "NaoImplementado":
      return `Recurso ainda não implementado: ${erro.mensagem}`;
    default:
      return "Erro inesperado ao comunicar com o backend.";
  }
}

/**
 * US1/US2/US3 — busca documentos do GeDoc para um SIAPE. O backend faz a
 * coleta real no portal (GedocRepository), filtra por SIAPE (R2) e agrupa por
 * categoria; erros chegam como `AppError` (ver `mensagemDeErro`).
 */
export async function buscarPorSiape(input: BuscarPorSiapeInput): Promise<ResultadoView> {
  return invoke<ResultadoView>("buscar_por_siape", { input });
}
