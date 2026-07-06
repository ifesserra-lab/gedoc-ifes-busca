// Wrapper tipado de `invoke()` — única porta de entrada para o IPC Tauri.
// Nenhum `.vue`/store deve chamar `invoke` diretamente (ver
// contracts/ipc-commands.md).

import { invoke } from "@tauri-apps/api/core";

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
  | { tipo: "FalhaArquivo"; mensagem: { motivo: string } };

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
    case "FalhaArquivo":
      return `Falha ao acessar arquivo: ${erro.mensagem.motivo}`;
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
 * US4 — baixa o PDF de um documento para o diretório de dados do app
 * (nunca o repositório — PII de terceiros, R7) e devolve só o **nome** do
 * arquivo gravado (nunca o caminho absoluto).
 */
export async function baixarDocumento(input: BaixarDocumentoInput): Promise<string> {
  return invoke<string>("baixar_documento", { input });
}

/**
 * US4 — abre, com o aplicativo padrão do sistema, um PDF já baixado.
 * `arquivo` é o nome devolvido por `baixarDocumento` (sanitizado no backend,
 * R7); nunca um caminho arbitrário.
 */
export async function abrirDocumento(input: AbrirDocumentoInput): Promise<void> {
  return invoke<void>("abrir_documento", { input });
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
 * US8 — lista as categorias persistidas (`AppHandle.path().app_config_dir()/categoria.json`,
 * semeado a partir do `config/categoria.json` empacotado na primeira
 * execução — ver `commands::categorias` no backend). Lista vazia é um
 * resultado válido (estado "vazio" da tela), não um erro.
 */
export async function listarCategorias(): Promise<Categoria[]> {
  return invoke<Categoria[]>("listar_categorias");
}

/**
 * US8 — substitui a lista completa de categorias. O backend valida R5 (nome
 * obrigatório e único, case-insensitive) antes de gravar; nada é escrito se
 * a validação falhar (ver `mensagemDeErro` para `CategoriaSemNome`/
 * `NomeDuplicado`).
 */
export async function salvarCategorias(categorias: Categoria[]): Promise<SalvarCategoriasResposta> {
  return invoke<SalvarCategoriasResposta>("salvar_categorias", { categorias });
}

/**
 * US7 — gera o relatório consolidado (Markdown + HTML self-contained, ver
 * decisão de PDF em `services::relatorio` no backend: nada de Chrome
 * headless) a partir do MESMO `resultado` já mostrado na tela (R1 — reflete
 * os resumos reais, sem refazer a busca) e abre o HTML com o app padrão do
 * sistema. Devolve só o **nome** do arquivo gravado (nunca o caminho
 * absoluto — R7), salvo em `<app_data_dir>/relatorios/`.
 */
export async function gerarRelatorio(resultado: ResultadoView): Promise<string> {
  return invoke<string>("gerar_relatorio", { resultado });
}

/**
 * US7 — monta um ZIP com todos os PDFs já baixados (US4) para `siape` em
 * `<app_data_dir>/relatorios/<siape>_documentos.zip` e revela o arquivo no
 * gerenciador de arquivos do SO. Sem nenhum PDF baixado ainda, rejeita com
 * `FalhaArquivo` (mensagem amigável — ver `mensagemDeErro`).
 */
export async function baixarZip(siape: string): Promise<string> {
  return invoke<string>("baixar_zip", { siape });
}
