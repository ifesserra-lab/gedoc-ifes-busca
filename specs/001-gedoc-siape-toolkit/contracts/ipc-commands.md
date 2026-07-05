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

## baixar_zip
- **Entrada**: `{ siape: string }`
- **Saída**: caminho do ZIP gerado (ou stream de bytes).
- **US**: US7. **Regras**: R3, R7.

## abrir_documento
- **Entrada**: `{ siape: string, arquivo: string }`
- **Saída**: abre/retorna o PDF individual; `arquivo` **sanitizado** (sem `/`,`\`).
- **US**: US4. **Regras**: R3, R7.

## gerar_pdf_resumo
- **Entrada**: `{ siape: string }`
- **Saída**: caminho do PDF do relatório.
- **US**: US7. **Regras**: R1.

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
