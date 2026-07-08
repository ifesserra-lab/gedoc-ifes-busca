---
description: "Task list — Botão Baixar PDF no relatório"
---

# Tasks: Botão "Baixar PDF" no relatório

**Tests**: obrigatórios (VII). HTML é string pura → testável sem rede.

## Phase 3: US1 (P1) 🎯

- [x] T001 [US1] Em `markdown_para_html` (`core/src/services/relatorio.rs`), injetar no início do `<body>` um `<div class="acoes no-print"><button class="btn-pdf" type="button" onclick="window.print()">Baixar PDF</button></div>`.
- [x] T002 [US1] No CSS embutido: estilos de `.acoes`/`.btn-pdf` com tokens (`--accent`/`--accent-contrast`), `--accent-contrast` nos `:root` (claro/escuro/print), e `.no-print { display:none }` dentro do `@media print`.
- [x] T003 [US1] Teste em `relatorio.rs` (mod tests): o HTML contém o botão "Baixar PDF" com `onclick="window.print()"` e a regra `.no-print` no `@media print`; segue self-contained (sem `<script`, sem `http`).
- [x] T004 [US1] Confirmar testes existentes verdes (conteúdo/estrutura, design system spec 007, self-contained).

## Phase 4: Polish

- [x] T005 [P] `cargo fmt` + `cargo clippy --all-targets -- -D warnings` + `cargo test` no `core`.
- [ ] T006 Validar: abrir um relatório, ver o botão, clicar → diálogo de PDF; conferir que o botão some no PDF.

## Notes

- `onclick` inline (não `<script>`) mantém "sem asset/JS externo" e o teste
  `html_e_self_contained...`.
- Só apresentação; conteúdo/estrutura inalterados (FR-005).
