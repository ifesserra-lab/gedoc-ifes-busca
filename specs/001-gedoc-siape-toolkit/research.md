# Research — GeDoc IFES Toolkit (Tauri 2.0 + Vue)

Fase 0. Decisões técnicas com justificativa e alternativas. Sem NEEDS
CLARIFICATION pendentes.

## D1 — Framework desktop
- **Decisão**: Tauri 2.0 (backend Rust, WebView).
- **Justificativa**: constituição v1.2.0; binário leve, seguro (permissions),
  Rust para I/O robusto.
- **Alternativas**: Electron (peso/segurança), PyInstaller do legado Python
  (não atende à stack alvo).

## D2 — Frontend
- **Decisão**: Vue 3 (Composition API, `<script setup>`, TS) + Pinia + Vue Router.
- **Justificativa**: mandado pela constituição; Pinia = ViewModel (MVC).
- **Alternativas**: React/Svelte (fora da stack definida).

## D3 — Acesso ao portal GeDoc (JSF/PrimeFaces)
- **Decisão**: cliente HTTP em Rust reproduzindo o fluxo AJAX parcial
  (ViewState + sessão), com **descoberta dinâmica dos IDs** (R8).
- **Justificativa**: o portal exige POST parcial; IDs autogerados mudam.
- **Alternativas**: headless browser (pesado/frágil). Rejeitado.
- **Referência**: comportamento validado no legado `src/buscar_gedoc.py`.

## D4 — Classificação e resumo (IA)
- **Decisão**: serviço externo via HTTP, atrás de um `port` (trait) com
  estratégias `keyword` e `llm` (Strategy pattern); cache por `link`.
- **Justificativa**: R4/R5/R6; troca de provedor sem afetar o domínio (DIP).
- **Alternativas**: acoplar a um SDK específico (rejeitado — acoplamento).

## D5 — Extração de texto do PDF
- **Decisão**: crate Rust de extração de texto (ex.: `pdf-extract`); fallback ao
  trecho da busca quando vazio.
- **Justificativa**: evita dependência externa de sistema (`pdftotext`).
- **Alternativas**: chamar binário `pdftotext` (dependência de ambiente).

## D6 — Geração do relatório PDF
- **Decisão**: montar HTML e imprimir via janela/Webview do Tauri para PDF.
- **Justificativa**: reusa a renderização já disponível; sem toolchain extra.
- **Alternativas**: crate de PDF puro (ex.: `printpdf`) — mais trabalho de layout.

## D7 — Persistência
- **Decisão**: arquivos em `data/` (resultados/PDFs/resumos/caches) e `config/`.
- **Justificativa**: simplicidade; conformidade com II/IV; sem banco.
- **Alternativas**: SQLite (`tauri-plugin-sql`) — adiado (YAGNI).

## D8 — Testes (TDD)
- **Decisão**: `cargo test`/`nextest` no Rust; Vitest no front; dublês para
  portal e IA (testes offline).
- **Justificativa**: Princípio VII; regras R1–R10 testáveis sem rede.
- **Alternativas**: testes de ponta-a-ponta com rede (frágeis) — só como smoke.

## Padrões de projeto adotados (Princípio IX)
- **Repository**: acesso ao GeDoc atrás de um trait (`GedocRepository`).
- **Strategy**: classificação `keyword` vs `llm`.
- **MVC**: Model/Controller (Rust) · View (Vue) · ViewModel (Pinia).
- **Ports & Adapters** (leve): domínio depende de traits, não de I/O concreto.
