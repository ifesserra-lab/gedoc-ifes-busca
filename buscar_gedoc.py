#!/usr/bin/env python3
"""Busca, filtra e baixa documentos do portal GeDoc do IFES.

O portal (https://gedoc.ifes.edu.br) roda em JSF/PrimeFaces: a busca nao e um
GET simples -- exige `ViewState`, cookie de sessao (`jsessionid`) e requisicoes
AJAX parciais. Este modulo encapsula esse fluxo:

  1. Abre a sessao (GET) e descobre dinamicamente os ids do formulario/botao,
     o ViewState e a URL de acao.
  2. Submete a busca (POST AJAX PrimeFaces) com a palavra-chave.
  3. Pagina o DataList ate coletar todos os registros.
  4. (Opcional) filtra por SIAPE, baixa os PDFs e gera uma pagina HTML.

Uso:
    python3 buscar_gedoc.py 1998547
    python3 buscar_gedoc.py 1998547 --baixar documentos --html index.html --json r.json
"""
from __future__ import annotations

import argparse
import html
import json
import logging
import os
import re
import sys
from dataclasses import asdict, dataclass, field
from string import Template
from typing import Dict, List, Optional, Tuple

import requests
from requests.adapters import HTTPAdapter

try:  # local do Retry varia entre versoes do urllib3
    from urllib3.util.retry import Retry
except ImportError:  # pragma: no cover
    from requests.packages.urllib3.util.retry import Retry  # type: ignore

log = logging.getLogger("gedoc")

# --------------------------------------------------------------------------- #
# Constantes
# --------------------------------------------------------------------------- #
BASE = "https://gedoc.ifes.edu.br"
PAGE = "/faces/pesquisarDocumentos/pesquisarHistorico.xhtml"
USER_AGENT = "Mozilla/5.0 (gedoc-busca-script)"

TIMEOUT = 30          # segundos por requisicao
TIMEOUT_DOWNLOAD = 90
ROWS_POR_PAGINA = 10  # o DataList do portal retorna 10 itens por pagina

REPOSITORIOS = {"0": "Boletim", "1": "GeDoc", "2": "Site IFES/Reitoria"}

# regex reutilizadas
_RE_VIEWSTATE = re.compile(
    r'name="javax\.faces\.ViewState"[^>]*value="([^"]+)"')
_RE_BTN_BUSCA = re.compile(
    r"PrimeFaces\.ab\(\{source:'([^']+)'[^)]*panelResultado")
_RE_ERRO = re.compile(r'ui-messages-error-detail">([^<]*)')
_RE_TOTAL = re.compile(r"([\d.]+)\s*registro")
_RE_DOC = re.compile(
    r'<a href="([^"]*?/documento/[0-9A-Fa-f]{32}[^"]*)"[^>]*'
    r'class="resultadoBuscaLinhaAzul">(.*?)</a>', re.S)
_RE_DATA = re.compile(r'resultadoBuscaLinhaVerde">\s*(\d{2}/\d{2}/\d{4})')
_RE_HIGHLIGHT = re.compile(r'class="highlight">(.*?)</div>', re.S)
_RE_SIAPE = re.compile(r"SIAPE\s*(?:n?º?\s*|:\s*)?([0-9]{5,8})", re.I)
_RE_TAGS = re.compile(r"<[^>]+>")
_RE_ESPACOS = re.compile(r"\s+")
_RE_NUM_TITULO = re.compile(r"N[ºo°]\s*(\d+)", re.I)
_RE_ANO = re.compile(r"\b(?:19|20)\d{2}\b")
_RE_CHARS_ILEGAIS = re.compile(r'[/\\:*?"<>|\n\r\t]')


class GedocError(RuntimeError):
    """Erro de negocio do fluxo GeDoc (layout mudou, sessao expirou, etc.)."""


