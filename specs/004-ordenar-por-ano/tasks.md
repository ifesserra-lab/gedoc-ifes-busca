---
description: "Task list — Ordenar portarias por ano"
---

# Tasks: Ordenar portarias por ano

**Input**: Design em `/specs/004-ordenar-por-ano/` (plan.md, spec.md, research.md, data-model.md, quickstart.md)

**Tests**: INCLUÍDOS e obrigatórios — Constituição VII (TDD): teste que falha antes de implementar. Sem rede (dados em memória).

**Organization**: 1 user story (US1, P1). Mudança pontual no núcleo.

## Path Conventions

- Núcleo: `core/src/usecases/buscar.rs` (`montar_resultado` + `mod tests`).
- Efeito automático em desktop (`src-tauri`) e web (`server`) — sem edições lá.

---

## Phase 1: Setup

Sem setup — crate e testes já existem. Nada a fazer.

## Phase 2: Foundational

Sem pré-requisitos bloqueantes.

---

## Phase 3: User Story 1 - Ordenar documentos por ano (Priority: P1) 🎯 MVP

**Goal**: itens de cada categoria ordenados por ano (desc; sem data ao fim; empate estável), sem mudar agrupamento/contagem.

**Independent Test**: `montar_resultado` com documentos de anos variados → cada categoria em ordem decrescente de ano; sem-data ao fim; `qtd`/`total` inalterados.

### Tests (escrever primeiro, devem FALHAR)

- [x] T001 [US1] Testes de ordenação em `montar_resultado` (mod tests de `core/src/usecases/buscar.rs`): (a) anos variados → desc por categoria; (b) documentos sem `data` → ao final; (c) empate de ano → data completa desc, estável; (d) `qtd`/`total` e agrupamento inalterados.

### Implementation

- [x] T002 [US1] Helper de extração do ano a partir de `Documento.data` (`DD/MM/AAAA`) → `Option<u16>` (ausente/ilegível = `None`), em `core/src/usecases/buscar.rs`.
- [x] T003 [US1] Ordenar os `itens` de cada `CategoriaGrupo` em `montar_resultado`: ano desc; `None` (sem ano) ao final; empate por data completa desc; ordenação **estável** (`sort_by`), preservando agrupamento e contagem — em `core/src/usecases/buscar.rs`.
- [x] T004 [US1] `cargo test --manifest-path core/Cargo.toml` verde (ordenação + regressão dos testes existentes de `montar_resultado`).

**Checkpoint**: busca (desktop e web) devolve documentos ordenados por ano.

---

## Phase 4: Polish & Cross-Cutting

- [x] T005 [P] `cargo fmt` + `cargo clippy --all-targets -- -D warnings` no `core`.
- [ ] T006 Validar `quickstart.md` (busca `1998547`/`1545450` → anos em ordem decrescente por categoria; sem-data ao fim).

---

## Dependencies & Execution Order

- T001 (teste, falha) → T002 (helper) → T003 (ordenação) → T004 (verde).
- T005/T006 após US1.
- Sem dependências de `src-tauri`/`server`/`app` (herdam via `ResultadoView`).

## Parallel Opportunities

- T005 (`[P]`) independente após a implementação.
- (Feature pequena; pouco paralelismo real.)

## Implementation Strategy

**MVP = US1 inteira** (T001–T004). T005/T006 são acabamento. Entrega única e pequena; validar com `cargo test` do núcleo e um smoke da busca.

## Notes

- Teste antes da implementação (VII); verificar que T001 falha antes de T002/T003.
- Issue-first (X): rodar `/speckit-taskstoissues` antes de implementar, se desejado.
- Só apresentação: não tocar coleta/filtro/classificação/resumo.
