# Feature Specification: Interface com Nuxt UI + design minimalista

**Feature Branch**: `002-ui-nuxt-minimalista`

**Created**: 2026-07-05

**Status**: Draft

**Input**: Reimplementar a interface do app (Tauri 2.0 + Vue 3) usando Nuxt UI
(modo Vue) e aplicar um design minimalista e moderno. Issue: #10.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Interface consistente e agradável (Priority: P1)

Como Analista, quero uma interface limpa e consistente (busca, resultados,
categorias), para usar o app com conforto e clareza.

**Why this priority**: a interface é o ponto de contato; consistência e clareza
reduzem erro e esforço.

**Independent Test**: navegar busca → resultados → categorias e ver o mesmo
sistema visual (tipografia, cores, espaçamento) e componentes coesos.

**Acceptance Scenarios**:

1. **Given** o app aberto, **When** navego entre as telas, **Then** cabeçalho,
   botões, inputs e cards seguem um único design system (tokens).
2. **Given** o sistema em tema claro ou escuro, **When** alterno, **Then** todas
   as telas respeitam o tema, com contraste WCAG AA.

---

### User Story 2 - Estados de UI claros (Priority: P2)

Como Analista, quero feedback visual claro em cada situação, para saber o que o
sistema está fazendo.

**Independent Test**: forçar cada estado (loading, vazio, erro, sucesso) e ver
um componente dedicado e legível.

**Acceptance Scenarios**:

1. **Given** uma busca em andamento, **When** aguardo, **Then** vejo estado de
   loading (skeleton/spinner) e o botão desabilitado.
2. **Given** um SIAPE sem resultados, **When** a busca termina, **Then** vejo um
   estado vazio com próximo passo; em erro, uma mensagem útil + "tentar de novo".

---

### Edge Cases

- Janela redimensionada (desktop): layout se adapta sem quebrar.
- Textos longos (títulos de portaria) truncam com reticências e tooltip.
- Sem conexão com serviço de IA: erro amigável, não trava a UI.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A UI MUST usar Nuxt UI (modo Vue) como biblioteca de componentes.
- **FR-002**: A UI MUST aplicar um design system minimalista via tokens (cor,
  tipografia, espaçamento, raio), com temas claro e escuro.
- **FR-003**: Toda tela MUST cobrir os cinco estados (idle, loading, vazio,
  erro, sucesso).
- **FR-004**: A UI MUST atender WCAG 2.1 AA (contraste, foco, teclado, aria).
- **FR-005**: Componentes MUST ser pequenos e sem regra de negócio (estado na
  store/IPC).
- **FR-006**: Fontes e assets MUST ser locais (sem rede) para respeitar a CSP do
  Tauri (`default-src 'self'`).

### Key Entities

- (Sem novas entidades de domínio — feature de apresentação; consome os dados de
  `specs/001-gedoc-siape-toolkit`.)

## Success Criteria *(mandatory)*

- **SC-001**: 100% das telas usam componentes do design system (sem estilos
  ad-hoc fora dos tokens).
- **SC-002**: Contraste de texto/ação ≥ 4.5:1 em claro e escuro.
- **SC-003**: Cada tela apresenta os 5 estados quando aplicável.
- **SC-004**: App carrega a UI sem requisições externas (CSP `self`), sem erros
  de console.

## Assumptions

- Nuxt UI 3 funciona em projeto Vue+Vite (não exige Nuxt full).
- Design minimalista = neutros + 1 cor de acento, muito espaço em branco,
  hierarquia tipográfica clara.
- Aplicar o design "moderno" após integrar os componentes (2 passos: integrar
  Nuxt UI → refinar visual).
