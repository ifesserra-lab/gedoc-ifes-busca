# Implementation Plan: Busca por nome/palavra-chave

**Branch**: `009-busca-por-nome` | **Date**: 2026-07-08 | **Spec**: [spec.md](./spec.md)

## Summary

Adicionar um **modo de busca**: `siape` (padrão) e `nome`. No modo `nome`, o
termo vai ao portal como palavra-chave, **sem** validação de SIAPE e **sem** o
filtro anti-falso-positivo por SIAPE — assim documentos que a busca por SIAPE
não traz (ex.: antigos sem o SIAPE no texto) aparecem. Reusa todo o pipeline
(coleta/paginação/classificação/resumo/ordenação). Mudança no núcleo
(`usecases::buscar`) + DTO + as duas bordas (tauri/server) + toggle no front.

## Technical Context

**Language**: Rust (`gedocs-core`) + Vue/TS (`app`). **Testing**: `cargo test`
(core), `vitest` (front). Sem novas deps.

## Constitution Check

PASS. II (LGPD): só dados públicos do portal; modo nome amplia alcance
(homônimos), opt-in consciente; sessão/TTL da web inalterados. V/VIII: reusa o
pipeline; só condiciona validação e o filtro por SIAPE. VII: testes do modo
nome (sem validar, sem filtrar) e regressão do modo SIAPE. Sem violações.

## Project Structure (afetado)

```text
core/src/dto.rs                  # BuscarPorSiapeInput: + campo `por` (siape|nome)
core/src/usecases/buscar.rs      # executar/executar_com_repo: + por_nome
                                 #   (pula validar + pula filtro por SIAPE) + testes
src-tauri/src/commands/buscar.rs # valida SIAPE só no modo siape; passa por_nome
server/src/rotas.rs              # passa por_nome ao executar
app/src/services/ipc.ts          # BuscarPorSiapeInput: + por
app/src/stores/busca.ts          # porNome + validação condicional + envia por
app/src/views/BuscaView.vue      # toggle SIAPE|Nome + rótulo/placeholder
app/tests/*                      # validação do modo nome
```

**Structure Decision**: modo carregado via `por` no DTO (`"siape"` default,
`"nome"`). No núcleo, `por_nome: bool` decide: (a) `siape::validar` só quando
`!por_nome`; (b) `validos = docs` (todos) quando `por_nome`, senão
`filtrar_por_siape + separar` (hoje). `montar_resultado`/classificação/
ordenação inalterados. Front: `porNome` alterna rótulo/validação; envia `por`.
