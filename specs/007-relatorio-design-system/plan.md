# Implementation Plan: Aplicar o design system no relatório

**Branch**: `007-relatorio-design-system` | **Date**: 2026-07-08 | **Spec**: [spec.md](./spec.md)

## Summary

Reestilizar o HTML do relatório (`core/src/services/relatorio.rs`, const
`CSS`) com os **tokens do app** (`app/src/assets/tokens.css`): paleta
verde-pinho + acento IFES, tipografia Inter, escala/espaçamento aprovados,
**tema claro/escuro** via `prefers-color-scheme`, e estilos de **impressão**.
Mantém o HTML **self-contained** (CSS embutido; sem fonte/recurso externo) e o
conteúdo/estrutura idênticos — só a camada visual muda. Vale desktop + web
(mesmo núcleo gera o HTML).

## Technical Context

**Language**: Rust (`gedocs-core`, `services::relatorio`). **Testing**:
`cargo test` (o HTML é string pura — testável sem rede). Sem novas deps.

## Constitution Check

PASS. II (privacidade): sem recurso externo (nada de rede pra renderizar).
XII (UI/UX): consistência com o design system, contraste AA nos dois temas.
IV/V: tokens espelhados do app (fonte de verdade em `tokens.css`/spec 002);
sem regra de negócio. VIII: mudança pequena (só a const `CSS` + 1 teste).
Sem violações.

## Project Structure (afetado)

```text
core/src/services/relatorio.rs   # reescrever const CSS (tokens do app + light/dark + print)
                                 # + teste travando tokens/tema/self-contained
```

**Structure Decision**: os tokens do app são **replicados embutidos** no CSS
do relatório (não dá para referenciar o CSS do app — self-contained). Define
`:root` (claro) + `@media (prefers-color-scheme: dark)` (escuro) + `@media
print` (impressão legível, força claro). Fonte: `Inter` no topo do stack
(cai para fonte de sistema se ausente — não embute o arquivo, mantém o HTML
pequeno; assumption da spec). Conteúdo/estrutura e nomes de arquivo
inalterados.
