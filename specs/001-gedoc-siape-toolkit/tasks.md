---
description: "Task list — GeDoc IFES Toolkit (Tauri 2.0 + Vue), TDD por user story"
---

# Tasks: GeDoc IFES Toolkit — Consulta por SIAPE

**Input**: `specs/001-gedoc-siape-toolkit/` (plan.md, spec.md, data-model.md, contracts/, research.md)

**Tests**: INCLUÍDOS — a constituição v1.2.0 (Princípio VII) exige TDD.

**Organização**: por user story (US1–US8), cada uma testável e entregável de
forma independente. Backend Rust em `src-tauri/`, frontend Vue em `src/`.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: paralelizável (arquivos distintos, sem dependência pendente).
- **[US#]**: user story do spec.md.

---

## Phase 1: Setup (infra compartilhada)

- [ ] T001 Inicializar app Tauri 2.0 + Vue 3 (TS) na raiz (`src-tauri/`, `src/`, `package.json`)
- [ ] T002 [P] Configurar Pinia e Vue Router em `src/main.ts`
- [ ] T003 [P] Configurar lint/format (rustfmt+clippy; eslint+prettier)
- [ ] T004 [P] Configurar testes: `cargo nextest` e Vitest (`vitest.config.ts`)
- [ ] T005 Definir `capabilities/` mínimas do Tauri (http, fs, dialog) em `src-tauri/capabilities/`

## Phase 2: Foundational (bloqueia todas as US)

- [ ] T006 Modelar entidades de domínio em `src-tauri/src/domain/` (Servidor, Documento, Categoria, ResultadoBusca) conforme data-model.md
- [ ] T007 [P] Definir `AppError` (thiserror, serializável) em `src-tauri/src/error.rs`
- [ ] T008 [P] Definir traits/ports em `src-tauri/src/ports/` (GedocRepository, Classificador, Resumidor, Cache)
- [ ] T009 Implementar `Cache` em arquivos (por link) em `src-tauri/src/services/cache.rs` (R6)
- [ ] T010 Registrar `tauri::Builder`, plugins e `invoke_handler` em `src-tauri/src/lib.rs`
- [ ] T011 [P] Camada de serviços IPC no front: `src/services/ipc.ts` (wrappers tipados de `invoke`)

**⚠️ Concluir Fase 2 antes de qualquer US.**

---

## Phase 3: US1 — Buscar e coletar (P1) 🎯 MVP

**Meta**: dado um SIAPE, coletar todos os documentos (paginação completa).
**Teste independente**: total coletado == total do portal.

- [ ] T012 [P] [US1] Teste: descoberta dinâmica de IDs/ViewState em `src-tauri/tests/gedoc_ids.rs` (R8)
- [ ] T013 [P] [US1] Teste: paginação coleta todos os registros (portal dublado) em `src-tauri/tests/gedoc_paginacao.rs`
- [ ] T014 [US1] Implementar `GedocRepository` (sessão, ViewState, IDs dinâmicos, busca AJAX) em `src-tauri/src/services/gedoc_repository.rs` (R8)
- [ ] T015 [US1] Implementar paginação + dedup por link em `gedoc_repository.rs`
- [ ] T016 [US1] Persistir `ResultadoBusca` em `data/resultado_<siape>.json` em `src-tauri/src/services/resultado_store.rs`
- [ ] T017 [US1] Comando `buscar_por_siape` (parcial: só coleta) em `src-tauri/src/commands/buscar.rs`

## Phase 4: US2 — Filtrar por SIAPE (P1)

**Meta**: manter só documentos que citam o SIAPE no texto.
**Teste independente**: itens listados contêm o SIAPE; demais em descartados.

- [ ] T018 [P] [US2] Teste: validação `^[0-9]{5,8}$` em `src-tauri/tests/siape_valido.rs` (R10)
- [ ] T019 [P] [US2] Teste: filtro marca contem_siape pelo trecho em `src-tauri/tests/filtro_siape.rs` (R2)
- [ ] T020 [US2] Validador de SIAPE em `src-tauri/src/domain/siape.rs` (R10)
- [ ] T021 [US2] Extração de `siapes[]` e filtro `contem_siape` em `gedoc_repository.rs` (R2)
- [ ] T022 [US2] Expor válidos × descartados no resultado (US2 completa a busca)

## Phase 5: US3 — Buscar pelo navegador (P1)

**Meta**: tela web para SIAPE → resultados.
**Teste independente**: SIAPE inválido bloqueia; válido mostra total+lista.

- [ ] T023 [P] [US3] Teste de componente: validação do campo SIAPE em `tests/BuscaView.spec.ts` (R10)
- [ ] T024 [US3] `stores/busca.ts` (Pinia): estado busca/loading/erro/resultado
- [ ] T025 [US3] `views/BuscaView.vue`: campo SIAPE, chamada `ipc.buscarPorSiape`, chips + lista
- [ ] T026 [US3] Roteamento `/` → BuscaView em `src/router/index.ts`

## Phase 6: US4 — Baixar documentos organizados (P2)

**Meta**: baixar PDFs `AAAA_NUMERO_ASSUNTO`, sem sobrescrever.
**Teste independente**: PDFs `%PDF` válidos; colisão recebe sufixo.

- [ ] T027 [P] [US4] Teste: nome determinístico + colisão em `src-tauri/tests/nome_arquivo.rs` (R3)
- [ ] T028 [US4] Derivar nome `AAAA_NUMERO_ASSUNTO` em `src-tauri/src/domain/nome_arquivo.rs` (R3)
- [ ] T029 [US4] Download dos PDFs (extração de texto p/ etapas seguintes) em `src-tauri/src/services/downloader.rs`
- [ ] T030 [US4] Comando `abrir_documento` (nome sanitizado) em `src-tauri/src/commands/documento.rs` (R7)
- [ ] T031 [P] [US4] Botão "PDF" por documento na lista em `views/BuscaView.vue`

## Phase 7: US5 — Classificar por categoria (P2)

**Meta**: uma categoria por documento (keyword|llm via config).
**Teste independente**: cada doc com 1 categoria; soma = total.

- [ ] T032 [P] [US5] Teste: Strategy keyword e fallback "Outros" em `src-tauri/tests/classificar.rs` (R4)
- [ ] T033 [P] [US5] Teste: LLM fora da lista → "Outros"; cache por link em `src-tauri/tests/classificar_llm.rs` (R4,R6)
- [ ] T034 [US5] `Classificador` (Strategy keyword/llm) em `src-tauri/src/services/classificador.rs` (R4,R5)
- [ ] T035 [US5] Carregar `config/categoria.json` em `src-tauri/src/services/categorias.rs` (R5)
- [ ] T036 [US5] Throttle + retry (429) no adapter de IA em `src-tauri/src/services/ia_client.rs` (R9)

## Phase 8: US6 — Resumir cada documento (P2)

**Meta**: resumo fiel; falha isolada não derruba lote.
**Teste independente**: resumo deriva do texto; documento ilegível usa trecho.

- [ ] T037 [P] [US6] Teste: resumo usa texto e não inventa; fallback ao trecho em `src-tauri/tests/resumir.rs` (R1)
- [ ] T038 [P] [US6] Teste: falha em 1 doc não aborta lote em `src-tauri/tests/resumir_lote.rs` (R9)
- [ ] T039 [US6] Extração de texto do PDF em `src-tauri/src/services/texto_pdf.rs`
- [ ] T040 [US6] `Resumidor` (IA) + cache por link em `src-tauri/src/services/resumidor.rs` (R1,R6)
- [ ] T041 [US6] Completar `buscar_por_siape` retornando categorias+resumos (contracts/ipc-commands.md)

## Phase 9: US7 — Relatório e ZIP (P3)

**Meta**: PDF do resumo agrupado + ZIP dos documentos.
**Teste independente**: PDF abre; ZIP contém todos os PDFs.

- [ ] T042 [P] [US7] Teste: markdown agrupado por categoria em `src-tauri/tests/relatorio.rs`
- [ ] T043 [US7] Gerar relatório (markdown→PDF via webview) em `src-tauri/src/services/relatorio.rs`
- [ ] T044 [US7] Comando `gerar_pdf_resumo` e `baixar_zip` em `src-tauri/src/commands/exportar.rs` (R3,R7)
- [ ] T045 [P] [US7] Botões "PDF do resumo" e "Baixar todos" em `views/BuscaView.vue`

## Phase 10: US8 — CRUD de categorias (P3)

**Meta**: cadastrar/editar/remover categorias (persistem em config).
**Teste independente**: criar persiste; nome duplicado rejeitado; remover funciona.

- [ ] T046 [P] [US8] Teste: salvar rejeita nome vazio/duplicado em `src-tauri/tests/categorias.rs` (R5)
- [ ] T047 [P] [US8] Teste de componente: modal CRUD em `tests/CategoriasView.spec.ts`
- [ ] T048 [US8] Comandos `listar_categorias`/`salvar_categorias` em `src-tauri/src/commands/categorias.rs` (R5)
- [ ] T049 [US8] `views/CategoriasView.vue` (tabela + modal add/editar/remover) + rota `/categorias`
- [ ] T050 [US8] `stores/categorias.ts` (Pinia) chamando IPC

## Phase 11: Polish & cross-cutting

- [ ] T051 [P] Garantir `.gitignore`: `data/`, `config/.env` fora do VCS (R7)
- [ ] T052 [P] Erros amigáveis na UI (sem stack trace) e estados vazios
- [ ] T053 [P] Rodar `quickstart.md` de ponta a ponta com um SIAPE real (validação)
- [ ] T054 Revisão via agente `pr-reviewer` antes do PR (Princípios VII, X)

---

## Dependências (ordem das stories)

- **Setup (F1) → Foundational (F2)** bloqueiam tudo.
- **US1 → US2** (US2 refina a coleta). US1 é o MVP.
- **US3** depende de US1/US2 (consome a busca).
- **US4, US5, US6** dependem de US1 (documentos); independentes entre si [P].
- **US7** depende de US5+US6 (relatório com categoria+resumo).
- **US8** independente (só config); pode ir em paralelo após F2.

## Paralelização (exemplos)
- F2: T007, T008, T011 em paralelo.
- US1: T012, T013 (testes) em paralelo antes de T014.
- Após US1: US4, US5, US6, US8 podem avançar em paralelo por times distintos.

## MVP
**US1 + US2 + US3** (P1): buscar por SIAPE no navegador, coletado e filtrado —
já entrega valor. Demais US são incrementos.

## Total
54 tasks · US1:6 · US2:5 · US3:4 · US4:5 · US5:5 · US6:5 · US7:4 · US8:5 ·
Setup:5 · Foundational:6 · Polish:4.
