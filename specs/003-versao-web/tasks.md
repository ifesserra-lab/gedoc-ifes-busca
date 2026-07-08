---
description: "Task list — Versão Web (uso interno, sem login, sessão efêmera + TTL)"
---

# Tasks: Versão Web (uso interno, sem login, sessão efêmera + TTL)

**Input**: Design em `/specs/003-versao-web/` (plan.md, spec.md, research.md, data-model.md, contracts/http-api.md, quickstart.md)

**Tests**: INCLUÍDOS e obrigatórios — Constituição, Princípio VII (TDD, NON-NEGOTIABLE): escrever teste que falha antes de implementar. Sem rede real (dublês de porta).

**Organization**: por user story (spec.md), em ordem de prioridade.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: pode rodar em paralelo (arquivos diferentes, sem dependência).
- **[Story]**: US1..US7 do spec.md.

## Path Conventions

- API (novo crate): `server/` (Cargo.toml, `src/`, `tests/`).
- Núcleo reusado (sem mudança na v1): `src-tauri/` (`gedocs_lib`).
- Frontend (existente): `app/` (`src/services/ipc.ts`, `tests/`).

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: inicializar o crate da API e a config de deploy/frontend.

- [x] T001 Criar o crate `server/` com `server/Cargo.toml` (path-dep `gedocs_lib = { path = "../src-tauri" }`; deps: `axum`, `tokio`, `tower-http` [cors, limit], `serde`, `serde_json`, `tower_governor`, `tracing`, `tracing-subscriber`)
- [x] T002 [P] Criar `server/src/main.rs` mínimo (bootstrap axum, bind de `GEDOCS_BIND`/`0.0.0.0:8787`)
- [x] T003 [P] Criar `app/.env.example` com `VITE_API_URL`
- [x] T004 [P] Criar `vercel.json` na raiz (installCommand/buildCommand em `app/`, `outputDirectory: app/dist`, rewrite SPA → `/index.html`)
- [x] T005 [P] Adicionar scaffolding de transporte web em `app/src/services/ipc.ts` (`isTauri()` via `'__TAURI_INTERNALS__' in window`, helper `apiFetch()` com `credentials: 'include'`, `API_BASE` de `VITE_API_URL`) — sem alterar comportamento desktop

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: infraestrutura que TODAS as stories usam.

**⚠️ CRITICAL**: nenhuma user story começa antes desta fase.

- [x] T006 Implementar `AppState` (data_dir de `GEDOCS_DATA_DIR`, `session_ttl` de `GEDOCS_SESSION_TTL`, `cors_origin` de `GEDOCS_CORS_ORIGIN`, `seed_categorias`) em `server/src/main.rs`
- [x] T007 [P] Implementar mapeamento `AppError` → resposta HTTP (`IntoResponse`, corpo `{tipo,mensagem}`, tabela de status do contrato) em `server/src/erro.rs`
- [x] T008 [P] Endpoint `GET /api/health` (`{ok:true}`) em `server/src/rotas.rs`
- [x] T009 Camada CORS restrita à origem (`cors_origin`) com credenciais habilitadas em `server/src/main.rs` (depende de T006)
- [x] T010 [P] Camadas de rate limit (`tower_governor`) + limite de tamanho de corpo (`RequestBodyLimitLayer`) em `server/src/main.rs` (FR-016, SC-007)
- [x] T011 [P] Harness de teste de integração (subir app em memória, dublês de `GedocRepository`/`ChatIa`/`HttpPort`) em `server/tests/comum.rs`

**Checkpoint**: base pronta — user stories podem começar.

---

## Phase 3: User Story 1 - Buscar documentos por SIAPE (Priority: P1) 🎯 MVP

**Goal**: buscar por SIAPE no navegador e ver documentos agrupados.

**Independent Test**: `POST /api/buscar` com SIAPE válido retorna `ResultadoView`; SIAPE inválido → 400 amigável; portal fora → 502.

