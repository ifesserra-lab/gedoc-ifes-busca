# Research — Ordenar portarias por ano (Fase 0)

## R1. Onde ordenar

- **Decisão**: ordenar em `gedocs_core::usecases::buscar::montar_resultado`,
  ao montar os `itens` de cada `CategoriaGrupo`.
- **Justificativa**: é o ponto único que produz a `ResultadoView` consumida
  por desktop (Tauri) **e** web (axum) — ordenar aqui vale nos dois sem
  duplicar (Princípio V). Já é puro e testável.
- **Alternativas**: ordenar na View (Vue) — duplicaria a lógica e não valeria
  para o relatório/exportação; ordenar no filtro/coleta — cedo demais, antes
  do agrupamento. Rejeitadas.

## R2. Chave e direção de ordenação

- **Decisão**: chave = **ano** extraído de `Documento.data` (formato
  `DD/MM/AAAA` do portal). Direção **decrescente** (mais recente primeiro).
  Empate no ano → desempata pela **data completa** (dia/mês) decrescente;
  sem data completa comparável, mantém a ordem original.
- **Justificativa**: consultar atos recentes primeiro é o uso mais comum
  (spec, Assumptions). Extrair só o ano é robusto a variações menores de
  formato.
- **Alternativas**: crescente (fica como possível config futura); ordenar por
  data completa sempre (exige parse estрито — mais frágil se o formato variar,
  por isso o ano é a chave primária).

## R3. Documentos sem data / formato inesperado

- **Decisão**: se o ano não puder ser extraído (`data` ausente ou ilegível),
  o documento é tratado como **"sem ano"** e vai para o **fim** da lista;
  nunca gera erro (degradação segura, R11/Princípio I).
- **Justificativa**: mantém a lista utilizável mesmo com dados imperfeitos.

## R4. Estabilidade

- **Decisão**: usar ordenação **estável** (`slice::sort_by`/`sort_by_key` do
  Rust é estável) para que documentos com a mesma chave preservem a ordem de
  origem (previsível, reprodutível — Princípio III).

## Incógnitas remanescentes

Nenhuma. Direção default (desc) e tratamento de "sem data" documentados.
