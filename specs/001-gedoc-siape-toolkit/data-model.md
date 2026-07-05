# Data Model — GeDoc IFES Toolkit

Fase 1. Derivado de [docs/ontology.yaml](../../docs/ontology.yaml). Entidades,
campos, validações (regras R#) e relações, mapeadas para o backend Rust.

## Entidades

### Servidor
| Campo | Tipo | Regras |
| --- | --- | --- |
| siape | String | obrigatório; `^[0-9]{5,8}$` (R10) |
| nome | String? | extraído do texto |

### Documento
| Campo | Tipo | Regras |
| --- | --- | --- |
| link | Url | chave; `/documento/<hash32>?inline` |
| titulo | String | obrigatório |
| tipo | Enum(TipoDocumento) | PORTARIA \| Despacho \| Outro |
| numero | String? | após "Nº" |
| ano | u16? | do título; senão da data |
| data | Date? | DD/MM/YYYY |
| trecho | String? | snippet da busca |
| siapes | Vec\<String\> | SIAPEs citados |
| arquivo | String? | `AAAA_NUMERO_ASSUNTO.pdf` (R3) |
| contem_siape | bool | termo presente no trecho (R2) |
| categoria | Ref(Categoria.nome)? | exatamente uma (R4) |
| resumo | String? | fiel à fonte (R1) |

### Categoria
| Campo | Tipo | Regras |
| --- | --- | --- |
| nome | String | obrigatório, único |
| descricao | String? | critério do LLM (R5) |
Persistência: `config/categoria.json`.

### ResultadoBusca
| Campo | Tipo |
| --- | --- |
| termo | String |
| total_bruto | u32 |
| total_com_siape | u32 |
| documentos | Vec\<Documento\> |
| descartados | Vec\<Documento\> |
Persistência: `data/resultado_<siape>.json`.

### Resumo
Relatório agregado por categoria → `data/resumo_<siape>.md` (+ `.pdf`).

### Cache
`{ tipo: classificacao|resumo, chave: link, valor: String }` → arquivos JSON
por SIAPE (R6).

### Repositorio
Enum: `0` Boletim · `1` GeDoc (padrão) · `2` Site.

## Relações
- Documento **cita** Servidor (N..N via `siapes`).
- Documento **pertence_a** Categoria (N..1).
- ResultadoBusca **contem** Documento (1..N); **buscado_em** Repositorio (N..1).
- Resumo **resume** ResultadoBusca (1..1).
- Cache **memoiza** Documento (1..1 via `link`).

## Invariantes (regras de domínio)
R1 fidelidade · R2 filtro por SIAPE no trecho · R3 nome determinístico ·
R4 uma categoria · R5 categorias=config · R6 cache por link · R7 sem PII
versionada · R8 IDs dinâmicos · R9 degradação segura · R10 SIAPE válido.

## Traits (ports) do domínio
- `GedocRepository`: `buscar(termo, repo) -> ResultadoBusca` (Repository).
- `Classificador`: `classificar(&Documento, &[Categoria]) -> String` (Strategy:
  keyword | llm).
- `Resumidor`: `resumir(texto) -> String`.
- `Cache`: `get(link)`, `put(link, valor)`.
