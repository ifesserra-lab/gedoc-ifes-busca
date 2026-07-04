# GeDoc IFES Toolkit — Documento de Análise e Projeto

**Versão:** 1.0.0 · **Data:** 2026-07-04 · **Fonte:** código em `src/`,
`docs/ontology.yaml` e `.specify/memory/constitution.md`

Este documento descreve a análise (problema, atores, requisitos, domínio) e o
projeto (arquitetura, componentes, dados, decisões) do sistema de consulta,
categorização e resumo de documentos do portal GeDoc do IFES.

## Sumário

1. [Visão geral](#1-visão-geral)
2. [Objetivos e escopo](#2-objetivos-e-escopo)
3. [Análise de requisitos](#3-análise-de-requisitos)
4. [Atores e casos de uso](#4-atores-e-casos-de-uso)
5. [Modelo de domínio](#5-modelo-de-domínio)
6. [Regras de negócio](#6-regras-de-negócio)
7. [Arquitetura](#7-arquitetura)
8. [Fluxo de dados (pipeline)](#8-fluxo-de-dados-pipeline)
9. [Modelo de dados e persistência](#9-modelo-de-dados-e-persistência)
10. [Interfaces](#10-interfaces)
11. [Decisões de projeto](#11-decisões-de-projeto)
12. [Requisitos não-funcionais](#12-requisitos-não-funcionais)
13. [Riscos e limitações](#13-riscos-e-limitações)
14. [Referências](#14-referências)

---

## 1. Visão geral

O **GeDoc IFES Toolkit** automatiza a consulta de atos administrativos públicos
(portarias, despachos) no portal [GeDoc](https://gedoc.ifes.edu.br) a partir da
matrícula **SIAPE** de um servidor. Dado um SIAPE, o sistema:

1. busca e pagina todos os documentos no portal;
2. filtra os que realmente citam o SIAPE no texto;
3. baixa os PDFs com nome padronizado;
4. classifica cada documento em uma categoria (via LLM guiada por configuração);
5. resume cada documento (Mistral);
6. gera um relatório em Markdown/PDF e disponibiliza os arquivos.

O portal é uma aplicação **JSF/PrimeFaces**: a busca não é um `GET` simples —
exige `ViewState`, sessão (`jsessionid`) e requisições AJAX parciais. O sistema
encapsula esse protocolo e descobre os identificadores do formulário em tempo de
execução.

## 2. Objetivos e escopo

**Objetivo:** reduzir o trabalho manual de localizar, organizar e sintetizar
documentos de um servidor no GeDoc.

**No escopo:**
- Busca por SIAPE com paginação completa e filtro por conteúdo.
- Download determinístico de PDFs.
- Categorização configurável (palavra-chave ou LLM).
- Resumo automático e geração de PDF consolidado.
- CRUD de categorias e uma interface web unificada.

**Fora do escopo:**
- Autenticação de usuários / controle de acesso.
- Persistência em banco de dados relacional.
- Escrita de qualquer dado de volta ao portal GeDoc (somente leitura).

## 3. Análise de requisitos

### 3.1 Requisitos funcionais

| ID | Requisito |
| --- | --- |
| RF1 | Buscar documentos por SIAPE (5 a 8 dígitos) no repositório escolhido. |
| RF2 | Coletar todos os resultados percorrendo a paginação. |
| RF3 | Manter apenas documentos cujo texto contém o SIAPE buscado. |
| RF4 | Baixar os PDFs nomeados como `AAAA_NUMERO_ASSUNTO.pdf`. |
| RF5 | Classificar cada documento em uma categoria. |
| RF6 | Permitir cadastrar/editar/remover categorias (nome + descrição). |
| RF7 | Resumir cada documento e agrupar o relatório por categoria. |
| RF8 | Gerar PDF do relatório e ZIP dos documentos. |
| RF9 | Expor busca, resultados e downloads por uma interface web. |

### 3.2 Requisitos não-funcionais

| ID | Requisito |
| --- | --- |
| RNF1 | **Privacidade/LGPD:** dados de terceiros nunca versionados. |
| RNF2 | **Reprodutibilidade:** reexecução usa cache; sem custo repetido de API. |
| RNF3 | **Resiliência:** retry com backoff e throttle contra rate limit (429). |
| RNF4 | **Robustez:** falha de um documento não aborta o lote. |
| RNF5 | **Portabilidade:** dependências mínimas; execução a partir da raiz. |
| RNF6 | **Configurabilidade:** categorias e credenciais fora do código. |

## 4. Atores e casos de uso

**Atores**
- **Usuário/Analista** — informa o SIAPE, consulta resultados, baixa arquivos e
  administra as categorias.
- **Portal GeDoc** (sistema externo) — fonte oficial dos documentos.
- **API Mistral** (sistema externo) — classificação e resumo.

**Casos de uso**

```
        ┌──────────────────────── GeDoc IFES Toolkit ────────────────────────┐
        │                                                                     │
Usuário │  UC1 Buscar documentos por SIAPE ───────────────► Portal GeDoc      │
  ─────►│  UC2 Baixar PDFs                                                     │
        │  UC3 Categorizar documentos      ───────────────► API Mistral       │
        │  UC4 Resumir documentos          ───────────────► API Mistral       │
        │  UC5 Gerar PDF do relatório                                         │
        │  UC6 Baixar ZIP dos documentos                                      │
        │  UC7 Administrar categorias (CRUD)                                  │
        └─────────────────────────────────────────────────────────────────────┘
```

## 5. Modelo de domínio

Derivado de `docs/ontology.yaml`. Sete entidades e seis relações.

### 5.1 Diagrama de entidades

```
        cita (N..N via siapes)
Servidor ◄──────────────────────► Documento ──pertence_a(N..1)──► Categoria
 (SIAPE)                             ▲   │                          (nome,
                                     │   └─ tem ── Resumo(doc)       descricao)
                    contem (1..N)    │
ResultadoBusca ──────────────────────┘
   │  └─ buscado_em (N..1) ─► Repositorio (Boletim|GeDoc|Site)
   └─ resume (1..1) ─► Resumo (markdown/pdf)

Cache ──memoiza(1..1 via link)──► Documento     (tipos: classificacao | resumo)
```

### 5.2 Entidades principais

| Entidade | Identidade | Descrição |
| --- | --- | --- |
| **Servidor** | `siape` | Servidor do IFES; chave natural é a matrícula SIAPE. |
| **Documento** | `link` | Ato administrativo; título, número, ano, data, trecho, `siapes[]`, `arquivo`, `categoria`, `resumo`. |
| **Categoria** | `nome` | Rótulo + descrição (critério do LLM); persiste em `config/categoria.json`. |
| **ResultadoBusca** | `termo` | Saída de uma busca; `total_bruto`, `total_com_siape`, `documentos`, `descartados`. |
| **Resumo** | — | Relatório agregado (Markdown/PDF). |
| **Cache** | `link` | Memoização por documento (classificação e resumo). |
| **Repositorio** | `codigo` | Coleção-fonte: `0` Boletim, `1` GeDoc (padrão), `2` Site. |

## 6. Regras de negócio

Invariantes do domínio (rastreadas ao código e à constituição):

| ID | Regra | Origem |
| --- | --- | --- |
| R1 | Todo dado deriva do conteúdo real; sem alucinação. | Const. I |
| R2 | Documento só é válido se o SIAPE aparece no trecho. | `filtrar_por_siape` |
| R3 | Nome de arquivo determinístico `AAAA_NUMERO_ASSUNTO.pdf`. | `nome_arquivo` |
| R4 | Exatamente uma categoria por documento; inválido → `Outros`. | `classificar_llm` |
| R5 | Categorias são configuração (`categoria.json`), não código. | Const. IV |
| R6 | Idempotência: cache por link evita rechamar a API. | Const. III |
| R7 | PII de terceiros (tudo em `data/`) nunca versionada. | Const. II |
| R8 | IDs do formulário JSF descobertos dinamicamente. | `GedocClient.abrir` |
| R9 | Degradação segura; retry+backoff+throttle em falhas externas. | Const. I |
| R10 | SIAPE e termo validam `^[0-9]{5,8}$`. | `app._SIAPE_RE` |

## 7. Arquitetura

Arquitetura em camadas com separação `src/` (código) · `config/` (configuração)
· `data/` (saídas). O front web segue um padrão MVC leve (view HTML, controller
= handlers HTTP, model = módulos de domínio).

### 7.1 Diagrama de componentes

```
┌─────────────────────────── Cliente (navegador) ───────────────────────────┐
│  prototipo/app.html (busca)         prototipo/categorias_app.html (CRUD)    │
└───────────────┬───────────────────────────────┬───────────────────────────┘
                │ HTTP/JSON                        │ HTTP/JSON
┌───────────────▼───────────────────────────────▼───────────────────────────┐
│                         src/app.py  (Controller / HTTP)                     │
│  /api/buscar  /api/pdf  /api/zip  /api/doc   /api/categorias  (GET/POST)    │
│                         run_pipeline()  ── orquestra ──┐                     │
└───────────────┬───────────────┬───────────────┬───────┼────────────────────┘
                │               │               │       │
        ┌───────▼──────┐ ┌──────▼───────┐ ┌─────▼─────┐ ┌▼───────────────┐
        │buscar_gedoc  │ │ categorizar  │ │  resumir  │ │  md_para_pdf   │
        │ (busca+dl)   │ │(classificação)│ │ _mistral  │ │  (md → pdf)    │
        └───────┬──────┘ └──────┬───────┘ └─────┬─────┘ └───────┬────────┘
                │               │               │               │
                │        ┌──────▼───────────────▼──────┐        │
                │        │      mistral_client.py       │        │
                │        │  (chat, .env, API key)       │        │
                │        └──────────────┬───────────────┘        │
                ▼                       ▼                         ▼
        Portal GeDoc (JSF)        API Mistral               Chrome headless
                                                                  │
        config/categoria.json ◄── CRUD                      data/ (PDFs, JSON,
                                                             resumos, .pdf, cache)
```

### 7.2 Componentes

| Componente | Responsabilidade | Tecnologia |
| --- | --- | --- |
| `buscar_gedoc.py` | Sessão JSF, ViewState, paginação, filtro SIAPE, download, HTML/JSON. | `requests` |
| `categorizar.py` | Classificação keyword/LLM guiada por `categoria.json`; cache. | `requests`/Mistral |
| `resumir_mistral.py` | Extrai texto (`pdftotext`), resume, agrupa por categoria. | Mistral + poppler |
| `md_para_pdf.py` | Converte Markdown em PDF. | `markdown` + Chrome |
| `mistral_client.py` | Cliente único da API (chat, `.env`, resolução de chave). | `requests` |
| `app.py` | Servidor HTTP, rotas/API, orquestração da pipeline. | `http.server` (stdlib) |
| `app_categorias.py` | Servidor mínimo dedicado ao CRUD de categorias. | `http.server` |

## 8. Fluxo de dados (pipeline)

Cada etapa consome a saída da anterior; todas idempotentes via cache.

```
SIAPE ─► [buscar] ─► ResultadoBusca (JSON) + PDFs
                          │
                          ▼
                     [categorizar] ─► Documento.categoria  (cache classificação)
                          │
                          ▼
                     [resumir]     ─► Documento.resumo + Resumo.md  (cache resumo)
                          │
                          ▼
                     [pdf]         ─► Resumo.pdf
```

Sequência de uma busca web (UC1→UC5):

```
Usuário → app.py: POST /api/buscar {siape}
app.py  → buscar_gedoc: coletar + baixar        (se não houver cache)
app.py  → categorizar: classificar_docs         (LLM, cache por link)
app.py  → resumir_mistral: gerar_markdown        (resume + agrupa, cache)
app.py  → md_para_pdf: html_para_pdf             (gera PDF)
app.py  → Usuário: JSON {total, categorias[], tem_pdf}
```

## 9. Modelo de dados e persistência

Sem banco relacional — persistência em arquivos, separada por natureza:

| Artefato | Local | Versionado? |
| --- | --- | --- |
| Categorias (config) | `config/categoria.json` | Sim |
| Credenciais | `config/.env` | **Não** (segredo) |
| Resultado da busca | `data/resultado_<siape>.json` | **Não** (PII) |
| PDFs baixados | `data/documentos_<siape>/` | **Não** (PII) |
| Resumo | `data/resumo_<siape>.md` / `.pdf` | **Não** (PII) |
| Cache classificação | `data/classificacao_<siape>.json` | **Não** |
| Cache resumo | `data/resumos_<siape>_cache.json` | **Não** |

Chave de cache = `link` do documento (URL canônica com hash de 32 caracteres).

## 10. Interfaces

### 10.1 API HTTP (`src/app.py`)

| Método | Rota | Descrição |
| --- | --- | --- |
| GET | `/` | Página de busca. |
| GET | `/categorias` | Página CRUD de categorias. |
| POST | `/api/buscar` | `{siape}` → executa a pipeline; retorna resultado agrupado. |
| GET | `/api/pdf?siape=` | PDF do relatório. |
| GET | `/api/zip?siape=` | ZIP dos PDFs. |
| GET | `/api/doc?siape=&arquivo=` | PDF individual. |
| GET/POST | `/api/categorias` | Ler / gravar categorias. |

### 10.2 CLI

```bash
python3 src/buscar_gedoc.py <SIAPE> --baixar data/documentos_<siape> \
    --html data/index_<siape>.html --json data/resultado_<siape>.json
python3 src/categorizar.py  --json data/resultado_<siape>.json --modo llm ...
python3 src/resumir_mistral.py --json ... --classificacao ...
python3 src/md_para_pdf.py data/resumo_<siape>.md
python3 src/app.py          # sistema web completo
```

## 11. Decisões de projeto

| # | Decisão | Justificativa |
| --- | --- | --- |
| D1 | Descoberta dinâmica dos IDs JSF. | IDs autogerados quebram em redeploy do portal (R8). |
| D2 | Filtro por presença do SIAPE no trecho, não só no rótulo "SIAPE". | O snippet pode cortar a palavra "SIAPE"; evita falsos-negativos (R2). |
| D3 | Categorias como configuração (`categoria.json`). | Mudar critérios sem alterar código; descrição vira prompt do LLM (R5). |
| D4 | Cache por link em classificação e resumo. | Reexecução gratuita e resiliente a interrupção (R6). |
| D5 | Cliente Mistral único (`mistral_client`). | DRY; ponto único de auth, retry e `.env`. |
| D6 | Servidor em `http.server` (stdlib). | Zero dependências; contorna restrição de instalação (PEP 668). |
| D7 | PDF via Chrome headless. | Sem toolchain LaTeX/wkhtmltopdf; usa o que já existe. |
| D8 | Saídas em `data/` (git-ignored). | Conformidade LGPD; nenhuma PII de terceiros no repositório (R7). |

## 12. Requisitos não-funcionais

- **Privacidade (LGPD):** `data/` e `config/.env` no `.gitignore`; toda saída
  com PII permanece local. Princípio do menor privilégio na coleta.
- **Desempenho e custo:** cache por documento; SIAPEs já consultados respondem
  instantaneamente; chamadas LLM só para itens novos.
- **Resiliência:** `throttle` entre chamadas (evita HTTP 429) e retry com
  backoff exponencial em erros transitórios (429/5xx).
- **Robustez:** exceções por documento são capturadas; o lote continua.
- **Manutenibilidade:** responsabilidade única por módulo; type hints; camadas
  `src/config/data` bem separadas.

## 13. Riscos e limitações

| Risco | Impacto | Mitigação |
| --- | --- | --- |
| Mudança de layout do portal GeDoc. | Busca/parse quebram. | IDs dinâmicos (D1); erros claros quando o layout muda. |
| Rate limit / cota da API Mistral. | Classificação/resumo falham. | Throttle + retry; cache reduz chamadas. |
| Homônimos / SIAPE citado como nº de processo. | Falso-positivo na busca. | Filtro R2 por presença no trecho; exibição dos "descartados". |
| Qualidade do OCR/texto do PDF. | Resumo pobre. | `pdftotext -layout`; fallback ao trecho da busca. |
| Ausência de testes automatizados. | Regressões silenciosas. | Validação pós-execução; trabalho futuro: suíte de testes. |

## 14. Referências

- `docs/ontology.yaml` — ontologia do domínio (entidades, relações, regras).
- `.specify/memory/constitution.md` — princípios do projeto (v1.0.0).
- `README.md` — instalação e uso.
- Código-fonte: `src/`.
- Tauri v2 (frente desktop futura): <https://v2.tauri.app>.

---

### Checklist de documentação
- [x] Título claro e visão geral inicial.
- [x] Pré-requisitos e escopo definidos.
- [x] Diagramas (casos de uso, entidades, componentes, fluxo).
- [x] Tabelas de requisitos, regras e decisões.
- [x] Interfaces (API + CLI) com exemplos.
- [x] Riscos e limitações.
- [x] Sumário navegável.

### Manutenção
Atualizar este documento quando: (a) `ontology.yaml` mudar; (b) novos
endpoints/módulos forem adicionados; (c) a constituição for emendada. Manter a
versão do cabeçalho alinhada à da constituição.
