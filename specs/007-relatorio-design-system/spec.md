# Feature Specification: Aplicar o design system no relatório

**Feature Branch**: `007-relatorio-design-system`

**Created**: 2026-07-08

**Status**: Draft

**Input**: User description: "aplique o mesmo design system no relatório"

## Contexto

O relatório consolidado é um **HTML self-contained** (gerado por
`services::relatorio`, aberto/impresso no navegador — decisão de não depender
de Chrome headless). Hoje ele tem um estilo próprio, diferente da interface do
app. Esta feature faz o relatório usar **o mesmo design system** do app
(cores, tipografia, espaçamento, temas claro/escuro), para consistência
visual e melhor leitura/impressão — mantendo o HTML autossuficiente (sem
recursos externos).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Relatório com a identidade visual do app (Priority: P1)

Como usuário, quero que o relatório gerado tenha a mesma aparência do app
(cores, fonte, espaçamento, cabeçalhos, tabelas/listas por categoria), para
que seja reconhecível, legível e profissional ao arquivar/imprimir.

**Why this priority**: é o objetivo direto do pedido; a inconsistência visual
atual prejudica leitura e percepção de qualidade.

**Independent Test**: gerar um relatório e verificar que ele usa a mesma
paleta/tipografia/espaçamento do app e agrupa por categoria de forma legível,
sem depender de arquivos externos.

**Acceptance Scenarios**:

1. **Given** uma busca com resultados, **When** gero o relatório, **Then** ele
   usa a **mesma paleta e tipografia** do app (tokens de cor, fonte, tamanhos
   e espaçamentos), com cabeçalho e seções por categoria consistentes.
2. **Given** o relatório aberto, **When** o dispositivo/navegador está em
   **tema escuro**, **Then** o relatório respeita o tema (claro/escuro) de
   forma legível, com contraste adequado.
3. **Given** o relatório, **When** ele é aberto sem internet ou impresso
   ("Salvar como PDF"), **Then** o visual se mantém — o HTML é
   **self-contained** (estilos e fontes embutidos, sem recursos externos).
4. **Given** o mesmo conteúdo de antes, **When** aplico o design system,
   **Then** o **conteúdo e a estrutura** (título, SIAPE, categorias, resumos)
   permanecem os mesmos — só a apresentação muda.

---

### Edge Cases

- **Tema escuro** do sistema/navegador: cores adaptam sem perder contraste.
- **Impressão/PDF**: layout legível em página (sem depender de cores de fundo
  que não imprimem; contraste em preto e branco aceitável).
- **Sem internet / offline**: nenhum recurso externo (fonte/CSS/imagem) — tudo
  embutido; o relatório abre igual.
- **Relatório longo** (muitos documentos): continua legível e navegável.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O relatório MUST usar os mesmos **tokens de design** do app —
  paleta de cores, **tipografia** (mesma família e escala) e espaçamento.
- **FR-002**: O relatório MUST suportar **tema claro e escuro**, respeitando a
  preferência do sistema/navegador, com contraste adequado (WCAG AA,
  Princípio XII).
- **FR-003**: O relatório MUST permanecer **self-contained**: estilos (e
  fontes, se usadas) embutidos no próprio HTML; **nenhum** recurso externo
  (mantém a decisão atual de não depender de Chrome/rede).
- **FR-004**: A mudança MUST ser apenas de **apresentação** — conteúdo,
  estrutura (título, SIAPE, agrupamento por categoria, resumos) e os nomes de
  arquivo permanecem iguais.
- **FR-005**: O relatório MUST permanecer legível e bem paginado ao
  **imprimir/Salvar como PDF**.
- **FR-006**: Vale para os dois alvos (desktop e web), pois ambos geram o
  relatório pelo mesmo núcleo.

### Key Entities *(include if data involved)*

- **Relatório (HTML)**: documento self-contained gerado a partir da
  `ResultadoView` (título, SIAPE, categorias, documentos, resumos). Só a
  camada visual (CSS embutido) muda nesta feature.
- **Design system**: tokens de cor, tipografia e espaçamento já usados pelo
  app (base de referência da aparência).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: O relatório usa a mesma paleta e tipografia do app (verificável
  visualmente e pelos tokens aplicados), em 100% das seções.
- **SC-002**: O relatório é legível em tema claro **e** escuro, com contraste
  AA (≥ 4.5:1 no texto).
- **SC-003**: O relatório abre sem nenhuma requisição externa (0 recursos
  externos) — self-contained.
- **SC-004**: O conteúdo/estrutura do relatório é idêntico ao anterior (mesma
  informação; só o visual muda).

## Assumptions

- "Mesmo design system" = os **tokens do app** (cores/tipografia/espaçamento,
  temas claro/escuro), replicados **embutidos** no HTML do relatório (não é
  possível referenciar o CSS do app, pois o relatório é self-contained).
- A tipografia pode ser aproximada por fonte de sistema equivalente se
  embutir a fonte inflar demais o arquivo; a prioridade é a **consistência de
  paleta, escala e espaçamento**.
- Mantém-se a decisão de **não** usar Chrome headless: o HTML é aberto/impresso
  pelo navegador; "Salvar como PDF" fica a cargo do usuário.
- A geração do relatório continua no núcleo compartilhado (desktop + web).

## Out of Scope

- Mudar o **conteúdo** do relatório (novas seções, campos).
- Gerar PDF nativo no servidor (segue "Salvar como PDF" pelo navegador).
- Redesenhar o design system do app em si.
