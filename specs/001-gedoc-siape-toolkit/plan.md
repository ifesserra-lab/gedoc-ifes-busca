# Implementation Plan: GeDoc IFES Toolkit — Consulta por SIAPE

**Branch**: `001-gedoc-siape-toolkit` | **Date**: 2026-07-04 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-gedoc-siape-toolkit/spec.md`

## Summary

Aplicativo desktop que, a partir de uma matrícula SIAPE, busca no portal GeDoc,
filtra por presença do SIAPE, baixa os documentos, classifica por categoria
configurável, resume e gera relatório — tudo por uma UI. A stack alvo (constituição
v1.2.0) é **Tauri 2.0 (Rust) + Vue 3 + Pinia** em arquitetura **MVC**, com
**TDD**, OO, código pequeno e padrões de projeto. A pipeline de domínio
(`buscar → categorizar → resumir → pdf`) e as regras R1–R10 vêm de
[docs/ontology.yaml](../../docs/ontology.yaml). A implementação Python atual em
`src/` é referência/legado a ser migrada.

## Technical Context

**Language/Version**: Rust 1.80+ (backend Tauri) · TypeScript 5 / Vue 3.5 (frontend)

**Primary Dependencies**: Tauri 2.0; plugins oficiais `tauri-plugin-http`
(acesso ao portal), `tauri-plugin-fs`, `tauri-plugin-dialog`; Vue 3 + Pinia +
Vue Router; serviço externo de IA (classificação/resumo) via HTTP.

**Storage**: sistema de arquivos local — `data/` (resultados, PDFs, resumos,
caches) e `config/` (categorias, credenciais). Sem banco relacional.

**Testing**: `cargo test` + `cargo nextest` (Rust, unit/integração) · Vitest +
Vue Test Utils (frontend) · dublês para portal/IA (testes sem rede — Princípio VII).

**Target Platform**: Desktop (macOS, Windows, Linux) via Tauri.

**Project Type**: desktop-app (frontend Vue + backend Rust).

**Performance Goals**: busca de SIAPE já em cache retorna em < 3 s; UI responsiva
(sem congelar durante a pipeline — trabalho assíncrono no Rust).

**Constraints**: offline após coleta (PDFs locais); respeitar rate limit do
serviço de IA (throttle + retry); nenhuma PII versionada (Princípio II).

**Scale/Scope**: ordem de dezenas a ~150 documentos por SIAPE; 8 user stories;
2 telas (busca + categorias).

## Constitution Check

*GATE: deve passar antes da Fase 0. Reavaliar após a Fase 1.*

| Princípio | Como o plano atende | Status |
| --- | --- | --- |
| I Fidelidade à fonte | IDs JSF descobertos em runtime; dados derivam do conteúdo. | ✅ |
| II Privacidade/LGPD (NN) | `data/` e `config/.env` fora do VCS; sem PII em artefatos. | ✅ |
| III Reprodutibilidade/cache | Cache por `link` (classificação e resumo). | ✅ |
| IV Config sobre código | Categorias em `config/categoria.json`. | ✅ |
| V Camadas e DRY | MVC: Model/Controller (Rust), View (Vue), estado (Pinia). | ✅ |
| VI Orientação a Objetos | Entidades da ontologia → structs/serviços coesos. | ✅ |
| VII TDD (NN) | Testes antes; R1–R10 cobertas; sem rede nos testes. | ✅ |
| VIII Código pequeno | Comandos/serviços curtos; um propósito. | ✅ |
| IX Padrões de projeto | MVC + Repository (fonte GeDoc) + Strategy (keyword/LLM). | ✅ |
| X Issue-first (NN) | Tasks viram issues (`/speckit-taskstoissues`) antes do código. | ✅ |
| XI Agentes | `tauri-mvc-expert` (impl.) e `pr-reviewer` (review). | ✅ |

**Resultado**: PASS — sem violações a justificar.

## Project Structure

### Documentation (this feature)

```text
specs/001-gedoc-siape-toolkit/
├── plan.md              # este arquivo
├── research.md          # Fase 0
├── data-model.md        # Fase 1
├── quickstart.md        # Fase 1
├── contracts/           # Fase 1 (comandos IPC)
│   └── ipc-commands.md
└── tasks.md             # Fase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
src-tauri/                     # Backend Rust (Model + Controller)
├── src/
│   ├── lib.rs                 # tauri::Builder, registro de comandos/plugins
│   ├── main.rs
│   ├── commands/              # Controllers (#[tauri::command]) — fronteira IPC
│   ├── domain/                # Model: Servidor, Documento, Categoria, ...
│   ├── services/              # busca, categorizacao, resumo, pdf, cache
│   ├── ports/                 # traits (Repository GeDoc, LLM) — Strategy/DIP
│   └── error.rs               # AppError (thiserror) serializavel
├── capabilities/              # permissions por janela
├── tests/                     # cargo test (integracao)
└── Cargo.toml

src/                           # Frontend Vue (View + ViewModel)
├── views/                     # telas: Busca, Categorias
├── components/
├── stores/                    # Pinia (estado / ViewModel)
├── services/                  # wrappers tipados de invoke()
└── router/

tests/                         # Vitest (frontend)

# Legado (Python) — referencia, sera migrado:
#   src/*.py, prototipo/*.html
```

**Structure Decision**: desktop-app Tauri (Option 2 adaptada): backend Rust em
`src-tauri/` (Model/Controller) e frontend Vue em `src/` (View/estado). A
separação de camadas respeita o Princípio V e a MVC do Princípio IX.

## Complexity Tracking

> Sem violações da Constitution Check — seção não aplicável.
