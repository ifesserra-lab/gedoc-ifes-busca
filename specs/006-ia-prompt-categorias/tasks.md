---
description: "Task list — IA classifica pelas categorias do category.json (no prompt)"
---

# Tasks: IA classifica pelas categorias do category.json (no prompt)

**Nota**: comportamento já implementado no núcleo (ver plan.md). Escopo aqui =
teste-guarda travando FR-002/SC-002 (o prompt inclui nome+descrição). TDD (VII).

## Phase 3: US1 (P1) 🎯

- [x] T001 [US1] Teste em `core/src/ports/classificador.rs` (mod tests): chama `montar_prompt` com categorias (nome+descrição) e afirma que o prompt do usuário contém "Categorias disponíveis:" e cada `nome` + `descrição`; o sistema pede UMA da lista.
- [x] T002 [US1] (já coberto) confirmar testes existentes: fora-da-lista → OUTROS (`llm_cai_em_outros_quando_categoria_esta_fora_da_lista`), JSON inválido → OUTROS, falha IA → keyword (R11).

## Phase 4: Polish

- [x] T003 [P] `cargo fmt` + `cargo clippy --all-targets -- -D warnings` + `cargo test` no `core`.

## Notes

- Sem mudança de runtime: FR-001..006 já atendidos por `ClassificadorLlm`
  (`montar_prompt` injeta as categorias; `extrair_categoria` restringe ao
  conjunto → OUTROS). Desktop e web herdam (mesmo núcleo).
