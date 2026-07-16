# Feature Specification: Resumo por IA em lote

**Feature Branch**: `011-resumo-lote`

**Created**: 2026-07-08

**Status**: Draft

**Input**: "melhorar o resumo com IA — enviar em lote em vez de 1 documento por vez"

## Contexto

O resumo por IA (modo `llm`) faz **1 chamada por documento** e o throttle
(~1,2 s, R9) serializa tudo — paralelizar não ajuda (o throttle é global). Esta
feature envia os documentos **em lote** (N por chamada), reduzindo o tempo.
**Fidelidade é crítica (Princípio I)**: o resumo NUNCA pode ser de outro
documento nem inventado — por isso cada item é **ancorado por id** e, à menor
suspeita (contagem/ids divergentes, JSON inválido), o lote **cai para o resumo
por-documento** já existente.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Resumir mais rápido, sem misturar documentos (Priority: P1)

Como usuário do modo IA, quero resumos rápidos em buscas grandes, com cada
resumo fiel ao seu documento (nunca trocado/inventado).

**Independent Test**: resumir um lote de N documentos e verificar que cada
resumo corresponde ao seu documento (por id), com muito menos chamadas que N.

**Acceptance Scenarios**:

1. **Given** N documentos com texto e modo IA, **When** resume, **Then** usa
   lotes (poucas chamadas) e cada documento recebe **seu** resumo (por id).
2. **Given** a resposta do lote com contagem/ids divergentes ou JSON inválido,
   **When** ocorre, **Then** o sistema **cai para o resumo por-documento** só
   naquele lote (R11) — fidelidade preservada, busca não aborta.
3. **Given** um documento sem texto-fonte (nem PDF, nem trecho), **When**
   resume, **Then** recebe o marcador "(sem texto)" — sem chamar a IA.
4. **Given** documentos já resumidos (cache por link, R6), **When** resume de
   novo, **Then** não entram no lote (cache hit).

---

### Edge Cases

- **Misattribution (crítico)**: id-âncora + validação de contagem/ids;
  divergência → fallback por-doc (não aceita resumo de item não confirmado).
- **Limite de tokens**: texto de PDF é grande → **lote pequeno** (ex.: 4–5) e
  texto truncado por documento (como hoje).
- **Falha parcial**: item ausente na resposta → fallback por-doc para ele.
- **Cache**: só não-cacheados entram no lote; resultados válidos são cacheados.

## Requirements *(mandatory)*

- **FR-001**: O resumo por IA MUST processar documentos **em lote** (vários por
  chamada), reduzindo o nº de chamadas.
- **FR-002**: Cada resumo MUST ser **ancorado por id** ao seu documento; nunca
  atribuído por ordem posicional.
- **FR-003**: Fidelidade (Princípio I): à menor divergência (contagem/ids/JSON
  inválido) o lote MUST cair no **resumo por-documento** existente — nunca
  aceitar um resumo não confirmado como do documento certo.
- **FR-004**: Documento sem texto-fonte MUST receber "(sem texto)" sem chamar a
  IA; falha de IA num documento MUST deixar o resumo ausente sem abortar (R11).
- **FR-005**: O cache por link (R6) MUST ser respeitado (só não-cacheados no
  lote; válidos cacheados).
- **FR-006**: O conteúdo do resumo MUST derivar do texto real do documento (R1)
  — sem invenção; o resumo continua fiel como no modo 1-por-doc.

### Key Entities

- **Lote de resumo**: documentos (id + texto-fonte truncado) numa chamada;
  resposta = itens `{id, resumo}`.

## Success Criteria *(mandatory)*

- **SC-001**: Para N documentos com texto não cacheados, o nº de chamadas de
  resumo cai para ~⌈N/tamanho_lote⌉.
- **SC-002**: 0 casos de resumo atribuído ao documento errado (id-âncora +
  fallback garantem).
- **SC-003**: Falha/ambiguidade de lote nunca aborta a busca (100% caem para
  por-doc).
- **SC-004**: Qualidade/fidelidade do resumo equivalente ao modo 1-por-doc.

## Assumptions

- A IA aceita JSON estruturado (`{"itens":[{"i":..,"resumo":".."}]}`).
- Lote de resumo pequeno (4–5) por causa do tamanho do texto de PDF.
- Reusa cache/fallback/texto-fonte já existentes (`services::resumidor`).

## Out of Scope

- Classificação em lote (spec 010).
- Mudar o prompt/idioma do resumo ou o modelo.