### Tests (escrever primeiro, devem FALHAR)

- [x] T012 [P] [US1] Teste de integração `POST /api/buscar` (modo keyword): sucesso agrupado, `SiapeInvalido` (400), `FalhaPortal` (502) com dublê de repo em `server/tests/buscar.rs`

### Implementation

- [x] T013 [US1] Implementar handler `POST /api/buscar` chamando `gedocs_lib::commands::buscar::executar` (keyword) em `server/src/rotas.rs`
- [x] T014 [US1] Branch web de `buscarPorSiape` (fetch `/api/buscar`) em `app/src/services/ipc.ts` + teste em `app/tests/ipc.spec.ts`

**Checkpoint**: busca funciona ponta-a-ponta no navegador (MVP).

---

## Phase 4: User Story 2 - Acesso sem login com sessão efêmera e privacidade (Priority: P1)

**Goal**: sessão automática sem login; PDFs isolados por sessão e apagados por TTL (LGPD).

**Independent Test**: cookie `gedocs_sid` emitido no 1º acesso; 2 sessões não acessam arquivos uma da outra (SC-002); dir some após TTL (SC-003).

### Tests (escrever primeiro, devem FALHAR)

- [x] T015 [P] [US2] Teste: `gedocs_sid` emitido no 1º acesso e isolamento entre 2 sessões (SC-002, FR-010/011) em `server/tests/sessao.rs`
- [x] T016 [P] [US2] Teste: sessão inativa além do TTL tem o dir removido (SC-003, FR-012) em `server/tests/ttl.rs`

### Implementation

- [x] T017 [US2] Middleware de sessão (cookie `gedocs_sid` opaco, `HttpOnly`/`SameSite=Lax`/`Secure` em prod) + resolução do dir `<data>/sessions/<sid>/` com sanitização em `server/src/sessao.rs`
- [x] T018 [US2] Tarefa em background de limpeza por TTL (varre dirs de sessão por última atividade) em `server/src/ttl.rs`
- [x] T019 [US2] Ligar middleware + tarefa TTL ao app em `server/src/main.rs` (depende de T017, T018)

**Checkpoint**: privacidade por sessão garantida antes de qualquer download.

---

## Phase 5: User Story 3 - Abrir e baixar o PDF (Priority: P1)

**Goal**: baixar o PDF para a sessão e abri-lo no navegador.

**Independent Test**: `POST /api/documento/baixar` grava na sessão e retorna `{arquivo}`; `GET /api/documento/:siape/:arquivo` serve o PDF; outra sessão recebe 404.

### Tests (escrever primeiro, devem FALHAR)

- [x] T020 [P] [US3] Teste `POST /api/documento/baixar` + `GET /api/documento/:siape/:arquivo` (isolado por sessão; 404 cross-session) em `server/tests/documento.rs`

### Implementation

- [x] T021 [US3] Handler `POST /api/documento/baixar` (`executar_download` no dir da sessão) em `server/src/rotas.rs`
- [x] T022 [US3] Handler `GET /api/documento/:siape/:arquivo` (`resolver_caminho_abertura` + stream PDF inline) em `server/src/rotas.rs`
- [x] T023 [US3] Branches web de `baixarDocumento` (fetch) e `abrirDocumento` (`window.open`) em `app/src/services/ipc.ts` + teste em `app/tests/ipc.spec.ts`

**Checkpoint**: fluxo buscar → baixar → abrir completo no navegador.

---

## Phase 6: User Story 4 - Classificação e resumo por IA (Priority: P2)

**Goal**: modo IA opcional com degradação segura para palavra-chave.

**Independent Test**: com dublê `ChatIa`, resultados vêm resumidos; sem chave, cai para keyword sem falhar (FR-004/005).

### Tests (escrever primeiro, devem FALHAR)

