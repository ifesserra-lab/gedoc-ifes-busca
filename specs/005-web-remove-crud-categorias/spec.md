# Feature Specification: Remover o CRUD de categorias na versão web

**Feature Branch**: `005-web-remove-crud-categorias`

**Created**: 2026-07-08

**Status**: Draft

**Input**: User description: "na versão web remova o crud de categorias."

## Contexto

Na versão web (uso interno, **sem login**), as categorias são uma
configuração **global** compartilhada. Deixar qualquer visitante criar,
editar ou remover categorias significa que uma pessoa altera a
classificação de todos, sem rastreabilidade. Esta feature **remove a
gestão de categorias na web**: as categorias passam a ser fixas (definidas
pelo operador/servidor) e a busca segue classificando normalmente. O app
**desktop** mantém o CRUD completo, sem mudança.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Web sem gestão de categorias (Priority: P1)

Como responsável pela versão web (sem login), quero que os visitantes NÃO
possam criar/editar/remover categorias, para que a classificação
compartilhada não seja alterada por qualquer um.

**Why this priority**: é o objetivo direto do pedido; sem isso, o estado
global fica exposto a alteração anônima (risco de integridade/consistência).

**Independent Test**: abrir a versão web e verificar que não há tela nem
ação de gerenciar categorias; a busca continua funcionando e classificando.

**Acceptance Scenarios**:

1. **Given** a versão web aberta, **When** o usuário navega pela interface,
   **Then** não existe tela nem link/menu de gerenciamento de categorias.
2. **Given** a versão web, **When** o usuário tenta acessar diretamente a
   rota de categorias, **Then** ela não está disponível (redirecionado/sem
   acesso), sem erro quebrado.
3. **Given** a versão web, **When** ocorre qualquer tentativa de gravar
   categorias (criar/editar/remover), **Then** a operação não é aceita.
4. **Given** a versão web, **When** o usuário faz uma busca, **Then** os
   documentos continuam classificados pelas categorias configuradas
   (a remoção do CRUD não afeta a classificação).

---

### User Story 2 - Desktop mantém o CRUD (Priority: P1)

Como usuário do app desktop, quero continuar podendo gerenciar categorias,
pois lá o uso é local e individual.

**Why this priority**: a remoção deve ser **apenas** na web; regressão no
desktop seria perda de funcionalidade existente.

**Independent Test**: no app desktop, abrir a tela de categorias e
criar/editar/remover normalmente.

**Acceptance Scenarios**:

1. **Given** o app desktop, **When** abro a gestão de categorias, **Then**
   posso criar, editar e remover como antes.
2. **Given** o app desktop, **When** salvo categorias, **Then** a próxima
   busca reflete as mudanças (comportamento atual preservado).

---

### Edge Cases

- **Rota direta** para categorias na web: tratada como indisponível
  (sem tela em branco/erro cru).
- **Chamada de escrita** de categorias chegando à web (ex.: cliente antigo):
  rejeitada/ignorada, sem efeito no estado global.
- A **classificação** por categoria na busca web permanece intacta (as
  categorias existem no servidor; só o gerenciamento pelo usuário some).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A versão web MUST NOT expor tela, menu ou link para
  gerenciar categorias (criar/editar/remover).
- **FR-002**: A versão web MUST NOT permitir gravar categorias (nenhuma
  operação de escrita disponível ao visitante).
- **FR-003**: O acesso direto à rota de categorias na web MUST resultar em
  indisponibilidade tratada (redirecionar/ocultar), sem erro quebrado.
- **FR-004**: A **busca** na web MUST continuar classificando os documentos
  pelas categorias configuradas (sem regressão).
- **FR-005**: O app **desktop** MUST manter o CRUD de categorias completo,
  inalterado.
- **FR-006**: Não MUST restar links/ações "mortos" apontando para a gestão
  de categorias na web.

### Key Entities *(include if data involved)*

- **Categoria**: rótulo de classificação (nome, descrição). Na web passa a
  ser **somente leitura pelo sistema** (fixa, definida pelo operador); na
  desktop continua gerenciável pelo usuário.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 0 caminhos na interface web levam à gestão de categorias
  (nenhuma tela/menu/link).
- **SC-002**: 100% das tentativas de gravar categorias pela web são
  recusadas/sem efeito.
- **SC-003**: A busca web classifica os documentos exatamente como antes da
  mudança (mesma cobertura de categorias).
- **SC-004**: O CRUD de categorias no desktop continua 100% funcional
  (criar/editar/remover + refletir na busca).

## Assumptions

- "Remover o CRUD" na web = remover a **gestão** (criar/editar/remover) e a
  tela correspondente; as categorias continuam existindo no servidor e
  guiando a classificação. Não é remover a classificação por categoria.
- As categorias na web são definidas pelo **operador/servidor** (config
  global), fora da interface do visitante.
- O **frontend é compartilhado** entre desktop e web; a remoção é
  condicionada ao contexto web, preservando o desktop.
- Sem login (contexto da versão web), não há papel de "administrador" na
  interface; portanto nenhuma gestão de categorias é exposta na web nesta v1.

## Out of Scope

- Gestão de categorias na web **atrás de login/administrador** (poderia ser
  uma feature futura quando houver autenticação).
- Mudança no formato/semântica das categorias em si (nomes, descrições).
- Alterações no CRUD do desktop.
