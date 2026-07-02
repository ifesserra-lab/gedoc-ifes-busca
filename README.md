# GeDoc IFES — busca e download de documentos

Ferramenta em Python para pesquisar documentos no
[GeDoc do IFES](https://gedoc.ifes.edu.br/faces/pesquisarDocumentos/pesquisarHistorico.xhtml)
por palavra-chave (ex.: um número **SIAPE**), coletar **todos** os resultados
(com paginação), baixar os PDFs e gerar uma página HTML com a lista.

O portal é uma aplicação **JSF/PrimeFaces**: a busca não é um GET simples — exige
`ViewState`, sessão (`jsessionid`) e requisições AJAX parciais. O script cuida de
todo esse fluxo automaticamente.

## O que o script faz

1. **Abre a sessão** (GET inicial) e captura `ViewState` + cookie de sessão.
2. **Submete a busca** (POST AJAX PrimeFaces) com a palavra-chave.
3. **Pagina** o `DataList` (10 por página) até coletar **todos** os registros.
4. **Filtra por SIAPE**: quando o termo é um número SIAPE, mantém apenas os
   documentos cujo **texto contém aquele SIAPE** (um documento pode citar vários
   SIAPEs; basta conter o buscado). Os demais são listados como descartados.
5. **Baixa os PDFs** (opcional) nomeando no padrão `AAAA_NUMERO_ASSUNTO.pdf`.
6. **Gera uma página HTML** (opcional) com título, data e links.

## Requisitos

- Python 3.8+
- [`requests`](https://pypi.org/project/requests/)

```bash
pip install requests
```

## Uso

```bash
# Só listar no terminal
python3 buscar_gedoc.py 1998547

# Baixar todos os PDFs + gerar página + salvar JSON
python3 buscar_gedoc.py 1998547 --baixar documentos --html index.html --json resultado.json
```

### Opções

| Flag | Descrição |
|------|-----------|
| `termo` | palavra-chave / SIAPE (obrigatório) |
| `--repositorio` | `0`=Boletim, `1`=GeDoc (padrão), `2`=Site IFES/Reitoria |
| `--baixar PASTA` | baixa todos os PDFs para a pasta indicada |
| `--html ARQ` | gera página HTML (ex.: `index.html`) |
| `--json ARQ` | salva o resultado estruturado em JSON |

### Padrão de nome dos PDFs

`AAAA_NUMERO_ASSUNTO.pdf`, derivado do título do documento:

- **AAAA** — ano (do título; se ausente, o ano da data de publicação)
- **NUMERO** — número após "Nº"
- **ASSUNTO** — restante do título

Exemplos:

```
2018_344_Autoriza afastamento para capacitação - PAULO SÉRGIO DOS SANTOS JÚNIOR.pdf
2016_9_PAULO SÉRGIO DOS SANTOS JÚNIOR - Afastamento do País.pdf
```

## Privacidade

Os PDFs e o JSON de resultado podem conter **dados pessoais de terceiros**
(nomes e SIAPEs de membros de comissões). Por isso o [.gitignore](.gitignore)
**exclui** `documentos/`, `*.pdf` e `*.json` deste repositório — rode o script
localmente para gerá-los. Não faça commit desses arquivos.

## Estrutura

```
buscar_gedoc.py   # script principal (busca, paginação, filtro, download, HTML)
index.html        # página de exemplo gerada (apenas links ao original)
README.md
.gitignore
```

## Aviso

Uso destinado à consulta de documentos públicos do IFES. Respeite os termos de
uso do portal e a legislação de proteção de dados (LGPD) ao manipular os PDFs.
