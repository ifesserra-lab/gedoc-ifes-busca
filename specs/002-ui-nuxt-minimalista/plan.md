# Implementation Plan: Interface com Nuxt UI + design minimalista

**Branch**: `002-ui-nuxt-minimalista` | **Date**: 2026-07-05 | **Spec**: [spec.md](./spec.md)

**Input**: Reimplementar a interface com Nuxt UI (modo Vue) e aplicar design
minimalista e moderno. Issue #10.

## Summary

Substituir os estilos ad-hoc do frontend Vue (em `app/`) por **Nuxt UI 3 em
modo Vue** (Vite + Tailwind v4 + Reka UI), definindo um **design system
minimalista** por tokens (neutros + 1 acento, light/dark) e recompondo as telas
(Busca, Categorias e, quando existirem, Relatório/Downloads) com componentes do
Nuxt UI, mantendo os 5 estados e acessibilidade WCAG AA (Constituição XII).
Fontes/assets locais para respeitar a CSP `default-src 'self'` do Tauri.

Abordagem em 2 passos (pedido do usuário): **(1) integrar Nuxt UI** e portar as
telas; **(2) refinar o visual** minimalista/moderno (espaçamento, hierarquia,
microinterações).

## Technical Context

**Language/Version**: TypeScript 5 / Vue 3.5 (frontend em `app/`)

**Primary Dependencies**: `@nuxt/ui` (modo Vue) + `tailwindcss` v4 + `reka-ui`
(transitivo); fontes self-hosted (`@fontsource/*`) para offline/CSP.

**Storage**: N/A (feature de apresentação; dados vêm do backend via IPC — 001).

**Testing**: Vitest + Vue Test Utils (testes de componente/estado).

**Target Platform**: Desktop (Tauri 2.0 WebView) — offline após carregar.

**Project Type**: desktop-app (frontend Vue). Só `app/` é afetado.

**Performance Goals**: UI sem jank; bundle enxuto (tree-shaking do Nuxt UI).

**Constraints**: CSP `default-src 'self'` — **sem** requisições externas
(fontes/ícones locais); nenhuma regra de negócio na View.

**Scale/Scope**: 2–3 telas; ~6–10 componentes base.

## Constitution Check

*GATE: passar antes da Fase 0; reavaliar após a Fase 1.*

| Princípio | Como o plano atende | Status |
| --- | --- | --- |
| V Camadas/DRY | UI só apresenta; estado na store/IPC. | ✅ |
| VI OO / VIII Pequeno | Componentes pequenos, um propósito. | ✅ |
| VII TDD | Testes de componente (validação, estados). | ✅ |
| IX Padrões | Design system + biblioteca de componentes (não reinventar). | ✅ |
| X Issue-first | Issue #10 aberta antes. | ✅ |
| XI Agentes | Implementação delegada ao `ui-ux-designer`. | ✅ |
| XII UI/UX | Tokens light/dark, 5 estados, WCAG AA, feedback. | ✅ |
| II Privacidade | Sem PII; sem assets externos (CSP self). | ✅ |

**Resultado**: PASS.

## Project Structure

### Documentation (this feature)

```text
specs/002-ui-nuxt-minimalista/
├── plan.md
├── research.md
├── quickstart.md
└── contracts/
    └── ui-components.md      # inventário de telas/componentes/estados
```

### Source Code (frontend em `app/`)

```text
app/
├── src/
│   ├── assets/
│   │   ├── tokens.css        # design system (cores/tipografia/espacamento, light/dark)
│   │   └── fonts.css         # @fontsource self-hosted (CSP self)
│   ├── ui.config.ts          # app.config do Nuxt UI (cores/acento)
│   ├── components/
│   │   ├── base/             # wrappers finos (se preciso) sobre Nuxt UI
│   │   └── busca/            # ResultadoLista, CategoriaChips, DocItem
│   ├── views/                # BuscaView, CategoriasView (recompostas)
│   ├── stores/               # Pinia (inalterado)
│   ├── services/             # ipc.ts (inalterado)
│   └── main.ts               # registra o plugin Vue do Nuxt UI
├── vite.config.ts            # plugin @nuxt/ui/vite + tailwind v4
└── tests/                    # Vitest (estados, validação, a11y básica)
```

**Structure Decision**: manter `app/` (Vue+Vite). Adotar Nuxt UI via plugin Vue
+ Vite; não migrar para Nuxt full (evita SSR/servidor, mantém o modelo Tauri).

## Complexity Tracking

> Sem violações — seção não aplicável.
