// Porta única de acesso ao backend — dual-mode (US web 003):
//  - Desktop (Tauri): `invoke()` dos comandos IPC.
//  - Web (navegador): `fetch()` da API HTTP (crate `server/`), escolhido em
//    runtime por `emTauri()`. As assinaturas exportadas são idênticas nos dois
//    modos, então stores/views não mudam.
// Nenhum `.vue`/store deve chamar `invoke`/`fetch` diretamente.

import { invoke } from "@tauri-apps/api/core";

/** Base da API web (Vercel → container). Vazio no desktop. */
const API_BASE = (
  (import.meta as unknown as { env?: Record<string, string | undefined> }).env
    ?.VITE_API_URL ?? ""
).replace(/\/+$/, "");

/** Runtime Tauri? (desktop) — senão, modo web. */
function emTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * `fetch` da API web. Envia o cookie de sessão (`credentials: "include"`) e,
 * em erro, rejeita com o MESMO payload `{tipo,mensagem}` do IPC — assim
 * `mensagemDeErro` funciona igual nos dois modos.
 */
async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const resp = await fetch(`${API_BASE}${path}`, {
    credentials: "include",
    headers:
      init?.body != null
        ? { "Content-Type": "application/json", ...(init?.headers ?? {}) }
        : init?.headers,
    ...init,
  });
  if (!resp.ok) {
    throw await erroDaResposta(resp);
  }
  return (await resp.json()) as T;
}

/** Extrai o `AppErrorPayload` do corpo de uma resposta não-ok. */
async function erroDaResposta(resp: Response): Promise<unknown> {
  try {
    return await resp.json();
  } catch {
    return {
      tipo: "FalhaPortal",
      mensagem: { motivo: `HTTP ${resp.status}` },
    } satisfies AppErrorPayload;
  }
}

/** URL absoluta de um endpoint da API web (para `window.open`). */
function apiUrl(path: string): string {
  return `${API_BASE}${path}`;
}

export interface BuscarPorSiapeInput {
  siape: string;
  repositorio?: "0" | "1" | "2";
  /**
   * Estratégia de classificação/resumo (US5/US6): `"llm"` também liga o
   * resumo por documento (backend: `commands::buscar`). Ausente ou
   * `"keyword"` (default) é grátis e instantâneo — sem tocar a API de IA.
   */
  modo?: "keyword" | "llm";
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
  | { tipo: "NaoImplementado"; mensagem: string }
  | { tipo: "FalhaArquivo"; mensagem: { motivo: string } }
  | { tipo: "LimiteTaxa"; mensagem: { motivo: string } };

function ehAppErrorPayload(valor: unknown): valor is AppErrorPayload {
  return typeof valor === "object" && valor !== null && "tipo" in valor;
}

/** Converte o erro (rejeição de `invoke`/`fetch`) numa mensagem amigável, sem stack trace. */
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
    case "FalhaArquivo":
      return `Falha ao acessar arquivo: ${erro.mensagem.motivo}`;
    case "LimiteTaxa":
      return erro.mensagem.motivo;
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
  if (emTauri()) return invoke<ResultadoView>("buscar_por_siape", { input });
  return apiFetch<ResultadoView>("/api/buscar", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

export interface BaixarDocumentoInput {
  siape: string;
  link: string;
  titulo: string;
  data: string | null;
}

export interface AbrirDocumentoInput {
  siape: string;
  arquivo: string;
}

/**
 * US4 — baixa o PDF de um documento para o armazenamento (desktop: dir do app;
 * web: dir da sessão — PII de terceiros, R7/LGPD) e devolve só o **nome** do
 * arquivo gravado (nunca o caminho absoluto).
 */
export async function baixarDocumento(input: BaixarDocumentoInput): Promise<string> {
  if (emTauri()) return invoke<string>("baixar_documento", { input });
  const r = await apiFetch<{ arquivo: string }>("/api/documento/baixar", {
    method: "POST",
    body: JSON.stringify(input),
  });
  return r.arquivo;
}

/**
 * US4 — abre um PDF já baixado. Desktop: app padrão do sistema. Web: nova aba
 * do navegador (`GET /api/documento/:siape/:arquivo`). `arquivo` é o nome
 * devolvido por `baixarDocumento` (sanitizado no backend, R7).
 */
export async function abrirDocumento(input: AbrirDocumentoInput): Promise<void> {
  if (emTauri()) return invoke<void>("abrir_documento", { input });
  const url = apiUrl(
    `/api/documento/${encodeURIComponent(input.siape)}/${encodeURIComponent(input.arquivo)}`,
  );
  window.open(url, "_blank", "noopener");
}

export interface Categoria {
  nome: string;
  descricao?: string | null;
}

export interface SalvarCategoriasResposta {
  ok: boolean;
  total: number;
}

/**
 * US8 — lista as categorias persistidas (desktop: `app_config_dir`; web:
 * arquivo global no servidor, semeado de `config/categoria.json`). Lista vazia
 * é um resultado válido (estado "vazio" da tela), não um erro.
 */
export async function listarCategorias(): Promise<Categoria[]> {
  if (emTauri()) return invoke<Categoria[]>("listar_categorias");
  return apiFetch<Categoria[]>("/api/categorias");
}

/**
 * US8 — substitui a lista completa de categorias. O backend valida R5 (nome
 * obrigatório e único, case-insensitive) antes de gravar; nada é escrito se
 * a validação falhar (ver `mensagemDeErro` para `CategoriaSemNome`/
 * `NomeDuplicado`).
 */
export async function salvarCategorias(categorias: Categoria[]): Promise<SalvarCategoriasResposta> {
  if (emTauri()) return invoke<SalvarCategoriasResposta>("salvar_categorias", { categorias });
  return apiFetch<SalvarCategoriasResposta>("/api/categorias", {
    method: "PUT",
    body: JSON.stringify(categorias),
  });
}

/**
 * US7 — gera o relatório consolidado (Markdown + HTML self-contained) a partir
 * do MESMO `resultado` já mostrado na tela (R1) e o abre (desktop: app padrão;
 * web: nova aba, `GET /api/relatorio/:siape`). Devolve o **nome** do arquivo.
 */
export async function gerarRelatorio(resultado: ResultadoView): Promise<string> {
  if (emTauri()) return invoke<string>("gerar_relatorio", { resultado });
  const r = await apiFetch<{ arquivo: string }>("/api/relatorio", {
    method: "POST",
    body: JSON.stringify(resultado),
  });
  window.open(apiUrl(`/api/relatorio/${encodeURIComponent(resultado.termo)}`), "_blank", "noopener");
  return r.arquivo;
}

/**
 * US7 — monta um ZIP com os PDFs já baixados para `siape` (desktop: revela no
 * gerenciador de arquivos; web: baixa o `.zip`). Sem nenhum PDF baixado na
 * sessão, rejeita com `FalhaArquivo` (mensagem amigável — ver `mensagemDeErro`).
 */
export async function baixarZip(siape: string): Promise<string> {
  if (emTauri()) return invoke<string>("baixar_zip", { siape });

  const resp = await fetch(apiUrl(`/api/zip/${encodeURIComponent(siape)}`), {
    credentials: "include",
  });
  if (!resp.ok) throw await erroDaResposta(resp);

  const nome = `${siape}_documentos.zip`;
  const blob = await resp.blob();
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = nome;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
  return nome;
}