# --------------------------------------------------------------------------- #
# Modelo
# --------------------------------------------------------------------------- #
@dataclass
class Documento:
    titulo: str
    link: str
    data: str = ""
    trecho: str = ""
    siapes: List[str] = field(default_factory=list)
    arquivo: Optional[str] = None
    contem_siape: bool = True

    @property
    def outros_siapes(self) -> List[str]:
        """SIAPEs citados no trecho, sem duplicatas."""
        return list(dict.fromkeys(self.siapes))


# --------------------------------------------------------------------------- #
# Utilitarios de texto
# --------------------------------------------------------------------------- #
def _texto(fragmento: str) -> str:
    """Remove tags HTML e normaliza espacos."""
    return _RE_ESPACOS.sub(" ", html.unescape(_RE_TAGS.sub(" ", fragmento))).strip()


def _esc(valor: object) -> str:
    """Escapa um valor para insercao segura em HTML."""
    return html.escape(str(valor), quote=True)


def _https(url: str) -> str:
    """Normaliza a URL do documento para HTTPS sem a porta 80."""
    return url.replace("http://gedoc.ifes.edu.br:80", "https://gedoc.ifes.edu.br")


# --------------------------------------------------------------------------- #
# Cliente HTTP do portal
# --------------------------------------------------------------------------- #
class GedocClient:
    """Encapsula sessao, ViewState e ids dinamicos do portal GeDoc."""

    def __init__(self, timeout: int = TIMEOUT, retries: int = 3) -> None:
        self.timeout = timeout
        self.sess = self._criar_sessao(retries)
        self.form = ""
        self.btn = ""
        self.datalist = ""
        self.action = PAGE
        self.viewstate = ""

    @staticmethod
    def _criar_sessao(retries: int) -> requests.Session:
        sess = requests.Session()
        sess.headers["User-Agent"] = USER_AGENT
        politica = Retry(
            total=retries,
            backoff_factor=0.5,
            status_forcelist=(429, 500, 502, 503, 504),
            allowed_methods=frozenset({"GET", "POST"}),
        )
        adapter = HTTPAdapter(max_retries=politica)
        sess.mount("https://", adapter)
        sess.mount("http://", adapter)
        return sess

    def abrir(self) -> None:
        """GET inicial: descobre ids do formulario, ViewState e URL de acao."""
        resp = self.sess.get(BASE + PAGE, timeout=self.timeout)
        resp.raise_for_status()
        page = resp.text

        m_vs = _RE_VIEWSTATE.search(page)
        m_btn = _RE_BTN_BUSCA.search(page)
        if not m_vs or not m_btn:
            raise GedocError(
                "Nao localizei ViewState/botao de busca -- o layout do portal "
                "pode ter mudado.")

        self.viewstate = m_vs.group(1)
        self.btn = m_btn.group(1)                 # ex.: j_idt65:j_idt115
        self.form = self.btn.rsplit(":", 1)[0]    # ex.: j_idt65
        self.datalist = f"{self.form}:dataList"

        m_ac = re.search(
            rf'<form\b[^>]*id="{re.escape(self.form)}"[^>]*action="([^"]+)"', page)
        self.action = m_ac.group(1) if m_ac else PAGE
        log.debug("ids: form=%s btn=%s action=%s", self.form, self.btn, self.action)

    def _post(self, data: Dict[str, str]) -> str:
        resp = self.sess.post(
            BASE + self.action,
            data=data,
            headers={
                "Faces-Request": "partial/ajax",
                "X-Requested-With": "XMLHttpRequest",
                "Content-Type": "application/x-www-form-urlencoded; charset=UTF-8",
            },
            timeout=self.timeout,
        )
        resp.raise_for_status()
        xml = resp.text
        self._atualizar_viewstate(xml)
        return xml

    def _atualizar_viewstate(self, xml: str) -> None:
        """Mantem o ViewState em dia com a ultima resposta parcial."""
        m = re.search(r"ViewState[^>]*><!\[CDATA\[([^\]]+)\]\]>", xml)
        if m:
            self.viewstate = m.group(1)

    def buscar(self, termo: str, repositorio: str = "1",
               campo: str = "RELEVANCIA", ordem: str = "DECRESCENTE") -> str:
        """Submete a busca e retorna o XML da resposta parcial."""
        data = {
            "javax.faces.partial.ajax": "true",
            "javax.faces.source": self.btn,
            "javax.faces.partial.execute": self.form,
            "javax.faces.partial.render": f"{self.form}:panelResultado "
                                          f"{self.form}:messages",
            self.btn: self.btn,
            self.form: self.form,
            "javax.faces.ViewState": self.viewstate,
        }
        data.update(self._campos_form(termo, repositorio, campo, ordem))
        return self._post(data)

    def _campos_form(self, termo: str, repositorio: str,
                     campo: str, ordem: str) -> Dict[str, str]:
        """Monta os campos do formulario (SelectOneMenu usa _focus + _input)."""
        selecionados = {"campo": campo, "ordem": ordem, "shardItems": repositorio}
        campos: Dict[str, str] = {f"{self.form}:nome": termo}
        for nome in ("mes", "ano", "campo", "campus", "ordem", "shardItems"):
            campos[f"{self.form}:{nome}_focus"] = ""
            campos[f"{self.form}:{nome}_input"] = selecionados.get(nome, "")
        return campos

    def pagina(self, first: int, rows: int = ROWS_POR_PAGINA) -> str:
        """Navega o DataList para o offset `first` e retorna o XML."""
        data = {
            "javax.faces.partial.ajax": "true",
            "javax.faces.source": self.datalist,
            "javax.faces.partial.execute": self.datalist,
            "javax.faces.partial.render": self.datalist,
            f"{self.datalist}_pagination": "true",
            f"{self.datalist}_first": str(first),
            f"{self.datalist}_rows": str(rows),
            f"{self.datalist}_encodeFeature": "true",
            self.form: self.form,
            "javax.faces.ViewState": self.viewstate,
        }
        return self._post(data)

    def coletar(self, termo: str, repositorio: str = "1") -> Tuple[int, List[Documento]]:
        """Executa a busca completa (com paginacao) e retorna (total, docs)."""
        xml = self.buscar(termo, repositorio)
        total, docs = parse_resposta(xml)
        if total is None:
            total = len(docs)

        vistos = {d.link for d in docs}
        first = ROWS_POR_PAGINA
        while len(docs) < total:
            _, mais = parse_resposta(self.pagina(first))
            novos = [d for d in mais if d.link not in vistos]
            if not novos:
                break
            vistos.update(d.link for d in novos)
            docs.extend(novos)
            first += ROWS_POR_PAGINA
        return total, docs


