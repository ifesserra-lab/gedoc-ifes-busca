# Implementation Plan: Ordenar portarias por ano

**Branch**: `004-ordenar-por-ano` | **Date**: 2026-07-08 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/004-ordenar-por-ano/spec.md`

## Summary

Ordenar os documentos de cada categoria por **ano** (decrescente, mais
recente primeiro; sem data ao fim; empate por data completa, estável). É uma
mudança de **apresentação** no núcleo puro: a ordenação entra em
`gedocs_core::usecases::buscar::montar_resultado`, que já monta os grupos por
categoria. Como desktop (Tauri) e web (axum) consomem a MESMA `ResultadoView`
desse use-case, a ordem passa a valer nos dois sem tocar comando/handler/UI.
Sem mudança de contrato (mesma forma de `ResultadoView`).

## Technical Context

**Language/Version**: Rust 1.80+ (núcleo `gedocs-core`).

**Primary Dependencies**: nenhuma nova — só a lib padrão (parse do ano a
partir do campo `data`, `DD/MM/AAAA`).

**Storage**: N/A (ordenação em memória, na montagem do resultado).

**Testing**: `cargo test` no `gedocs-core` (testes de `montar_resultado`,
sem rede — Princípio VII).

**Target Platform**: núcleo compartilhado → efeito automático em desktop e web.

**Project Type**: alteração de núcleo (use-case puro).

**Performance Goals**: desprezível (ordena listas pequenas por busca).

**Constraints**: não alterar coleta/filtro/classificação/resumo, agrupamento
por categoria, nem a contagem (`qtd`/`total`) — só a ordem de exibição.

**Scale/Scope**: 1 função (`montar_resultado`) + testes. ~0 novos arquivos.

## Constitution Check

*GATE: passar antes da Fase 0; reavaliar após a Fase 1.*

| Princípio | Status | Nota |
|---|---|---|
| I. Fidelidade à fonte | PASS | ordena o que veio da fonte; não inventa/omite dado. |
| II. Privacidade/LGPD | PASS | não muda o que é coletado/exposto. |
| III. Reprodutibilidade | PASS | ordenação determinística e estável. |
| IV. Config sobre código | PASS | direção default (desc) documentada; toggle fica fora de escopo. |
| V. Camadas/DRY | PASS | ordena no núcleo, um só lugar → desktop+web herdam. |
| VI. Orientação a objetos | PASS | regra de apresentação no use-case do domínio. |
| VII. TDD (NN) | PASS | teste de ordenação antes da implementação. |
| VIII. Código pequeno | PASS | um comparador + `sort_by` estável. |
| IX. Padrões | PASS | sem novo padrão; só ordenação. |
| X. Issue-first (NN) | PASS (processo) | tasks viram issues antes de implementar. |
| XI. Agentes | PASS (processo) | Rust → `tauri-mvc-expert` se delegado. |
| XII. UI/UX | PASS | melhora a leitura da lista; sem mudança estrutural de tela. |

**Gate**: PASS. Sem violações → sem Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/004-ordenar-por-ano/
├── plan.md              # este arquivo
├── research.md          # Fase 0 (decisão de ordenação)
├── data-model.md        # Fase 1 (chave de ordenação)
├── quickstart.md        # Fase 1 (como validar)
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks (depois)
```

### Source Code (repository root)

```text
core/src/
├── usecases/buscar.rs   # ALTERAR: ordenar itens por ano em montar_resultado
└── domain/              # (possível) helper de extração de ano, se ficar mais limpo
core/tests|src           # testes de montar_resultado (ordenação)
```

**Structure Decision**: alteração pontual em
`gedocs_core::usecases::buscar::montar_resultado` (onde os itens de cada
`CategoriaGrupo` são montados). A extração do ano a partir de `Documento.data`
(`DD/MM/AAAA`) pode virar um helper pequeno (em `domain` ou local ao
use-case) para ficar testável isoladamente. Nada muda em `src-tauri`,
`server` ou `app` — todos já consomem a `ResultadoView` resultante.

## Complexity Tracking

Sem violações de constituição — seção não aplicável.
