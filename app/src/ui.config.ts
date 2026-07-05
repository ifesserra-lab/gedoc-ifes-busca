// Tema do Nuxt UI (modo Vue) — T007.
//
// Nuxt UI resolve suas cores semânticas (`primary`, `neutral`, ...) a partir
// de paletas Tailwind (50-950). Aqui só mapeamos QUAL paleta cada papel usa;
// a paleta `accent` (1 cor de acento, D4) é definida em `assets/tokens.css`
// via `@theme static`, para manter uma única fonte de verdade de cor.
//
// Consumido por `vite.config.ts` (plugin `@nuxt/ui/vite`).
export const uiTheme = {
  ui: {
    colors: {
      primary: "accent",
      neutral: "slate",
    },
  },
  theme: {
    colors: ["primary", "secondary", "info", "success", "warning", "error"] as string[],
  },
};