# --------------------------------------------------------------------------- #
# Parsing da resposta
# --------------------------------------------------------------------------- #
def parse_resposta(xml: str) -> Tuple[Optional[int], List[Documento]]:
    """Extrai (total_de_registros, [Documento]) de uma resposta parcial JSF."""
    erros = _RE_ERRO.findall(xml)
    if erros:
        raise GedocError("Erro do servidor: " + " / ".join(erros))
    if "<redirect" in xml:
        raise GedocError("Sessao expirada (redirect). Refaca a busca.")

    m_total = _RE_TOTAL.search(xml)
    total = int(m_total.group(1).replace(".", "")) if m_total else None

    anchors = list(_RE_DOC.finditer(xml))
    docs: List[Documento] = []
    for i, a in enumerate(anchors):
        fim = anchors[i + 1].start() if i + 1 < len(anchors) else len(xml)
        bloco = xml[a.end():fim]

        m_data = _RE_DATA.search(bloco)
        m_hl = _RE_HIGHLIGHT.search(bloco)
        trecho = _texto(m_hl.group(1)) if m_hl else ""

        docs.append(Documento(
            titulo=_texto(a.group(2)),
            link=a.group(1),
            data=m_data.group(1) if m_data else "",
            trecho=trecho,
            siapes=_RE_SIAPE.findall(trecho),
        ))

    # dedup preservando ordem
    vistos, unicos = set(), []
    for d in docs:
        if d.link not in vistos:
            vistos.add(d.link)
            unicos.append(d)
    return total, unicos


