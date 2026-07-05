# Design aprovado — tokens e diretrizes (feature 002)

Direção aprovada (2026-07-05): minimalista institucional, **1 acento verde-pinho**
(identidade IFES) + neutros com viés verde; dado em mono tabular; densidade de
ferramenta desktop. Referência visual: mockup aprovado (Artifact).

## Cores (tokens)

| Token | Light | Dark |
| --- | --- | --- |
| paper (fundo) | `#F6F8F6` | `#0E1512` |
| surface | `#FFFFFF` | `#151E1A` |
| surface-2 | `#F0F4F1` | `#1B2621` |
| ink (texto) | `#14211B` | `#E8EEEA` |
| muted | `#5E6B64` | `#93A79C` |
| faint | `#8A968F` | `#6D7E75` |
| border | `#E4EAE5` | `#27332C` |
| accent | `#17784E` | `#34B37E` |
| accent-press | `#125F3D` | `#2A9169` |
| accent-soft (bg/foco) | `#E7F1EC` | `#122A20` |
| danger | `#B23B2E` | `#EF9A8E` |
| warn | `#9A6B12` | `#E4B75C` |

Regras: só o **acento** carrega cor; chips de categoria são neutros e só o
selecionado fica accent. Contraste ≥ 4.5:1 em ambos os temas.

## Tipografia
- UI: `"Inter", -apple-system, "Segoe UI", Roboto, system-ui, sans-serif`
  (Inter self-hosted via @fontsource, sempre disponível offline)
- Dado (SIAPE, datas, contagens): `ui-monospace, "SF Mono", Menlo, monospace`
  com `font-variant-numeric: tabular-nums`.
- Escala: 12 / 13 / 14 / 16 / 20 / 28 / 34. Labels caixa-alta, tracking ~.12em.
- Títulos: `text-wrap: balance`. Corpo/resumo: máx ~60ch.

## Layout
- Top bar sticky (marca + nav Buscar/Categorias + toggle de tema).
- Busca: eyebrow + título + campo SIAPE (mono) + botão; hint abaixo.
- Cabeçalho-resumo: total (mono) + pessoa; ações de export à direita.
- Filtro: chips com contagem (mono); "Todos" default.
- Lista agrupada por categoria (label hairline): cada doc = dot + título + meta
  (data mono + pill categoria) + resumo + ação PDF.
- Coluna única, `max-width: 960px`, respiro generoso, divisórias hairline,
  raio 10/16, sombra discreta.

## Estados (5)
idle · loading (skeleton shimmer) · vazio · erro (com "tentar novamente") ·
sucesso. Componentes base reutilizáveis (LoadingState/EmptyState/ErrorState).

## Acessibilidade
Alvos ≥ 40px (ações) / 48px (botão primário de busca); foco visível
(`0 0 0 3px accent-soft`); `aria` em ícones; `prefers-reduced-motion`.

## Mapeamento p/ Nuxt UI 4
- accent → `primary` no `ui.config.ts` (paleta accent 50–950).
- chips → `UBadge`/`UButton` toggle; lista → `UCard`/`UTable`; export → `UButton`
  variantes `ghost`; input → `UInput` (mono). Tokens em `assets/tokens.css`.
