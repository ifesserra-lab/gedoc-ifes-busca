# GeDoc IFES — busca, categorização e resumo de documentos

Ferramentas em Python para pesquisar documentos no
[GeDoc do IFES](https://gedoc.ifes.edu.br/faces/pesquisarDocumentos/pesquisarHistorico.xhtml)
por palavra-chave (ex.: um número **SIAPE**), baixar os PDFs, **categorizá-los**
e **resumi-los** com a API da Mistral, gerando Markdown e PDF.

O portal roda em **JSF/PrimeFaces**: a busca exige `ViewState`, sessão
(`jsessionid`) e requisições AJAX parciais. Os ids do formulário são
descobertos dinamicamente (resistem a redeploy do servidor).

## Estrutura

```
gedocs/
├── src/                     # codigo
│   ├── buscar_gedoc.py      # busca + paginacao + filtro SIAPE + download + HTML
│   ├── categorizar.py       # classifica em categorias (keyword ou LLM)
│   ├── resumir_mistral.py   # resume cada doc e agrupa por categoria
│   ├── md_para_pdf.py       # Markdown -> PDF (via Chrome headless)
│   └── mistral_client.py    # cliente compartilhado da API Mistral
├── config/
│   ├── categoria.json       # nome + descricao das categorias (base do LLM)
│   ├── .env.example         # modelo de credenciais
│   └── .env                 # sua chave Mistral (NAO versionado)
├── data/                    # tudo gerado (PDFs, JSON, resumos, PDF) — NAO versionado
└── README.md
```

`data/` e `config/.env` estão no [.gitignore](.gitignore): contêm PDFs/resumos
com **dados pessoais de terceiros** e a chave da API. Gere-os localmente.

## Requisitos

- Python 3.8+, [`requests`](https://pypi.org/project/requests/),
  [`markdown`](https://pypi.org/project/Markdown/)
- `pdftotext` (pacote **poppler**) — extração de texto dos PDFs
- Google Chrome / Chromium — geração de PDF
- Chave da API Mistral (para categorização LLM e resumo)

```bash
pip install requests markdown
cp config/.env.example config/.env   # e preencha MISTRAL_KEY
```

## Pipeline

Todos os comandos rodam a partir da raiz do projeto.

### 1. Buscar + baixar

```bash
python3 src/buscar_gedoc.py 1802019 \
    --baixar data/documentos_1802019 \
    --html   data/index_1802019.html \
    --json   data/resultado_1802019.json
```

Baixa todos os resultados (com paginação), filtra os que contêm o SIAPE no
texto e nomeia os PDFs como `AAAA_NUMERO_ASSUNTO.pdf`.

### 2. Categorizar

As categorias são definidas em [config/categoria.json](config/categoria.json)
(nome + descrição). No modo `llm`, a Mistral classifica cada documento com base
nessas descrições:

```bash
python3 src/categorizar.py \
    --json  data/resultado_1802019.json \
    --pdfs  data/documentos_1802019 \
    --out   data/categorizado_1802019 \
    --md    data/categorizado_1802019.md \
    --modo  llm \
    --cache data/classificacao_1802019.json
```

Copia os PDFs em subpastas por categoria e gera um Markdown com tabela +
listas. `--modo keyword` usa regras por palavra-chave (grátis, sem API).
Editar `categoria.json` muda as categorias sem tocar no código.

### 3. Resumir (agrupado por categoria)

```bash
python3 src/resumir_mistral.py \
    --json          data/resultado_1802019.json \
    --pdfs          data/documentos_1802019 \
    --out           data/resumo_1802019.md \
    --cache         data/resumos_1802019_cache.json \
    --classificacao data/classificacao_1802019.json
```

Extrai o texto de cada PDF, pede um resumo curto à Mistral e gera um Markdown
com **tabela de categorias** + **uma seção por categoria** contendo as
portarias (data, SIAPE, link, arquivo e resumo). Use `--limit 3` para testar.

### 4. Gerar PDF

```bash
python3 src/md_para_pdf.py data/resumo_1802019.md \
    --out data/resumo_1802019.pdf --titulo "Resumo GeDoc - SIAPE 1802019"
```

## Cache e custo

- `categorizar` e `resumir_mistral` mantêm **cache** (por link do documento).
  Reexecuções não repetem chamadas à Mistral — reprocessar é grátis.
- `categorizar` aplica _throttle_ entre chamadas para respeitar o rate limit
  (HTTP 429); há retry com backoff em erros transitórios.

## Privacidade (LGPD)

Os PDFs, resumos e o `data/` contêm **dados pessoais de terceiros** (nomes e
SIAPEs de membros de comissões). Nada em `data/` é versionado. Use apenas para
consulta de documentos públicos do IFES e respeite a legislação.
