# Contratos IPC (Tauri) — GeDoc IFES Toolkit

Fase 1. Comandos `#[tauri::command]` expostos pelo backend Rust à View (Vue via
`invoke`). Contratos tipados; erros retornam `AppError` serializável.

## buscar_por_siape
- **Entrada**: `{ siape: string, repositorio?: "0"|"1"|"2" }`
- **Saída**: `ResultadoView { termo, total, categorias: CategoriaGrupo[], tem_pdf }`
  onde `CategoriaGrupo = { categoria, qtd, itens: DocView[] }` e
  `DocView = { titulo, data, link, arquivo?, resumo }`.
- **Erros**: `SiapeInvalido` (R10), `FalhaPortal`, `FalhaIA`.
- **US**: US1, US2, US3, US5, US6. **Regras**: R1, R2, R6, R8, R9, R10.

## baixar_documento
- **Entrada**: `{ siape: string, link: string, titulo: string, data?: string }`
- **Saída**: nome do arquivo gravado (`AAAA_NUMERO_ASSUNTO.pdf`, R3) — **nunca** o
  caminho absoluto (R7). Grava em `<app_data_dir>/documentos/<siape>/`, fora do
  repositório. Idempotente: reexecutar não rebaixa um arquivo já presente.
- **Erros**: `SiapeInvalido` (R10), `FalhaPortal` (rede), `FalhaArquivo` (disco).
- **US**: US4. **Regras**: R3, R7, R10.

## baixar_zip
- **Entrada**: `{ siape: string }`
- **Saída**: **nome** do ZIP gravado (`<siape>_documentos.zip`, R3) — nunca o
  caminho absoluto (R7). Monta, a partir de `<app_data_dir>/documentos/<siape>/`
  (US4), um ZIP com todo `*.pdf` já baixado, em
  `<app_data_dir>/relatorios/<siape>_documentos.zip`, e revela o arquivo no
  gerenciador de arquivos do SO.
- **Erros**: `SiapeInvalido` (R10), `FalhaArquivo` (nenhum PDF baixado ainda —
  mensagem amigável — ou falha de disco).
- **US**: US7. **Regras**: R3, R7.

## abrir_documento
- **Entrada**: `{ siape: string, arquivo: string }`
- **Saída**: abre, com o app padrão do SO, o PDF já baixado em
  `<app_data_dir>/documentos/<siape>/<arquivo>`; `arquivo` **sanitizado** (sem
  `/`, `\`, `..` ou vazio) antes de qualquer acesso a disco.
- **Erros**: `SiapeInvalido` (R10), `FalhaArquivo` (nome inválido ou arquivo
  inexistente).
- **US**: US4. **Regras**: R3, R7.

## gerar_relatorio
- **Entrada**: `{ resultado: ResultadoView }` — a mesma `ResultadoView` que
  `buscar_por_siape` devolveu à View (o relatório reflete a busca já mostrada
  na tela, R1; não refaz a busca nem toca rede).
- **Saída**: **nome** do HTML gravado (`<siape>_relatorio.html`, R3) — nunca o
  caminho absoluto (R7). Gera um Markdown agrupado por categoria
  (`services::relatorio::gerar_markdown`) e sua versão HTML self-contained
  (`markdown_para_html`, CSS A4 inline, sem assets externos), grava os dois em
  `<app_data_dir>/relatorios/` e abre o HTML com o app padrão do sistema —
  **decisão**: sem Chrome headless/binário externo; "Imprimir → Salvar como
  PDF" no navegador produz um PDF equivalente.
- **Erros**: `SiapeInvalido` (R10, valida `resultado.termo`), `FalhaArquivo`.
- **US**: US7. **Regras**: R1, R3, R7.

## listar_categorias
- **Entrada**: `{}` · **Saída**: `Categoria[]` (de `config/categoria.json`).
- **US**: US8. **Regras**: R5.

## salvar_categorias
- **Entrada**: `{ categorias: Categoria[] }` (nome obrigatório, único)
- **Saída**: `{ ok: true, total }` · **Erros**: `CategoriaSemNome`, `NomeDuplicado`.
- **US**: US8. **Regras**: R5.

## Regras gerais dos contratos
- Toda entrada de IPC é validada no backend (R10, sanitização de caminho).
- Falha de um item não derruba o lote (R9).
- Nenhum comando expõe caminho absoluto de `data/`/`config/` ao cliente além do
  necessário (R7).
