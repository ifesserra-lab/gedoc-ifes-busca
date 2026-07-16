# Feature Specification: Busca por nome/palavra-chave (além do SIAPE)

**Feature Branch**: `009-busca-por-nome`

**Created**: 2026-07-08

**Status**: Draft

**Input**: User description: "especifique e implemente" — busca alternativa por
nome/palavra-chave para alcançar documentos que o portal não retorna na busca
por SIAPE (ex.: documentos antigos sem o SIAPE no texto indexado — caso
1466806, nada antes de 2016).

## Contexto

O portal GeDoc pesquisa por **palavra-chave** (campo "Palavras chave"). A
busca por SIAPE joga o número nesse campo e depois filtra os documentos que
citam o SIAPE no texto (anti-falso-positivo). Documentos antigos que **não
têm o SIAPE no texto indexado** não voltam. Esta feature adiciona um **modo
de busca por nome/palavra-chave**: o termo vai direto ao portal e os
resultados são exibidos **sem** o filtro por SIAPE — alcançando documentos que
a busca por SIAPE não encontra. A busca por SIAPE segue como padrão.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Buscar por nome/palavra-chave (Priority: P1)

Como usuário, quero buscar por nome (ou palavra-chave), para encontrar
documentos que a busca por SIAPE não retorna (ex.: atos antigos sem o SIAPE no
texto).

**Why this priority**: é o objetivo do pedido; destrava documentos hoje
inalcançáveis pela busca por SIAPE.

**Independent Test**: escolher o modo "Nome", buscar um nome e ver os
documentos do portal para aquele termo, sem exigir SIAPE e sem descartar por
SIAPE.

**Acceptance Scenarios**:

1. **Given** o modo **Nome/palavra-chave**, **When** busco por um termo (ex.:
   um nome), **Then** vejo os documentos que o portal retorna para aquele
   termo, **sem** filtro por SIAPE.
2. **Given** o modo **Nome**, **When** o termo não é um SIAPE, **Then** a
   busca **não** é rejeitada por validação de SIAPE (aceita texto livre).
3. **Given** o modo **SIAPE** (padrão), **When** busco, **Then** o
   comportamento é o de hoje: valida SIAPE (R10) e filtra por SIAPE
   (anti-falso-positivo, R2).
4. **Given** qualquer modo, **When** há resultados, **Then** eles são
   classificados/agrupados por categoria e ordenados por ano como hoje.

---

### Edge Cases

- **Termo vazio** no modo Nome: bloqueado com aviso (nada a buscar).
- **Termo muito curto/genérico** no modo Nome: pode trazer muitos resultados
  (inclusive de outras pessoas / homônimos) — é esperado nesse modo; o usuário
  escolhe usá-lo. Sem o filtro por SIAPE não há como distinguir homônimos.
- **Modo Nome com resultados de várias pessoas**: exibidos como vieram do
  portal (o app não infere a "pessoa certa" nesse modo).
- **Portal indisponível / sem resultados**: mesma mensagem amigável de hoje.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema MUST oferecer **dois modos de busca**: por **SIAPE**
  (padrão) e por **nome/palavra-chave**.
- **FR-002**: No modo **nome**, o termo MUST ir ao portal como palavra-chave e
  os resultados MUST ser exibidos **sem** o filtro por SIAPE.
- **FR-003**: No modo **nome**, a validação de SIAPE (5–8 dígitos, R10) MUST
  **não** ser aplicada; o termo aceita texto livre (não vazio).
- **FR-004**: No modo **SIAPE**, o comportamento atual MUST ser preservado:
  validação de SIAPE (R10) + filtro por SIAPE (R2, anti-falso-positivo).
- **FR-005**: Em ambos os modos, classificação (US5), resumo opcional (US6),
  agrupamento (US3) e ordenação por ano MUST continuar valendo.
- **FR-006**: A tela MUST deixar claro o modo selecionado e adaptar o rótulo/
  validação do campo (SIAPE numérico × termo livre).
- **FR-007**: Vale para desktop e web (mesmo núcleo de busca).

### Key Entities *(include if data involved)*

- **Consulta**: termo + modo (`siape` | `nome`). Modo decide validação e se o
  filtro por SIAPE é aplicado.
- **Documento / ResultadoView**: inalterados; no modo nome, `total` = itens
  (sem descarte por SIAPE).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: No modo nome, uma busca por um termo retorna os documentos do
  portal para aquele termo sem descartar por SIAPE (itens = total do portal).
- **SC-002**: No modo nome, termos não numéricos/não-SIAPE **não** são
  rejeitados pela validação de SIAPE.
- **SC-003**: No modo SIAPE, os resultados são idênticos aos de hoje (mesma
  validação + filtro) — sem regressão.
- **SC-004**: Documentos que a busca por SIAPE não trazia (ex.: antigos sem o
  SIAPE no texto) passam a aparecer quando buscados por nome.

## Assumptions

- O portal pesquisa por **palavra-chave** ("Palavras chave"); o modo nome usa
  o mesmo campo, só que sem o filtro por SIAPE no app.
- **Privacidade (Princípio II/LGPD)**: os documentos do portal são **públicos**;
  o modo nome amplia o alcance (pode trazer homônimos/terceiros), mas não expõe
  nada além do que o portal já publica. A sessão efêmera + TTL da web continua
  valendo. O usuário opta pelo modo nome de forma consciente.
- Modo padrão continua **SIAPE** (mais preciso). Nome é opt-in.
- Reusa o mesmo pipeline (coleta/paginação/classificação/ordenação); só muda
  validação e a aplicação (ou não) do filtro por SIAPE.

## Out of Scope

- Desambiguar homônimos no modo nome (o app não escolhe "a pessoa certa").
- Buscar por outros campos estruturados (processo, assunto) — o portal só
  expõe "Palavras chave" para texto.
- Mudar o modo padrão (segue SIAPE).