- [ ] T024 [P] [US4] Teste `POST /api/buscar` modo llm: com `ChatIa` dublê resume; sem chave degrada para keyword sem 5xx em `server/tests/buscar_ia.rs` — PENDENTE: o handler HTTP usa `executar` (constrói a infra internamente), sem ponto de injeção de `ChatIa`; a degradação segura já é coberta pelos testes de `gedocs_lib` (`commands::buscar`). Exige extrair um ponto de injeção para testar via HTTP.

> Nota: os testes de integração foram consolidados em `server/tests/api.rs` (em vez de um arquivo por story). Cobertura: health, buscar (SIAPE inválido), sessão/isolamento (SC-002), TTL (SC-003), categorias CRUD+validação, relatório, zip vazio, rate limit.

### Implementation

- [x] T025 [US4] Estender handler de busca: modo llm resolve chave server-side, cache por sessão (`<sid>/cache`), degradação segura em `server/src/rotas.rs`
- [x] T026 [US4] Garantir passthrough do `modo` (keyword|llm) em `app/src/services/ipc.ts` + teste em `app/tests/ipc.spec.ts`

**Checkpoint**: IA opcional funcionando; chave nunca no cliente (SC-004).

---

## Phase 7: User Story 5 - Gerar relatório consolidado (Priority: P2)

**Goal**: gerar o relatório HTML da busca atual e abrir no navegador.

**Independent Test**: `POST /api/relatorio` gera na sessão; `GET /api/relatorio/:siape` serve o HTML.

### Tests (escrever primeiro, devem FALHAR)

- [x] T027 [P] [US5] Teste `POST /api/relatorio` + `GET /api/relatorio/:siape` (HTML gerado e servido) em `server/tests/relatorio.rs`

### Implementation

- [x] T028 [US5] Handler `POST /api/relatorio` (`executar_gerar_relatorio` no dir da sessão) em `server/src/rotas.rs`
- [x] T029 [US5] Handler `GET /api/relatorio/:siape` (serve `text/html`) em `server/src/rotas.rs`
- [x] T030 [US5] Branch web de `gerarRelatorio` (POST + `window.open` do GET) em `app/src/services/ipc.ts` + teste em `app/tests/ipc.spec.ts`

**Checkpoint**: relatório abrível/imprimível no navegador.

---

## Phase 8: User Story 6 - Baixar ZIP dos PDFs da sessão (Priority: P2)

**Goal**: empacotar os PDFs baixados na sessão em um ZIP.

**Independent Test**: com PDFs na sessão, `GET /api/zip/:siape` baixa o `.zip`; sem PDFs, erro amigável (FR-007).

### Tests (escrever primeiro, devem FALHAR)

- [x] T031 [P] [US6] Teste `GET /api/zip/:siape`: com PDFs baixa; sem PDFs erro amigável em `server/tests/zip.rs`

### Implementation

- [x] T032 [US6] Handler `GET /api/zip/:siape` (`montar_zip` do dir da sessão; stream `attachment`) em `server/src/rotas.rs`
- [x] T033 [US6] Branch web de `baixarZip` (fetch blob + download) em `app/src/services/ipc.ts` + teste em `app/tests/ipc.spec.ts`

**Checkpoint**: pacote ZIP da sessão baixável.

---

## Phase 9: User Story 7 - Gerenciar categorias (Priority: P3)

**Goal**: CRUD de categorias globais.

**Independent Test**: `GET /api/categorias` lista; `PUT /api/categorias` grava; nome vazio/duplicado é rejeitado; última gravação vence (FR-008/017).

### Tests (escrever primeiro, devem FALHAR)

- [x] T034 [P] [US7] Teste `GET`/`PUT /api/categorias`: lista global, `CategoriaSemNome`/`NomeDuplicado` (400), last-write-wins em `server/tests/categorias.rs`

### Implementation

