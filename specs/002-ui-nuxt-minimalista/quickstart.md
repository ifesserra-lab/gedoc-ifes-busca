# Quickstart / Validação — UI com Nuxt UI

Guia para instalar, rodar e validar a nova interface. Componentes/estados em
[contracts/ui-components.md](./contracts/ui-components.md); decisões em
[research.md](./research.md).

## Instalação (frontend `app/`)
```bash
cd app
npm i @nuxt/ui                 # traz Tailwind v4 + Reka UI (transitivo)
npm i @fontsource/inter        # fonte self-hosted (CSP self)
npm i -D @iconify-json/lucide  # ícones locais (offline)
```
- `vite.config.ts`: adicionar o plugin `@nuxt/ui/vite`.
- `main.ts`: `import ui from '@nuxt/ui/vue-plugin'; app.use(ui)`.
- Importar `assets/tokens.css` e `assets/fonts.css` no `main.ts`.

## Rodar
```bash
# app desktop (Tauri)
app/node_modules/.bin/tauri dev
# ou só o front no navegador
cd app && npm run dev
```

## Testes (TDD)
```bash
cd app && npm test
```

## Cenários de validação
1. **US1 consistência**: navegar Busca ↔ Categorias → mesmo design system
   (tokens), tema claro/escuro aplicado em ambas, contraste AA.
2. **US2 estados**: forçar loading/vazio/erro/sucesso na Busca → componente
   dedicado em cada um; botão desabilita em loading.
3. **Offline/CSP**: abrir no Tauri → sem erros de CSP no console; nenhuma
   requisição a fonte/ícone externo (fontes/ícones locais).
4. **A11y**: navegar só com teclado (Tab/Enter/Esc no modal); foco visível.

## Critérios de sucesso
SC-001 telas 100% no design system · SC-002 contraste ≥ 4.5:1 (light/dark) ·
SC-003 5 estados presentes · SC-004 carrega sem requisições externas.
