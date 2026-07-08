---
description: "Task list — Aplicar o design system no relatório"
---

# Tasks: Aplicar o design system no relatório

**Tests**: obrigatórios (VII). HTML é string pura → testável sem rede.

## Phase 3: US1 (P1) 🎯

- [x] T001 [US1] Reescrever a const `CSS` em `core/src/services/relatorio.rs` com os tokens do app: `:root` (tema claro — paper/surface/ink/muted/border/accent/accent-soft), `@media (prefers-color-scheme: dark)` (tema escuro), tipografia `Inter` (stack, sem embutir), escala/espaçamento aprovados, e `@media print` (impressão legível, força claro). Cabeçalhos com acento, tabelas com `--border`/`--surface-2`, meta/resumo em `--muted`.
- [x] T002 [US1] Teste em `relatorio.rs` (mod tests): `markdown_para_html` gera CSS com os tokens do design (ex.: acento `#17784e`/`--accent`), `prefers-color-scheme: dark` e `Inter`; e **self-contained** (sem `@import` nem `url(http`).
- [x] T003 [US1] Confirmar testes existentes de conteúdo/estrutura seguem verdes (R1/XSS: `o_html_gerado_reflete_o_resumo...`, escape de título, pipeline `gerar_markdown → markdown_para_html`).

## Phase 4: Polish

- [x] T004 [P] `cargo fmt` + `cargo clippy --all-targets -- -D warnings` + `cargo test` no `core`.
- [ ] T005 Validar visual: gerar um relatório e conferir paleta/tipografia/tema (claro/escuro) + impressão ("Salvar como PDF").

## Notes

- Só apresentação: conteúdo, estrutura e nomes de arquivo inalterados (FR-004).
- Self-contained mantido (FR-003): nenhum recurso externo.
