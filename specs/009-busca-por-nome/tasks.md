---
description: "Task list — Busca por nome/palavra-chave"
---

# Tasks: Busca por nome/palavra-chave

**Tests**: obrigatórios (VII).

## Phase 3: US1 (P1) 🎯

- [x] T001 [US1] `core/src/dto.rs`: adicionar campo `por: Option<String>` em `BuscarPorSiapeInput` (`"siape"` default | `"nome"`).
- [x] T002 [US1] `core/src/usecases/buscar.rs`: `executar_com_repo` e `executar` recebem `por_nome: bool` — `siape::validar` só quando `!por_nome`; no modo nome `validos = docs` (sem `filtrar_por_siape`/`separar`). Classificação/resumo/ordenação inalterados.
- [x] T003 [US1] Testes core em `usecases/buscar.rs`: (a) `por_nome=true` mantém doc que NÃO cita o SIAPE (modo siape descarta); (b) `executar` com termo não-SIAPE no modo nome não dá `SiapeInvalido`; (c) modo siape segue validando+filtrando (regressão).
- [x] T004 [US1] `src-tauri/src/commands/buscar.rs`: valida SIAPE só no modo siape; passa `por_nome` a `executar`.
- [x] T005 [US1] `server/src/rotas.rs`: deriva `por_nome` do input e passa a `executar`.
- [x] T006 [US1] `app/src/services/ipc.ts`: `BuscarPorSiapeInput` + `por?: "siape"|"nome"`.
- [x] T007 [US1] `app/src/stores/busca.ts`: `porNome` (ref) + validação condicional (SIAPE regex só no modo siape; nome = termo não vazio) + envia `por` na busca.
- [x] T008 [US1] `app/src/views/BuscaView.vue`: toggle "Buscar por: SIAPE | Nome" + rótulo/placeholder/validação adaptados; texto do resumo ("SIAPE X" × "Nome: X").
- [x] T009 [US1] Testes front (vitest) do store: modo nome valida termo não vazio (sem exigir SIAPE) e envia `por:"nome"`.

## Phase 4: Polish

- [x] T010 [P] `cargo fmt`/`clippy -D warnings` + `cargo test` (core/server/tauri) e `vitest` + `vue-tsc` (app).
- [ ] T011 Validar no ar: modo nome traz docs que a busca por SIAPE não trazia (ex.: termo do caso 1466806).

## Notes

- Padrão continua SIAPE. Nome é opt-in, sem filtro por SIAPE (pode trazer
  homônimos — esperado). Só dados públicos do portal.
