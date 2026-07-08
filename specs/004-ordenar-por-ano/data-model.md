# Data Model — Ordenar portarias por ano (Fase 1)

Sem mudança de esquema/contrato. Reusa as entidades existentes; só define a
**chave de ordenação** derivada.

## Documento / DocView (reuso)

Campo relevante para esta feature:

| Campo | Tipo | Uso na ordenação |
|---|---|---|
| `data` | string? | fonte do **ano** (formato `DD/MM/AAAA` do portal) |

**Chave de ordenação derivada** (não persistida):

- `ano` = inteiro extraído de `data` (os 4 dígitos do ano). Ausente/ilegível
  → `None`.
- `data_completa` = a data como comparável (quando parseável), para desempate
  dentro do mesmo ano.

## Regra de ordenação (dentro de cada `CategoriaGrupo`)

1. Documentos **com ano** vêm antes dos **sem ano**.
2. Entre os com ano: **ano decrescente** (maior primeiro).
3. Empate de ano: **data completa decrescente**; sem data completa
   comparável, mantém a **ordem original** (ordenação estável).
4. Documentos **sem ano**: ao final, na ordem original.

Invariantes preservadas (FR-006): mesmo conjunto de documentos por categoria,
mesma contagem (`qtd`/`total`), mesmo agrupamento — só a ordem muda.
