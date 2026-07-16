# Feature Specification: Classificação por IA em lote

**Feature Branch**: `010-classificacao-lote`

**Created**: 2026-07-08

**Status**: Draft

**Input**: "melhorar o classificador com IA — enviar em lote em vez de 1 documento por vez"

## Contexto

Hoje a classificação por IA (modo `llm`) faz **1 chamada por documento**. Com o
throttle de ~1,2 s entre chamadas (R9), um SIAPE com 239 documentos leva
minutos. Esta feature envia os documentos **em lote** (N por chamada), reduzindo
drasticamente o nº de chamadas e o tempo — sem perder fidelidade (a IA continua
escolhendo entre as categorias do `category.json`; resposta fora do conjunto →
"Outros").

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Classificar mais rápido, mesmo resultado (Priority: P1)

Como usuário do modo IA, quero que a classificação seja rápida em buscas
grandes, mantendo as mesmas categorias do `category.json`.

**Independent Test**: classificar um lote de N documentos com a IA e verificar
que cada um recebe uma das categorias definidas, com muito menos chamadas de
IA do que N.

**Acceptance Scenarios**:

1. **Given** N documentos e modo IA, **When** classifica, **Then** usa poucas
   chamadas (lotes) em vez de N — cada documento recebe uma categoria do
   `category.json`.
2. **Given** a resposta do lote, **When** um item traz categoria fora da lista
   (ou vem malformado), **Then** esse documento cai em "Outros" (R4) — sem
   afetar os demais.
3. **Given** um lote que falha (erro de IA/JSON inválido/itens faltando),
   **When** ocorre, **Then** o sistema **cai para o caminho por-documento** (ou
   palavra-chave) só naquele lote (R11) — a busca nunca aborta.
4. **Given** documentos já classificados (cache por link, R6), **When**
   classifica de novo, **Then** eles não entram no lote (cache hit).

---

### Edge Cases

- **Lote parcial**: resposta com menos itens que o enviado → itens ausentes
  usam fallback por-documento.
- **Ordem/troca**: cada item é **ancorado por id/índice**, não por ordem —
  contagem e ids são validados; divergência → fallback.
- **Limite de tokens**: tamanho do lote é limitado para caber no contexto.
- **Cache**: só entram no lote os documentos não cacheados; resultados válidos
  são cacheados individualmente.

## Requirements *(mandatory)*

- **FR-001**: A classificação por IA MUST processar os documentos **em lote**
  (vários por chamada), reduzindo o nº de chamadas em relação a 1-por-doc.
- **FR-002**: Cada documento no lote MUST ser **ancorado por id**; o resultado
  é atribuído ao documento certo (não por ordem posicional).
- **FR-003**: A categoria atribuída MUST pertencer ao `category.json`; fora do
  conjunto ou ausente → "Outros" (R4).
- **FR-004**: Falha de um lote (erro/JSON inválido/itens faltando) MUST cair no
  caminho por-documento (ou palavra-chave) só daquele lote, sem abortar (R11).
- **FR-005**: O cache por link (R6) MUST ser respeitado — só não-cacheados
  entram no lote; resultados válidos são cacheados.
- **FR-006**: O resultado final (categorias por documento) MUST ser equivalente
  ao do modo 1-por-doc para os mesmos documentos/categorias (sem regressão de
  qualidade).

### Key Entities

- **Lote de classificação**: conjunto de documentos (id + título + trecho)
  enviado numa chamada; resposta = itens `{id, categoria}`.

## Success Criteria *(mandatory)*

- **SC-001**: Para N documentos não cacheados, o nº de chamadas de IA de
  classificação cai para ~⌈N/tamanho_lote⌉ (ex.: N=150, lote=15 → ~10 chamadas).
- **SC-002**: 100% dos documentos recebem uma categoria do `category.json`
  (fora do conjunto → Outros).
- **SC-003**: Falha de lote nunca aborta a busca (100% degradam para
  por-doc/keyword).
- **SC-004**: Mesma cobertura de categorias que o modo 1-por-doc.

## Assumptions

- A IA (Mistral) aceita resposta JSON estruturada (`{"itens":[{"i":..,"categoria":".."}]}`).
- Tamanho de lote inicial ~15 (título+trecho são curtos); ajustável.
- Reusa o mesmo prompt/critério (categorias no prompt, spec 006) e o cache/
  fallback já existentes.

## Out of Scope

- Resumo em lote (spec 011).
- Mudar o modelo/idioma do prompt.
