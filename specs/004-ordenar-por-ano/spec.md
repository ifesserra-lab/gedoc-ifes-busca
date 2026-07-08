# Feature Specification: Ordenar portarias por ano

**Feature Branch**: `004-ordenar-por-ano`

**Created**: 2026-07-08

**Status**: Draft

**Input**: User description: "ordene as portarias por ano"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Ver documentos ordenados por ano (Priority: P1)

Como usuário que consulta os documentos de um SIAPE, quero que as portarias
apareçam ordenadas por ano (mais recente primeiro), para localizar
rapidamente os atos mais novos sem varrer a lista inteira.

**Why this priority**: é o valor central do pedido; muda diretamente como o
resultado da busca é lido. Entrega utilidade imediata sozinha.

**Independent Test**: buscar um SIAPE com documentos de anos variados e
verificar que, em cada categoria, os documentos aparecem do ano mais recente
para o mais antigo.

**Acceptance Scenarios**:

1. **Given** um resultado com documentos de anos diferentes (ex.: 2018,
   2022, 2019), **When** a lista é exibida, **Then** os documentos aparecem
   ordenados do ano mais recente para o mais antigo (2022, 2019, 2018).
2. **Given** dois documentos do mesmo ano, **When** a lista é exibida,
   **Then** eles são ordenados pela data completa (mais recente primeiro) e,
   sem data completa comparável, mantêm a ordem original (ordenação estável).
3. **Given** documentos sem data, **When** a lista é exibida, **Then** eles
   aparecem por último, depois de todos os documentos com data.
4. **Given** o filtro por categoria (chip) e a visão "Todas", **When**
   qualquer uma é exibida, **Then** a ordenação por ano vale igualmente.

---

### Edge Cases

- Documento **sem data** (`data` ausente): vai para o fim da lista, sem
  quebrar a ordenação dos demais.
- **Empate de ano**: desempata pela data completa (dia/mês) quando
  disponível; senão preserva a ordem de origem (estável).
- **Data em formato inesperado**: se o ano não puder ser extraído, o
  documento é tratado como "sem data" (vai ao fim), nunca causa erro.
- Ordenação vale **por grupo de categoria** e também na visão "Todas".

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema MUST exibir os documentos de cada categoria
  ordenados pelo **ano** do documento.
- **FR-002**: A ordem padrão MUST ser **decrescente** (ano mais recente
  primeiro).
- **FR-003**: Empates no mesmo ano MUST ser desempatados pela data completa
  (mais recente primeiro); sem data completa comparável, a ordenação MUST
  ser estável (preserva a ordem original entre iguais).
- **FR-004**: Documentos **sem data** (ou com data da qual não se extrai o
  ano) MUST aparecer por último, depois de todos os com data.
- **FR-005**: A ordenação MUST ser aplicada de forma consistente em cada
  grupo de categoria e na visão consolidada ("Todas").
- **FR-006**: A ordenação NÃO MUST alterar o conteúdo, o agrupamento por
  categoria, nem a contagem de documentos — só a ordem de exibição.

### Key Entities *(include if data involved)*

- **Documento**: item do resultado; possui uma **data** (da qual se extrai o
  **ano**) usada como chave de ordenação. Data pode estar ausente.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Em um resultado com documentos de anos variados, 100% dos
  documentos com data ficam em ordem decrescente de ano dentro de cada
  categoria.
- **SC-002**: 100% dos documentos sem data aparecem depois dos documentos
  com data.
- **SC-003**: O documento mais recente de cada categoria é o primeiro da
  lista daquela categoria.
- **SC-004**: A contagem por categoria e o total permanecem idênticos aos de
  antes da ordenação (a ordenação não perde nem duplica documentos).

## Assumptions

- **Direção padrão = decrescente** (mais recente primeiro); é o mais útil
  para consultar atos recentes. Caso se prefira crescente, é ajuste de
  configuração/decisão, não muda o restante da spec.
- O **ano** é derivado do campo de **data** do documento (formato usual
  `DD/MM/AAAA` vindo do portal); quando ausente/ilegível, o documento é
  tratado como "sem data".
- A ordenação é **por categoria** (a tela agrupa por categoria); dentro de
  cada grupo os documentos seguem a ordem por ano. Vale também na visão
  "Todas".
- É uma mudança de **apresentação** (ordem de exibição); não altera coleta,
  filtro por SIAPE, classificação nem resumo.

## Out of Scope

- Ordenar por outros campos (título, categoria, número da portaria).
- Controle de ordenação pelo usuário (alternar asc/desc na tela) — pode ser
  uma feature futura.
- Reordenar o conteúdo do relatório/ZIP (esta spec cobre a lista da busca; o
  relatório pode herdar a mesma ordem se já parte do mesmo resultado).
