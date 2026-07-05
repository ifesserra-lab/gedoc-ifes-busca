# Quickstart / Validação — GeDoc IFES Toolkit (Tauri + Vue)

Guia para rodar e validar a feature de ponta a ponta. Detalhes de dados e
contratos: [data-model.md](./data-model.md) · [contracts/ipc-commands.md](./contracts/ipc-commands.md).

## Pré-requisitos
- Rust (stable) + Node 18+.
- `config/.env` com a chave do serviço de IA (ou modo `keyword`).
- Categorias em `config/categoria.json`.

## Setup
```bash
npm install
npm run tauri dev      # sobe o app desktop (Vue + Rust)
```

## Testes (TDD — Princípio VII)
```bash
cargo test             # ou: cargo nextest run  (backend Rust)
npm run test           # Vitest (frontend)
```
Testes rodam **sem rede**: portal e IA são dublados.

## Cenários de validação (mapeados às User Stories)

1. **US1/US2 — buscar e filtrar**: informar um SIAPE conhecido →
   **esperado**: total = total do portal; todo item lista contém o SIAPE;
   itens sem o SIAPE aparecem como descartados.
2. **US3 — buscar pelo navegador**: SIAPE inválido → bloqueado; SIAPE válido →
   total + chips por categoria + lista com resumos.
3. **US4 — baixar organizado**: PDFs `%PDF` válidos, nome `AAAA_NUMERO_ASSUNTO`;
   nomes iguais não se sobrescrevem.
4. **US5 — classificar**: cada documento com exatamente uma categoria; soma das
   categorias = total.
5. **US6 — resumir**: resumo fiel; documento ilegível usa o trecho; falha de um
   não derruba os demais.
6. **US7 — relatório/zip**: PDF do resumo agrupado por categoria abre; ZIP
   contém todos os PDFs.
7. **US8 — categorias**: criar categoria (persiste em `config/categoria.json`);
   nome duplicado é rejeitado; remover funciona.

## Critérios de sucesso (do spec)
SC-001 coleta completa · SC-002 0 falsos-positivos · SC-003 reexecução sem novas
chamadas · SC-004 PDFs válidos · SC-005 sem PII versionada · SC-006 SIAPE em
cache < 3 s.
