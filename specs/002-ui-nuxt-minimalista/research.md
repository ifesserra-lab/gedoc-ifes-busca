# Research — UI com Nuxt UI + design minimalista

Fase 0. Decisões com justificativa e alternativas.

## D1 — Nuxt UI em projeto Vue (não Nuxt)
- **Decisão**: usar **Nuxt UI 3 em modo Vue** (plugin Vite `@nuxt/ui/vite` +
  `import ui from '@nuxt/ui/vue-plugin'` no `main.ts`).
- **Justificativa**: pedido do usuário; Nuxt UI 3 suporta Vue puro (Vite),
  baseado em Tailwind v4 + Reka UI (acessível). Evita migrar para Nuxt full
  (SSR/servidor), incompatível com o modelo Tauri (SPA local).
- **Alternativas**: Nuxt full (peso/SSR desnecessário); PrimeVue/Vuetify
  (fora do pedido); manter CSS próprio (mais trabalho, menos consistência).

## D2 — Estilo: Tailwind v4
- **Decisão**: Tailwind v4 (trazido pelo Nuxt UI) + camada de tokens própria.
- **Justificativa**: utilitários + design tokens; tree-shaking.
- **Alternativas**: CSS puro com variáveis (perde utilitários e componentes).

## D3 — Fontes e ícones offline (CSP)
- **Decisão**: **self-host** de fontes via `@fontsource/*` e ícones locais
  (coleção Iconify instalada localmente, ex.: `@iconify-json/lucide`), sem CDN.
- **Justificativa**: CSP do Tauri é `default-src 'self'`; Nuxt UI por padrão
  puxaria fontes/ícones remotos — quebraria offline e a CSP (Princípio II/XII).
- **Alternativas**: afrouxar CSP (rejeitado — menor privilégio).

## D4 — Design system minimalista (tokens)
- **Decisão**: paleta neutra (escala de cinza) + **1 cor de acento**; tipografia
  com 1 família (ex.: Inter self-hosted) e escala modular; espaçamento múltiplo
  de 4; raio suave; sombras discretas. Light e dark via tokens.
- **Justificativa**: minimalista/moderno = poucos elementos, muito respiro,
  hierarquia por tipografia e espaço, não por cor.
- **Alternativas**: multi-cor/《темы》carregados (contra o minimalismo).

## D5 — Mapeamento de componentes (telas → Nuxt UI)
- Busca: `UInput` (SIAPE) + `UButton` + `UBadge` (chips de categoria) +
  `UCard`/`UTable` (lista) + estados com `USkeleton`/`UAlert`.
- Categorias (CRUD): `UTable` + `UModal` + `UForm`/`UInput` + `UButton`.
- Estados: componentes dedicados `LoadingState`, `EmptyState`, `ErrorState`
  (finos, sobre `UAlert`/`USkeleton`).

## D6 — Acessibilidade e temas
- **Decisão**: Reka UI (base do Nuxt UI) já entrega semântica/teclado; validar
  contraste dos tokens (AA) e foco visível; alternador de tema claro/escuro.
- **Justificativa**: Constituição XII (WCAG AA).

## D7 — Testes
- **Decisão**: Vitest + Vue Test Utils para validação (R10 no input), presença
  dos 5 estados e não-chamada de IPC em entrada inválida.
- **Justificativa**: Princípio VII (TDD); testes sem rede.
