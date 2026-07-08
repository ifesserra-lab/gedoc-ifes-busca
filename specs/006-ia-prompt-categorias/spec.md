# Feature Specification: IA classifica pelas categorias do category.json (no prompt)

**Feature Branch**: `006-ia-prompt-categorias`

**Created**: 2026-07-08

**Status**: Draft

**Input**: User description: "sempre o category.json para categorizar as portarias e a IA deve usar o category.json para categorizar as portarias. Colocar esse dado no prompt"

## Contexto

As categorias vivem em `category.json` (nome + descrição) — é a fonte de
verdade para classificar as portarias (Princípio IV: configuração sobre
código). A classificação por palavra-chave já usa esse arquivo. Esta feature
garante que a classificação **por IA** também use SEMPRE as categorias do
`category.json`: os nomes e descrições devem ir **dentro do prompt** enviado
à IA, para que ela escolha entre as categorias definidas — em vez de inventar
rótulos livres.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - IA classifica nas categorias configuradas (Priority: P1)

Como usuário, quero que a classificação por IA use as categorias definidas em
`category.json`, para que os documentos caiam nas categorias que eu configurei
(consistentes com o modo palavra-chave) e não em rótulos arbitrários.

**Why this priority**: é o objetivo direto do pedido; sem isso a IA pode
retornar categorias fora do conjunto, quebrando o agrupamento e a
consistência com as categorias configuradas.

**Independent Test**: com um `category.json` de categorias conhecidas, rodar a
classificação por IA e verificar que cada documento recebe uma das categorias
definidas (nenhum rótulo fora da lista).

**Acceptance Scenarios**:

1. **Given** um `category.json` com categorias (nome + descrição), **When** a
   IA classifica um documento, **Then** a categoria atribuída é **uma das
   definidas** no arquivo.
2. **Given** as categorias configuradas, **When** o pedido é montado para a
   IA, **Then** o **prompt contém os nomes e as descrições** das categorias
   (a IA recebe esse dado para decidir).
3. **Given** um documento que não se encaixa em nenhuma categoria, **When** a
   IA classifica, **Then** ele recebe a categoria padrão ("Outros") em vez de
   um rótulo inventado.
4. **Given** que eu edito o `category.json` (adiciono/removo/renomeio), **When**
   faço uma nova busca com IA, **Then** a classificação passa a considerar as
   categorias atualizadas, sem mudança de código.

---

### Edge Cases

- **`category.json` ausente/vazio**: a classificação não falha — os documentos
  caem em "Outros" (degradação segura, R11).
- **IA devolve uma categoria fora da lista** (ou texto livre): é mapeada para
  a categoria padrão ("Outros"), nunca aceita como categoria nova.
- **Descrição vazia** em uma categoria: usa só o nome no prompt; ainda válida.
- **Falha da IA em 1 documento**: cai no classificador por palavra-chave para
  aquele documento (R11), sem abortar o lote.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A classificação (palavra-chave e IA) MUST usar SEMPRE as
  categorias definidas em `category.json` como conjunto de destino.
- **FR-002**: O prompt enviado à IA MUST incluir os **nomes e descrições** das
  categorias do `category.json`, para guiar a decisão.
- **FR-003**: A categoria atribuída pela IA MUST ser uma das definidas; uma
  resposta fora do conjunto MUST ser mapeada para a categoria padrão
  ("Outros").
- **FR-004**: Editar o `category.json` MUST refletir na classificação
  seguinte, sem alterar código (Princípio IV).
- **FR-005**: Sem categorias configuradas (arquivo ausente/vazio), a
  classificação MUST degradar com segurança (documentos em "Outros"), sem
  erro.
- **FR-006**: A falha da IA para um documento MUST cair no classificador por
  palavra-chave para aquele documento, sem abortar o lote (R11).

### Key Entities *(include if data involved)*

- **Categoria** (`category.json`): `nome` + `descricao`. Conjunto único de
  destino da classificação; injetado no prompt da IA.
- **Documento**: recebe exatamente uma categoria do conjunto definido (ou
  "Outros").

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% dos documentos classificados pela IA recebem uma categoria
  pertencente ao `category.json` (0 rótulos fora do conjunto).
- **SC-002**: O prompt da IA contém as categorias configuradas (nome +
  descrição) em 100% das chamadas de classificação.
- **SC-003**: Alterar o `category.json` muda o resultado da classificação na
  busca seguinte sem qualquer mudança de código.
- **SC-004**: Falha de IA ou ausência de categorias nunca aborta a busca
  (100% dos casos degradam com segurança).

## Assumptions

- `category.json` é a **fonte única** de categorias (nome + descrição), já
  usada pelo modo palavra-chave; esta feature estende o mesmo conjunto à IA.
- A categoria padrão para "não se encaixa"/fora do conjunto é **"Outros"**
  (convenção já existente no domínio).
- A mudança é de **classificação/prompt**; não altera coleta, filtro por
  SIAPE, resumo nem a ordenação.
- Vale para as duas frentes que usam a IA (desktop e web), pois ambas
  compartilham o mesmo núcleo de classificação.

## Out of Scope

- Criar/editar categorias por outra interface (ver spec 005: web sem CRUD).
- Mudar o resumo por IA (US6) — esta feature é só sobre a classificação.
- Taxonomia hierárquica/subcategorias.
