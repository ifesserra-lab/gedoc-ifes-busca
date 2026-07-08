# Feature Specification: Botão "Baixar PDF" no relatório

**Feature Branch**: `008-relatorio-botao-pdf`

**Created**: 2026-07-08

**Status**: Draft

**Input**: User description: "tenha um botão de download em pdf do relatório"

## Contexto

O relatório é um HTML self-contained aberto no navegador; hoje o usuário
precisa lembrar de "Imprimir → Salvar como PDF". Esta feature adiciona um
**botão visível "Baixar PDF"** no próprio relatório, que aciona o fluxo de
PDF do navegador (imprimir → salvar como PDF) em um clique. Mantém a decisão
de projeto de **não** depender de Chrome headless / geração de PDF no
servidor (frágil, dependência pesada): o PDF é o do navegador, e o layout de
impressão já foi preparado (spec 007). O botão **não** aparece no PDF final.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Baixar o relatório em PDF com um clique (Priority: P1)

Como usuário, quero um botão "Baixar PDF" no relatório, para salvar/imprimir
o PDF sem precisar achar o menu de impressão do navegador.

**Why this priority**: é o pedido direto; reduz atrito para o entregável mais
usado (arquivar/compartilhar o relatório em PDF).

**Independent Test**: abrir um relatório gerado e verificar que há um botão
"Baixar PDF" que, ao ser clicado, abre o diálogo de salvar/imprimir PDF do
navegador; o botão não aparece no PDF resultante.

**Acceptance Scenarios**:

1. **Given** um relatório aberto, **When** o usuário vê a tela, **Then** há um
   botão visível **"Baixar PDF"** (posição fixa/visível, sem rolar até achar).
2. **Given** o relatório, **When** o usuário clica em "Baixar PDF", **Then** o
   navegador abre o fluxo de PDF (imprimir → salvar como PDF) do relatório.
3. **Given** o PDF gerado/impresso, **When** ele é visualizado, **Then** o
   **botão não aparece** no documento final.
4. **Given** o relatório, **When** ele é aberto sem internet, **Then** o botão
   funciona — nada externo é carregado (self-contained).

---

### Edge Cases

- **Impressão/salvar**: o botão (e qualquer controle) é ocultado no
  `@media print`, para não sujar o PDF.
- **Sem internet**: nenhum recurso externo — o botão e sua ação são embutidos
  no HTML.
- **Navegador sem diálogo de impressão** (raro): o conteúdo do relatório
  continua acessível; o usuário ainda pode usar o menu do navegador.
- **Tema claro/escuro**: o botão respeita o design system (spec 007).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O relatório MUST exibir um botão visível rotulado **"Baixar
  PDF"** (ou equivalente claro), fácil de achar ao abrir.
- **FR-002**: Clicar no botão MUST acionar o fluxo de PDF do navegador
  (imprimir → salvar como PDF) para o relatório atual.
- **FR-003**: O botão (e outros controles de tela) MUST ser ocultado na
  impressão/PDF — não aparece no documento final.
- **FR-004**: O relatório MUST permanecer **self-contained** — o botão e sua
  ação são embutidos no HTML; **nenhum** recurso externo.
- **FR-005**: O **conteúdo, a estrutura e o visual** do relatório permanecem
  os mesmos (spec 007) — a feature só adiciona o botão.
- **FR-006**: Vale para os dois alvos (desktop e web), pois ambos geram o
  relatório pelo mesmo núcleo.

### Key Entities *(include if data involved)*

- **Relatório (HTML)**: documento self-contained gerado a partir da
  `ResultadoView`. Ganha um controle de tela ("Baixar PDF") que aciona o PDF
  do navegador; o controle é omitido na impressão.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 1 clique no botão abre o diálogo de salvar/imprimir PDF do
  navegador (0 passos extras de menu).
- **SC-002**: O botão não aparece no PDF final (0 ocorrências no documento
  impresso).
- **SC-003**: 0 recursos externos carregados pelo relatório (self-contained).
- **SC-004**: Conteúdo/estrutura do relatório idênticos ao anterior (só o
  botão é novo).

## Assumptions

- "Download em PDF" = **print-to-PDF do navegador** acionado por botão
  (`window.print()`), mantendo a decisão existente de não gerar PDF no
  servidor nem depender de Chrome headless (ver `services/relatorio.rs`). O
  layout de impressão já existe (spec 007).
- O botão vive **no HTML do relatório** (aberto em nova aba), no topo, com a
  identidade visual do app (spec 007), oculto em `@media print`.
- Um script inline mínimo é aceitável e permanece self-contained (sem recurso
  externo) — Princípio II mantido.

## Out of Scope

- Gerar um arquivo **.pdf** no servidor (headless Chrome / lib de PDF) —
  contradiz a decisão de projeto e adiciona dependência pesada.
- Personalizar o nome do arquivo PDF (fica a cargo do diálogo do navegador).
- Mudar o conteúdo/layout do relatório além do botão.
