// Tema do Nuxt UI (modo Vue) — T007.
//
// Nuxt UI resolve suas cores semânticas (`primary`, `neutral`, ...) a partir
// de paletas Tailwind (50-950). Aqui só mapeamos QUAL paleta cada papel usa;
// as paletas `accent` (verde-pinho, identidade IFES) e `pine` (neutro com
// viés verde) são definidas em `assets/tokens.css` via `@theme static`, para
// manter uma única fonte de verdade de cor — ver
// `specs/002-ui-nuxt-minimalista/design-tokens.md` (redesign aprovado).
//
// Consumido por `vite.config.ts` (plugin `@nuxt/ui/vite`).
export const uiTheme = {
  ui: {
    colors: {
      primary: "accent",
      neutral: "pine",
    },
  },
  theme: {
    colors: ["primary", "secondary", "info", "success", "warning", "error"] as string[],
  },
};
