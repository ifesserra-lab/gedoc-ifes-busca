---
description: "Task list — GeDoc IFES Toolkit (Tauri 2.0 + Vue), TDD por user story"
---

# Tasks: GeDoc IFES Toolkit — Consulta por SIAPE

**Input**: `specs/001-gedoc-siape-toolkit/` (plan.md, spec.md, data-model.md, contracts/, research.md)

**Tests**: INCLUÍDOS — a constituição v1.3.0 (Princípio VII) exige TDD.

**Organização**: por user story (US1–US8), cada uma testável e entregável de
forma independente. Backend Rust em `src-tauri/`, frontend Vue em `app/`.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: paralelizável (arquivos distintos, sem dependência pendente).
- **[US#]**: user story do spec.md.

---

## Phase 1: Setup (infra compartilhada)

- [x] T001 Inicializar app Tauri 2.0 + Vue 3 (TS): backend `src-tauri/`, frontend `app/` (`app/package.json`)
- [x] T002 [P] Configurar Pinia e Vue Router em `app/src/main.ts` + `app/src/router/index.ts`
- [~] T003 [P] Lint/format: rustfmt+clippy (`-D warnings`) ✅; eslint/prettier no front ainda pendente
- [x] T004 [P] Testes: `cargo test` (Rust) e Vitest (`app/`) configurados (nextest opcional, não adotado)
- [~] T005 Definir `capabilities/` do Tauri em `src-tauri/capabilities/` — opener adicionado em #4; http/fs não usados (reqwest direto)

## Phase 2: Foundational (bloqueia todas as US)

- [x] T006 Modelar entidades de domínio em `src-tauri/src/domain/` (Servidor/siape, Documento, Categoria, texto, nome_arquivo)
- [x] T007 [P] Definir `AppError` (thiserror, serializável) em `src-tauri/src/error.rs`
- [~] T008 [P] Traits/ports em `src-tauri/src/ports/` — GedocRepository, Classificador, HttpPort, `ports::ia::ChatIa` (US5) ✅; Resumidor pendente (US6)
- [x] T009 `CacheArquivo` genérico por link em `src-tauri/src/services/cache.rs` (R6) — feito em US5 (usado pela classificação `llm`); reutilizável tal como está por US6 (resumo), com outro arquivo
- [x] T010 Registrar `tauri::Builder`, plugins e `invoke_handler` em `src-tauri/src/lib.rs`
- [x] T011 [P] Camada de serviços IPC no front: `app/src/services/ipc.ts` (wrappers tipados de `invoke`)

**⚠️ Concluir Fase 2 antes de qualquer US.**

---

## Phase 3: US1 — Buscar e coletar (P1) 🎯 MVP

**Meta**: dado um SIAPE, coletar todos os documentos (paginação completa).
**Teste independente**: total coletado == total do portal.

- [x] T012 [P] [US1] Teste: descoberta dinâmica de IDs/ViewState (R8) — como testes unitários em `gedoc_repository.rs` (`descobre_ids_do_fixture_home`) + fixture `tests/fixtures/home_pesquisa.html`
- [x] T013 [P] [US1] Teste: paginação coleta todos os registros (portal dublado `FakeHttp`) — `buscar_agrega_documentos_de_multiplas_paginas` em `gedoc_repository.rs`
- [x] T014 [US1] Implementar `GedocRepository` (sessão, ViewState, IDs dinâmicos, busca AJAX) em `src-tauri/src/services/gedoc_repository.rs` (R8) + adapter `src-tauri/src/ports/http.rs`
- [x] T015 [US1] Implementar paginação + dedup por link em `gedoc_repository.rs`
- [~] T016 [US1] ~~Persistir `ResultadoBusca` em `data/resultado_<siape>.json`~~ — **fora de escopo na arquitetura Tauri/IPC**: `buscar_por_siape` devolve `ResultadoView` direto à View; não há necessidade de persistir em disco no MVP (evita gravar PII — Princípio II). Reabrir se surgir requisito de cache/offline.
- [x] T017 [US1] Comando `buscar_por_siape` (coleta + filtro + agrupamento) em `src-tauri/src/commands/buscar.rs`

## Phase 4: US2 — Filtrar por SIAPE (P1)

**Meta**: manter só documentos que citam o SIAPE no texto.
**Teste independente**: itens listados contêm o SIAPE; demais em descartados.

- [x] T018 [P] [US2] Teste: validação `^[0-9]{5,8}$` em `src-tauri/tests/siape_valido.rs` (R10)
- [x] T019 [P] [US2] Teste: filtro marca contem_siape pelo trecho em `src-tauri/tests/filtro_siape.rs` (R2)
- [x] T020 [US2] Validador de SIAPE em `src-tauri/src/domain/siape.rs` (R10)
- [x] T021 [US2] Extração de `siapes[]` (parser) e filtro `contem_siape` em `services/filtro.rs` (R2, fronteira digit-boundary vs falso-positivo — issue #2)
- [x] T022 [US2] Expor válidos × descartados: `filtro::separar` + `montar_resultado` só agrupa válidos

## Phase 5: US3 — Buscar pelo navegador (P1)

**Meta**: tela web para SIAPE → resultados.
**Teste independente**: SIAPE inválido bloqueia; válido mostra total+lista.

- [x] T023 [P] [US3] Teste de componente: validação do campo SIAPE em `app/tests/BuscaView.spec.ts` (R10) + `siape.spec.ts` + `busca_store.spec.ts`
- [x] T024 [US3] `app/src/stores/busca.ts` (Pinia): estado idle/loading/erro/resultado
- [x] T025 [US3] `app/src/views/BuscaView.vue`: campo SIAPE, `ipc.buscarPorSiape`, chips + lista agrupada
- [x] T026 [US3] Roteamento `/` → BuscaView em `app/src/router/index.ts`

## Phase 6: US4 — Baixar documentos organizados (P2)

**Meta**: baixar PDFs `AAAA_NUMERO_ASSUNTO`, sem sobrescrever.
**Teste independente**: PDFs `%PDF` válidos; colisão recebe sufixo.

- [x] T027 [P] [US4] Teste: nome determinístico + colisão em `src-tauri/tests/nome_arquivo.rs` (R3)
- [x] T028 [US4] Derivar nome `AAAA_NUMERO_ASSUNTO` em `src-tauri/src/domain/nome_arquivo.rs` (R3)
- [x] T029 [US4] Download dos PDFs (extração de texto p/ etapas seguintes) em `src-tauri/src/services/downloader.rs`
- [x] T030 [US4] Comando `abrir_documento` (nome sanitizado) em `src-tauri/src/commands/documento.rs` (R7)
- [x] T031 [P] [US4] Botão "PDF" por documento em `app/src/components/busca/DocItem.vue` (baixar+abrir via IPC)

## Phase 7: US5 — Classificar por categoria (P2)

**Meta**: uma categoria por documento (keyword|llm via config).
**Teste independente**: cada doc com 1 categoria; soma = total.

- [x] T032 [P] [US5] Teste: Strategy keyword e fallback "Outros" em `src-tauri/tests/classificar.rs` (R4)
- [x] T033 [P] [US5] Teste: LLM fora da lista → "Outros"; cache por link em `src-tauri/tests/classificar_llm.rs` (R4,R6)
- [x] T034 [US5] `Classificador` (Strategy keyword/llm) — `ClassificadorPalavraChave`/`ClassificadorLlm` (impl do trait) ficaram em `src-tauri/src/ports/classificador.rs`, junto do trait já existente; a orquestração que escolhe o modo e liga cache/fallback (`ModoClassificacao`, `classificar_lote`) foi para `src-tauri/src/services/classificador.rs` — é essa função que `commands::buscar` chama (R4,R5,R6,R11)
- [x] T035 [US5] Carregar `config/categoria.json` em `src-tauri/src/services/categorias.rs` (R5) — inclui `caminho_padrao()` (candidatos relativos, análogo à resolução do `.env`); CRUD de escrita continua TODO de US8
- [x] T036 [US5] Throttle + retry (429/5xx) no adapter de IA — ficou em `src-tauri/src/ports/ia.rs` (não `services/ia_client.rs`): é uma fronteira de I/O (Port/Adapter), mesmo padrão de `ports::http::ReqwestHttp`. Inclui `ChatIa` (trait), `MistralClient` (adapter, throttle 1.2s + retry 2^tentativa até 4 tentativas) e a leitura da chave (`resolver_api_key`: env `MISTRAL_API_KEY`/`MISTRAL_KEY` ou `config/.env`/`.env`) (R9)
- [x] `commands::buscar`: fiado — `buscar_por_siape` classifica cada documento válido antes de agrupar; modo default `keyword` (instantâneo, sem API); modo `llm` só ativa com `input.modo == "llm"` **e** chave configurada, senão degrada para `keyword` (R11). `montar_resultado` agora agrupa por `doc.categoria`, na ordem de `config/categoria.json`, omitindo grupos vazios

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
- [ ] T045 [P] [US7] Botões "PDF do resumo" e "Baixar todos" em `app/src/views/BuscaView.vue`

## Phase 10: US8 — CRUD de categorias (P3)

**Meta**: cadastrar/editar/remover categorias (persistem em config).
**Teste independente**: criar persiste; nome duplicado rejeitado; remover funciona.

- [ ] T046 [P] [US8] Teste: salvar rejeita nome vazio/duplicado em `src-tauri/tests/categorias.rs` (R5)
- [ ] T047 [P] [US8] Teste de componente: modal CRUD em `app/tests/CategoriasView.spec.ts`
- [ ] T048 [US8] Comandos `listar_categorias`/`salvar_categorias` em `src-tauri/src/commands/categorias.rs` (R5)
- [~] T049 [US8] `app/src/views/CategoriasView.vue` (tabela + modal add/editar/remover) + rota `/categorias` — UI já entregue (#13); falta ligar ao IPC de persistência
- [~] T050 [US8] `app/src/stores/categorias.ts` (Pinia) chamando IPC — store existe (#13); trocar stub por `listar/salvar_categorias`

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
