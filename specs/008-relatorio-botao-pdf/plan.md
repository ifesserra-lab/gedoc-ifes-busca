# Implementation Plan: Botão "Baixar PDF" no relatório

**Branch**: `008-relatorio-botao-pdf` | **Date**: 2026-07-08 | **Spec**: [spec.md](./spec.md)

## Summary

Adicionar um botão **"Baixar PDF"** no HTML do relatório
(`core/src/services/relatorio.rs::markdown_para_html`) que aciona o
print-to-PDF do navegador (`window.print()` via `onclick` inline — sem
`<script>` externo). O botão usa o design system (spec 007) e é ocultado na
impressão (`@media print .no-print { display:none }`), então não aparece no
PDF. Mantém self-contained (sem recurso externo) e o conteúdo inalterado.
Vale desktop + web (mesmo núcleo).

## Technical Context

**Language**: Rust (`gedocs-core`, `services::relatorio`). **Testing**:
`cargo test` (HTML é string pura). Sem novas deps. Sem `<script>` externo
(onclick inline; a checagem de teste "sem `<script`" segue válida).

## Constitution Check

PASS. II: self-contained, nada externo. XII: botão com tokens do app, oculto
na impressão. VIII: mudança pequena (1 botão + CSS + teste). V: só
apresentação, sem regra de negócio. Sem violações.

## Project Structure (afetado)

```text
core/src/services/relatorio.rs   # markdown_para_html: injeta o botão no <body>
                                 # + CSS do botão (tokens) + .no-print no @media print
                                 # + --accent-contrast nos :root (texto sobre acento)
                                 # + teste do botão
```

**Structure Decision**: o botão é injetado no início do `<body>`
(`<div class="acoes no-print"><button ... onclick="window.print()">Baixar
PDF</button></div>`), antes do corpo do relatório. CSS do botão usa
`--accent`/`--accent-contrast`; `.no-print` some em `@media print`. `onclick`
inline evita `<script>` (mantém a invariante de "sem asset/JS externo" e o
teste existente).
