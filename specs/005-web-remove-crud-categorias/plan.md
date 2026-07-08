# Implementation Plan: Remover o CRUD de categorias na versão web

**Branch**: `005-web-remove-crud-categorias` | **Date**: 2026-07-08 | **Spec**: [spec.md](./spec.md)

## Summary

Na web, remover a gestão de categorias: esconder o link/rota da tela de
categorias e remover os endpoints de leitura/escrita de categorias da API.
A classificação segue usando as categorias globais do servidor (a busca lê o
arquivo `categoria.json` internamente, não os endpoints). O **desktop**
mantém o CRUD (comandos Tauri `listar/salvar_categorias` + tela) — intacto.

## Technical Context

**Language/Version**: TS/Vue 3 (frontend), Rust (server axum).
**Primary Dependencies**: existentes. **Storage**: N/A.
**Testing**: vitest + vue-tsc (frontend), `cargo test`/clippy (server).
**Project Type**: web (frontend + API); desktop compartilha o frontend.

## Constitution Check

PASS. II (LGPD): reduz superfície (visitante anônimo não altera estado
global). V/VIII: mudança pequena e localizada; a classificação (regra) não
muda. VII: testes de rota/endpoint. XII: remove caminho morto na web.
Sem violações → sem Complexity Tracking.

## Project Structure (afetado)

```text
app/src/services/ipc.ts     # exportar emTauri() (detecção web/desktop)
app/src/App.vue             # esconder link "Categorias" no web
app/src/router/index.ts     # guard /categorias → redirect no web
server/src/lib.rs           # remover rotas GET/PUT /api/categorias + seed
server/src/rotas.rs         # remover handlers listar/salvar_categorias
server/tests/api.rs         # remover teste CRUD categorias; ajustar cookie
```

**Structure Decision**: frontend compartilhado → condicionar por `emTauri()`
(desktop mantém tudo; web esconde). API web perde os endpoints de categorias
(desktop usa IPC Tauri, não HTTP). Sem mudança no núcleo (`gedocs-core`).