# --------------------------------------------------------------------------- #
# Filtro por SIAPE
# --------------------------------------------------------------------------- #
def eh_siape(termo: str) -> bool:
    return termo.isdigit() and 5 <= len(termo) <= 8


def filtrar_por_siape(docs: List[Documento], termo: str) -> None:
    """Marca cada doc com `contem_siape`.

    Um documento contem o SIAPE se o numero aparece no trecho do texto --
    rotulado como "SIAPE NNNN" ou nao, pois o snippet pode cortar a palavra
    "SIAPE" fora da janela exibida.
    """
    procura_siape = eh_siape(termo)
    for d in docs:
        d.contem_siape = (
            (termo in d.siapes or termo in d.trecho) if procura_siape else True
        )


# --------------------------------------------------------------------------- #
# Nome de arquivo YYYY_NUMERO_ASSUNTO
# --------------------------------------------------------------------------- #
def nome_arquivo(doc: Documento) -> str:
    """Deriva o nome do PDF no formato AAAA_NUMERO_ASSUNTO.pdf a partir do titulo."""
    titulo = doc.titulo

    m_num = _RE_NUM_TITULO.search(titulo)
    numero = m_num.group(1) if m_num else "0"

    m_ano = _RE_ANO.search(titulo)
    if m_ano:
        ano = m_ano.group(0)
    else:
        m_data = re.search(r"(\d{4})\s*$", doc.data)
        ano = m_data.group(1) if m_data else "0000"

    # assunto = titulo sem o prefixo "TIPO Nº NUM - AAAA -"
    assunto = re.sub(r"^\s*[A-Za-zÀ-ÿ]+\s*", "", titulo)              # tipo
    assunto = _RE_NUM_TITULO.sub("", assunto, count=1)               # Nº NUM
    assunto = re.sub(r"^\s*-?\s*(?:19|20)\d{2}\s*-?\s*", "", assunto)  # ano
    assunto = assunto.strip(" -–—")

    nome = _RE_CHARS_ILEGAIS.sub("_", f"{ano}_{numero}_{assunto}")
    nome = _RE_ESPACOS.sub(" ", nome).strip().strip(".")
    return nome[:180] + ".pdf"


def _nome_unico(nome: str, usados: set) -> str:
    """Evita sobrescrever arquivos com o mesmo nome derivado."""
    if nome not in usados:
        usados.add(nome)
        return nome
    base, ext = os.path.splitext(nome)
    i = 2
    while f"{base} ({i}){ext}" in usados:
        i += 1
    unico = f"{base} ({i}){ext}"
    usados.add(unico)
    return unico