- [x] T035 [US7] Handlers `GET /api/categorias` + `PUT /api/categorias` (`services::categorias` sobre `<data>/categoria.json` global, semeado de `config/categoria.json`) em `server/src/rotas.rs`
- [x] T036 [US7] Branches web de `listarCategorias` + `salvarCategorias` em `app/src/services/ipc.ts` + teste em `app/tests/ipc.spec.ts`

**Checkpoint**: todas as ações do desktop disponíveis na web (SC-006).

---

## Phase 10: Polish & Cross-Cutting Concerns

- [x] T037 [P] Criar `server/Dockerfile` (multi-stage Rust; runtime com libs necessárias — opção A1) e `server/.dockerignore`
- [x] T038 [P] Criar `server/fly.toml` (exemplo de deploy; envs `GEDOCS_*` e secret `MISTRAL_API_KEY`)
- [x] T039 [P] Atualizar `docs/plano-web.md` com status e links de `specs/003-versao-web/`
- [x] T040 Hardening (Princípio II): confirmar zero PII em logs, cookie `Secure` em prod, origem CORS só via env
- [x] T041 Rodar a validação do `quickstart.md` (cenários 1–10)
- [x] T042 [P] `cargo fmt`/`clippy` em `server/` e `eslint`/`vue-tsc` em `app/`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: sem dependências.
- **Foundational (Phase 2)**: depende do Setup — BLOQUEIA todas as stories.
- **US1 (Phase 3)**: só depende da Foundational (busca keyword não usa sessão).
- **US2 (Phase 4)**: depende da Foundational — **pré-requisito de US3/US5/US6** (dir de sessão).
- **US3 (Phase 5)**: depende de US2.
- **US4 (Phase 6)**: depende de US1 (estende o handler de busca) + US2 (cache por sessão).
- **US5 (Phase 7)** e **US6 (Phase 8)**: dependem de US2 (dir de sessão). US6 pressupõe downloads (US3) para ter o que empacotar.
- **US7 (Phase 9)**: só depende da Foundational (config global, sem sessão).
- **Polish (Phase 10)**: depois das stories desejadas.

### Within Each User Story

- Teste escrito e FALHANDO antes da implementação (Princípio VII).
- Handler (server) antes do branch web (`ipc.ts`).

### Parallel Opportunities

- Setup: T002/T003/T004/T005 em paralelo.
- Foundational: T007/T008/T010/T011 em paralelo (T006 antes de T009).
- Após a Foundational: **US1 e US7 podem correr em paralelo** (não usam sessão); US2 desbloqueia US3/US5/US6.
- Cada teste `[P]` de story é independente (arquivos de teste distintos).

---

## Parallel Example: User Story 1

```bash
# Teste primeiro (deve falhar):
Task: "T012 Teste de integração POST /api/buscar em server/tests/buscar.rs"
# Depois implementar:
Task: "T013 Handler POST /api/buscar em server/src/rotas.rs"
Task: "T014 Branch web buscarPorSiape em app/src/services/ipc.ts"
```

---

## Implementation Strategy

### MVP (User Story 1)

1. Phase 1 Setup → 2. Phase 2 Foundational → 3. Phase 3 US1 → **validar busca no navegador** → deploy/demo.

### Entrega incremental

Setup+Foundational → US1 (MVP) → US2 (privacidade) → US3 (PDF) → US4 (IA) → US5 (relatório) → US6 (ZIP) → US7 (categorias) → Polish. Cada story testável e entregável isoladamente.

### Escopo MVP sugerido

**US1** (buscar por SIAPE). Para um MVP "útil de verdade" no navegador com privacidade, agrupar **US1 + US2 + US3** (todos P1).

---

## Notes

- `[P]` = arquivos diferentes, sem dependência.
- Cada story é independentemente testável (arquivos de teste separados por story).
- Verificar que os testes falham antes de implementar (Princípio VII).
- Issue-first (Princípio X): rodar `/speckit-taskstoissues` antes de implementar.
- Commit por task ou grupo lógico, referenciando a issue.
