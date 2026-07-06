---
description: "Tasks — UI com Nuxt UI + design minimalista (Vue/Tauri), TDD"
---

# Tasks: Interface com Nuxt UI + design minimalista

**Input**: `specs/002-ui-nuxt-minimalista/` (spec.md, plan.md, research.md, contracts/)

**Tests**: INCLUÍDOS (Constituição VII — TDD). Vitest + Vue Test Utils.

**Organização**: por user story (US1, US2). Só o frontend `app/` é afetado.
Implementação delegada ao agente `ui-ux-designer`.

## Format: `[ID] [P?] [Story] Description`
- **[P]**: paralelizável (arquivos distintos). **[US#]**: user story.

---

## Phase 1: Setup (dependências e build)

- [ ] T001 Instalar `@nuxt/ui` em `app/` (traz Tailwind v4 + Reka UI)
- [ ] T002 [P] Instalar fontes/ícones locais: `@fontsource/inter`, `@iconify-json/lucide` (offline/CSP)
- [ ] T003 Registrar plugin Vue do Nuxt UI em `app/src/main.ts` (`@nuxt/ui/vue-plugin`)
- [ ] T004 Adicionar plugin `@nuxt/ui/vite` + Tailwind v4 em `app/vite.config.ts`

## Phase 2: Foundational (design system — bloqueia as US)

- [ ] T005 Criar tokens do design system em `app/src/assets/tokens.css` (cores neutras + 1 acento, tipografia, espaçamento, raio; light/dark)
- [ ] T006 [P] Self-host de fontes em `app/src/assets/fonts.css` (Inter via @fontsource) e importar no `main.ts`
- [ ] T007 Configurar acento/tema do Nuxt UI em `app/src/ui.config.ts` mapeando para os tokens
- [ ] T008 [P] Componentes base finos em `app/src/components/base/`: `LoadingState.vue`, `EmptyState.vue`, `ErrorState.vue` (sobre USkeleton/UAlert)
- [ ] T009 [P] Teste: `app/tests/estados.spec.ts` — cada base state renderiza (loading/vazio/erro)
- [ ] T010 Verificar CSP offline: nenhuma requisição externa de fonte/ícone (checar console no Tauri)

**⚠️ Concluir Fase 2 antes das US.**

---

## Phase 3: US1 — Interface consistente (P1) 🎯 MVP

**Meta**: telas Busca e Categorias no mesmo design system, tema claro/escuro.
**Teste independente**: navegar entre telas → tokens/componentes coesos, AA.

- [ ] T011 [P] [US1] Teste: `app/tests/BuscaView.spec.ts` — validação SIAPE (R10) bloqueia e não chama `invoke`
- [ ] T012 [P] [US1] Teste: `app/tests/CategoriasView.spec.ts` — modal abre; salvar rejeita nome vazio/duplicado
- [ ] T013 [US1] Recompor `app/src/views/BuscaView.vue` com UInput/UButton/UBadge/UCard(UTable) e tokens
- [ ] T014 [P] [US1] Componentes de busca em `app/src/components/busca/`: `CategoriaChips.vue`, `DocItem.vue`
- [ ] T015 [US1] Recompor `app/src/views/CategoriasView.vue` com UTable + UModal + UForm/UInput
- [ ] T016 [US1] Alternador de tema (claro/escuro) no cabeçalho em `app/src/App.vue`
- [ ] T017 [US1] Garantir contraste AA e foco visível nos tokens/estados (ajustes em tokens.css)

## Phase 4: US2 — Estados de UI claros (P2)

**Meta**: loading/vazio/erro/sucesso dedicados; feedback imediato.
**Teste independente**: forçar cada estado → componente correto; botão desabilita.

- [x] T018 [P] [US2] Teste: `app/tests/estados_busca.spec.ts` — store em loading/vazio/erro renderiza o componente certo
- [x] T019 [US2] Estados da store `busca.ts` ligados aos componentes base na BuscaView (já entregue em #13)
- [x] T020 [US2] Skeleton de lista em loading (`LoadingState :linhas`) + botão Buscar desabilitado/`:loading` (BuscaView)
- [x] T021 [US2] EmptyState ("nenhum documento") e ErrorState (com retry) na BuscaView
- [x] T022 [P] [US2] Toast de sucesso (Nuxt UI `useToast`) ao criar/editar/remover categoria (CategoriasView)

## Phase 5: Polish — design minimalista/moderno & a11y

- [ ] T023 [P] Refino visual: espaçamento/hierarquia/microinterações (transições sutis) nas telas
- [ ] T024 [P] Truncar títulos longos com reticências + tooltip (DocItem)
- [ ] T025 Auditoria a11y: navegação por teclado (Tab/Enter/Esc no modal), aria-labels em ícones
- [ ] T026 [P] Remover estilos ad-hoc antigos (garantir 100% via tokens — SC-001)
- [ ] T027 Revisão via agente `pr-reviewer` antes do PR; validar quickstart.md no app desktop

---

## Dependências
- Setup (F1) → Foundational (F2: design system) bloqueiam as US.
- **US1** depende de F2 (tokens + base states). **US2** depende de US1 (telas existem).
- Polish depende de US1+US2.

## Paralelização (exemplos)
- F2: T006, T008, T009 em paralelo.
- US1: T011, T012, T014 em paralelo; T013/T015 dependem dos tokens (F2).

## MVP
**US1** (P1): telas no design system, tema claro/escuro, consistência — já
entrega o valor visual. US2 (estados) e Polish são incrementos.

## Total
27 tasks · Setup:4 · Foundational:6 · US1:7 · US2:5 · Polish:5.