# --------------------------------------------------------------------------- #
# Saidas: download, HTML, JSON
# --------------------------------------------------------------------------- #
def baixar(client: GedocClient, docs: List[Documento], pasta: str) -> None:
    """Baixa os PDFs para `pasta`, preenchendo `doc.arquivo`."""
    os.makedirs(pasta, exist_ok=True)
    usados: set = set()
    total = len(docs)
    for i, d in enumerate(docs, 1):
        resp = client.sess.get(_https(d.link), timeout=TIMEOUT_DOWNLOAD)
        resp.raise_for_status()
        d.arquivo = _nome_unico(nome_arquivo(d), usados)
        with open(os.path.join(pasta, d.arquivo), "wb") as f:
            f.write(resp.content)
        log.info("baixado %2d/%d (%d KB) %s",
                 i, total, len(resp.content) // 1024, d.arquivo)


_HTML_TMPL = Template("""<!doctype html>
<html lang="pt-BR">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>GeDoc IFES - documentos SIAPE $termo</title>
<style>
  :root { --azul:#0b5cad; --verde:#1f7a3d; --bg:#f4f6f9; --card:#fff; --linha:#e5e9ef; }
  * { box-sizing:border-box; }
  body { margin:0; font-family:system-ui,Segoe UI,Roboto,Arial,sans-serif; background:var(--bg); color:#1a2330; }
  header { background:var(--azul); color:#fff; padding:24px 20px; }
  header h1 { margin:0 0 4px; font-size:20px; }
  header p { margin:0; opacity:.9; font-size:14px; }
  .wrap { max-width:1000px; margin:20px auto; padding:0 16px; }
  .resumo { display:flex; gap:16px; flex-wrap:wrap; margin-bottom:16px; }
  .card { background:var(--card); border:1px solid var(--linha); border-radius:10px; padding:14px 18px; }
  .card b { font-size:22px; color:var(--azul); }
  table { width:100%; border-collapse:collapse; background:var(--card); border:1px solid var(--linha); border-radius:10px; overflow:hidden; }
  th,td { padding:12px 14px; text-align:left; border-bottom:1px solid var(--linha); vertical-align:top; font-size:14px; }
  th { background:#eef2f7; font-size:12px; text-transform:uppercase; letter-spacing:.03em; color:#5a6474; }
  tr:last-child td { border-bottom:none; }
  .num { color:#9aa4b2; width:34px; }
  .titulo { font-weight:600; }
  .siapes { color:#5a6474; font-size:12px; margin-top:3px; }
  .data { white-space:nowrap; color:#5a6474; }
  .acoes { white-space:nowrap; }
  .btn { display:inline-block; margin:2px 0 2px 6px; padding:6px 10px; border-radius:6px; background:var(--verde); color:#fff; text-decoration:none; font-size:12px; font-weight:600; }
  .btn.ghost { background:transparent; color:var(--azul); border:1px solid var(--azul); }
  footer { text-align:center; color:#8a95a5; font-size:12px; margin:24px 0; }
</style>
</head>
<body>
<header>
  <h1>Documentos GeDoc IFES</h1>
  <p>Busca por SIAPE <b>$termo</b> &mdash; documentos que contem esse SIAPE no texto</p>
</header>
<div class="wrap">
  <div class="resumo">
    <div class="card"><b>$n</b><br>documentos</div>
    <div class="card"><b>$total</b><br>registros na busca</div>
  </div>
  <table>
    <thead><tr><th>#</th><th>Documento</th><th>Data</th><th>Acoes</th></tr></thead>
    <tbody>
$linhas
    </tbody>
  </table>
  <footer>Gerado por buscar_gedoc.py &middot; fonte: gedoc.ifes.edu.br</footer>
</div>
</body>
</html>""")


def _linha_html(i: int, d: Documento, termo: str, pasta_pdf: str) -> str:
    pdf_local = ""
    if d.arquivo:
        href = _esc(f"{pasta_pdf}/{d.arquivo}")
        pdf_local = f'<a class="btn" href="{href}" target="_blank">PDF baixado</a>'
    return (
        "      <tr>\n"
        f'        <td class="num">{i}</td>\n'
        "        <td>\n"
        f'          <div class="titulo">{_esc(d.titulo)}</div>\n'
        f'          <div class="siapes">SIAPE: <b>{_esc(termo)}</b></div>\n'
        "        </td>\n"
        f'        <td class="data">{_esc(d.data)}</td>\n'
        '        <td class="acoes">\n'
        f"          {pdf_local}\n"
        f'          <a class="btn ghost" href="{_esc(_https(d.link))}" '
        'target="_blank">Original</a>\n'
        "        </td>\n"
        "      </tr>"
    )


def gerar_html(docs: List[Documento], termo: str, total: int,
               arquivo: str, pasta_pdf: str) -> None:
    """Gera a pagina HTML com a lista de documentos."""
    linhas = "\n".join(_linha_html(i, d, termo, pasta_pdf)
                       for i, d in enumerate(docs, 1))
    pagina = _HTML_TMPL.substitute(
        termo=_esc(termo), n=len(docs), total=total, linhas=linhas)
    with open(arquivo, "w", encoding="utf-8") as f:
        f.write(pagina)


def salvar_json(docs: List[Documento], fora: List[Documento],
                termo: str, total: int, arquivo: str) -> None:
    payload = {
        "termo": termo,
        "total_bruto": total,
        "total_com_siape": len(docs),
        "documentos": [asdict(d) for d in docs],
        "descartados": [asdict(d) for d in fora],
    }
    with open(arquivo, "w", encoding="utf-8") as f:
        json.dump(payload, f, ensure_ascii=False, indent=2)


# --------------------------------------------------------------------------- #
# Apresentacao no terminal
# --------------------------------------------------------------------------- #
def imprimir_resultado(validos: List[Documento], fora: List[Documento],
                       termo: str, total: int) -> None:
    print(f"Resultado da pesquisa para '{termo}': {total} registro(s) brutos")
    if eh_siape(termo):
        print(f"Contendo o SIAPE {termo}: {len(validos)} | "
              f"sem esse SIAPE no texto: {len(fora)}")
    print()

    for i, d in enumerate(validos, 1):
        outros = [s for s in d.outros_siapes if s != termo]
        extra = f"  (tambem: {', '.join(outros)})" if outros else ""
        print(f"{i:2d}. {d.titulo}")
        print(f"    data: {d.data} | SIAPE no texto: {termo}{extra}")
        print(f"    {d.link}")

    if fora:
        print(f"\n--- {len(fora)} descartado(s): '{termo}' aparece mas NAO no "
              f"trecho do texto ---")
        for d in fora:
            sp = ", ".join(d.outros_siapes) or "nenhum"
            print(f" x  {d.titulo}  [SIAPE no texto: {sp}]")


# --------------------------------------------------------------------------- #
# CLI
# --------------------------------------------------------------------------- #
def parse_args(argv: Optional[List[str]] = None) -> argparse.Namespace:
    ap = argparse.ArgumentParser(description="Busca documentos no GeDoc IFES.")
    ap.add_argument("termo", help="palavra-chave (ex: 1998547)")
    ap.add_argument("--repositorio", default="1", choices=sorted(REPOSITORIOS),
                    help="0=Boletim 1=GeDoc 2=Site (padrao 1)")
    ap.add_argument("--json", metavar="ARQ", help="salva resultado em arquivo JSON")
    ap.add_argument("--html", metavar="ARQ", help="gera pagina HTML (ex: index.html)")
    ap.add_argument("--baixar", metavar="PASTA",
                    help="baixa todos os PDFs para a pasta indicada")
    ap.add_argument("-q", "--quiet", action="store_true",
                    help="silencia mensagens de progresso")
    return ap.parse_args(argv)


def main(argv: Optional[List[str]] = None) -> int:
    args = parse_args(argv)
    logging.basicConfig(
        level=logging.WARNING if args.quiet else logging.INFO,
        format="%(message)s", stream=sys.stderr)

    termo = args.termo.strip()
    try:
        client = GedocClient()
        client.abrir()
        total, docs = client.coletar(termo, args.repositorio)

        filtrar_por_siape(docs, termo)
        validos = [d for d in docs if d.contem_siape]
        fora = [d for d in docs if not d.contem_siape]

        imprimir_resultado(validos, fora, termo, total)

        if args.baixar:
            log.info("\nBaixando %d PDF(s) em '%s/'...", len(validos), args.baixar)
            baixar(client, validos, args.baixar)
        if args.html:
            gerar_html(validos, termo, total, args.html, args.baixar or "documentos")
            log.info("\nPagina gerada: %s", args.html)
        if args.json:
            salvar_json(validos, fora, termo, total, args.json)
            log.info("Salvo em %s", args.json)
    except GedocError as e:
        log.error("Erro: %s", e)
        return 1
    except requests.RequestException as e:
        log.error("Falha de rede: %s", e)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
