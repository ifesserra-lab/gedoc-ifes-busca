# Data Model — Versão Web (Fase 1)

Entidades derivadas do [spec.md](./spec.md). As entidades de domínio
(Documento, Categoria, ResultadoView) **reusam** as estruturas já
existentes em `gedocs_lib` — não são redefinidas. A novidade da web é a
**Sessão**.

## Sessão (nova)

Contexto efêmero atribuído a um visitante sem login.

| Campo | Tipo | Regras |
|---|---|---|
| `id` | string opaca (aleatória) | portada no cookie `gedocs_sid`; única |
| `criada_em` | timestamp | definida no 1º acesso |
| `ultima_atividade` | timestamp | atualizada a cada request |
| `ttl` | duração | default 1h de inatividade (configurável) |
| `dir` | caminho | `<data>/sessions/<id>/` (documentos, relatorios, cache) |

**Regras**:
- Isolamento (FR-011): uma sessão só acessa arquivos sob o próprio `dir`.
  O `id` do cookie NUNCA compõe caminho sem sanitização.
- Expiração (FR-012): se `agora - ultima_atividade > ttl`, a sessão está
  expirada; a varredura remove seu `dir`.

**Estados**: `ativa` → (inatividade > TTL) → `expirada` → (varredura) →
`removida`. Nada persiste após `removida`.

## Documento (reuso — `DocView`)

Item retornado pela busca. Estrutura já existente.

| Campo | Tipo | Observação |
|---|---|---|
| `titulo` | string | |
| `data` | string? | |
| `link` | string | chave única (cache por link) |
| `arquivo` | string? | preenchido após download (nome, nunca caminho) |
| `resumo` | string? | preenchido no modo IA |

## Categoria (reuso)

Configuração **global** de classificação.

| Campo | Tipo | Regras |
|---|---|---|
| `nome` | string | obrigatório; único case-insensitive (FR-008) |
| `descricao` | string? | opcional |

**Concorrência**: última gravação vence, sem trava (FR-017).

## ResultadoView (reuso)

Agrupamento retornado pela busca.

| Campo | Tipo |
|---|---|
| `termo` | string (SIAPE) |
| `total` | número |
| `categorias` | lista de `{ categoria, qtd, itens[] }` |
| `tem_pdf` | booleano |

## Relatório (artefato)

HTML+MD consolidado da busca atual, gravado em
`<sessão.dir>/relatorios/<siape>_relatorio.{html,md}`. Determinístico por
SIAPE (sobrescreve).

## Pacote (ZIP) (artefato)

`<sessão.dir>/relatorios/<siape>_documentos.zip` com os PDFs baixados na
sessão. Sem PDFs → erro amigável (FR-007).

## Erro (`AppError`, reuso)

Serializado como `{ tipo, mensagem }`. Tipos: `SiapeInvalido`,
`FalhaPortal`, `FalhaIA`, `CategoriaSemNome`, `NomeDuplicado`,
`NaoImplementado`, `FalhaArquivo`. Mapeamento HTTP em
[contracts/http-api.md](./contracts/http-api.md).
