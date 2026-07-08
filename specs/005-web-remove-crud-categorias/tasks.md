---
description: "Task list — Remover o CRUD de categorias na versão web"
---

# Tasks: Remover o CRUD de categorias na versão web

**Tests**: obrigatórios (VII). Frontend compartilhado → condicionar por `emTauri()`.

## Phase 3: US1 - Web sem gestão de categorias (P1) 🎯 MVP

- [x] T001 [US1] Exportar `emTauri()` em `app/src/services/ipc.ts` (detecção web/desktop reutilizável).
- [x] T002 [US1] `app/src/App.vue`: montar `links` como computed que exclui "Categorias" quando `!emTauri()` (web).
- [x] T003 [US1] `app/src/router/index.ts`: `beforeEnter` na rota `/categorias` → redireciona para `busca` quando `!emTauri()`.
- [x] T004 [US1] `server/src/lib.rs`: remover as rotas `GET`/`PUT /api/categorias` (e o campo `seed_categorias` do `AppState`, agora sem uso).
- [x] T005 [US1] `server/src/rotas.rs`: remover `listar_categorias`/`salvar_categorias` e imports agora sem uso (Categoria, services::categorias).
- [x] T006 [US1] `server/tests/api.rs`: remover o teste de CRUD de categorias; trocar a fonte do cookie de sessão em `documento_isolado_por_sessao_sc002` para outra rota protegida (ex.: `GET /api/relatorio/:siape`); ajustar `estado()` (sem `seed_categorias`).

## Phase 4: US2 - Desktop mantém CRUD (P1)

- [x] T007 [US2] Confirmar (sem alteração de código) que o desktop mantém a tela e os comandos Tauri `listar/salvar_categorias`: no Tauri, `emTauri()` é true → link/rota visíveis; server web não afeta o IPC.

## Phase 5: Polish

- [x] T008 [P] `cargo fmt`/`clippy -D warnings` + `cargo test` (server/core); `vitest` + `vue-tsc` (app).
- [ ] T009 Validar: web (sem `__TAURI__`) não mostra Categorias e `/categorias` redireciona; `PUT /api/categorias` some (404); busca ainda classifica. Desktop mostra Categorias e CRUD funciona.

## Notes

- Só apresentação/superfície na web; a **classificação** (núcleo) não muda.
- Desktop: nenhuma edição — comportamento preservado (US2).
