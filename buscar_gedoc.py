#!/usr/bin/env python3
"""
Busca documentos no GeDoc IFES por palavra-chave e lista todos os resultados.

Site: https://gedoc.ifes.edu.br/faces/pesquisarDocumentos/pesquisarHistorico.xhtml
Backend JSF/PrimeFaces (requer ViewState + sessao). Este script:
  1. Abre a sessao (GET) e captura ViewState + cookie jsessionid
  2. Submete a busca (POST AJAX PrimeFaces) com a palavra-chave
  3. Pagina o DataList ate coletar todos os registros
  4. Imprime titulo, fonte/data, trecho e link de cada documento

Uso:
    python3 buscar_gedoc.py 1998547
    python3 buscar_gedoc.py "1998547" --repositorio 1 --json saida.json
"""
import argparse
import html
import json
import os
import re

import requests

BASE = "https://gedoc.ifes.edu.br"
PAGE = "/faces/pesquisarDocumentos/pesquisarHistorico.xhtml"
FORM = "j_idt65"                 # id do <form>
BTN = f"{FORM}:j_idt115"        # botao Pesquisar
DATALIST = f"{FORM}:dataList"   # lista de resultados


def novo_viewstate(sess):
    """GET inicial: retorna (action_url, viewstate)."""
    r = sess.get(BASE + PAGE, timeout=30)
    r.raise_for_status()
    html_txt = r.text
    m_vs = re.search(r'name="javax\.faces\.ViewState"[^>]*value="([^"]+)"', html_txt)
    m_ac = re.search(r'<form[^>]*action="([^"]+)"', html_txt)
    if not m_vs or not m_ac:
        raise RuntimeError("Nao encontrei ViewState/action na pagina inicial.")
    return m_ac.group(1), m_vs.group(1)


def _post_ajax(sess, action, data):
    r = sess.post(
        BASE + action,
        data=data,
        headers={
            "Faces-Request": "partial/ajax",
            "X-Requested-With": "XMLHttpRequest",
            "Content-Type": "application/x-www-form-urlencoded; charset=UTF-8",
        },
        timeout=30,
    )
    r.raise_for_status()
    return r.text


def buscar(sess, action, viewstate, termo, repositorio="1",
           campo="RELEVANCIA", ordem="DECRESCENTE"):
    """Submete a busca. Retorna XML da resposta parcial."""
    data = {
        "javax.faces.partial.ajax": "true",
        "javax.faces.source": BTN,
        "javax.faces.partial.execute": FORM,
        "javax.faces.partial.render": f"{FORM}:panelResultado {FORM}:messages",
        BTN: BTN,
        FORM: FORM,
        f"{FORM}:nome": termo,
        f"{FORM}:mes_focus": "", f"{FORM}:mes_input": "",
        f"{FORM}:ano_focus": "", f"{FORM}:ano_input": "",
        f"{FORM}:campo_focus": "", f"{FORM}:campo_input": campo,
        f"{FORM}:campus_focus": "", f"{FORM}:campus_input": "",
        f"{FORM}:ordem_focus": "", f"{FORM}:ordem_input": ordem,
        f"{FORM}:shardItems_focus": "", f"{FORM}:shardItems_input": repositorio,
        "javax.faces.ViewState": viewstate,
    }
    return _post_ajax(sess, action, data)


def pagina(sess, action, viewstate, first, rows=10):
    """Navega o DataList para o offset 'first'. Retorna XML."""
    data = {
        "javax.faces.partial.ajax": "true",
        "javax.faces.source": DATALIST,
        "javax.faces.partial.execute": DATALIST,
        "javax.faces.partial.render": DATALIST,
        f"{DATALIST}_pagination": "true",
        f"{DATALIST}_first": str(first),
        f"{DATALIST}_rows": str(rows),
        f"{DATALIST}_encodeFeature": "true",
        FORM: FORM,
        "javax.faces.ViewState": viewstate,
    }
    return _post_ajax(sess, action, data)


def _texto(s):
    """Remove tags e normaliza espacos."""
    return re.sub(r"\s+", " ", html.unescape(re.sub("<[^>]+>", " ", s))).strip()


def _siapes(trecho):
    """Extrai todos os numeros de SIAPE mencionados no trecho."""
    # captura o numero logo apos a palavra SIAPE (tolera 'nº', ':', tags <b>)
    return re.findall(r"SIAPE\s*(?:n?º?\s*|:\s*)?([0-9]{5,8})", trecho, re.I)


def extrair(xml):
    """Extrai (total, [docs]) de uma resposta parcial JSF.

    Cada doc traz: titulo, link, data, trecho, e a lista de SIAPEs
    encontrados no texto do trecho.
    """
    erro = re.findall(r'ui-messages-error-detail">([^<]*)', xml)
    if erro:
        raise RuntimeError("Erro do servidor: " + " / ".join(erro))
    if "<redirect" in xml:
        raise RuntimeError("Sessao expirada (redirect). Refaca a busca.")

    total = None
    mt = re.search(r"([\d.]+)\s*registro", xml)
    if mt:
        total = int(mt.group(1).replace(".", ""))

    # localiza o inicio de cada documento (link azul do titulo)
    anchors = list(re.finditer(
        r'<a href="([^"]*?/documento/[0-9A-Fa-f]{32}[^"]*)"[^>]*'
        r'class="resultadoBuscaLinhaAzul">(.*?)</a>', xml, re.S))

    docs = []
    for i, a in enumerate(anchors):
        link = a.group(1)
        titulo = _texto(a.group(2))
        # bloco = do fim deste anchor ate o proximo anchor
        ini = a.end()
        fim = anchors[i + 1].start() if i + 1 < len(anchors) else len(xml)
        bloco = xml[ini:fim]

        md = re.search(r'resultadoBuscaLinhaVerde">\s*(\d{2}/\d{2}/\d{4})', bloco)
        data = md.group(1) if md else ""

        mh = re.search(r'class="highlight">(.*?)</div>', bloco, re.S)
        trecho = _texto(mh.group(1)) if mh else ""

        docs.append({
            "titulo": titulo,
            "link": link,
            "data": data,
            "trecho": trecho,
            "siapes": _siapes(trecho),
        })

    # dedup preservando ordem
    vistos, out = set(), []
    for d in docs:
        if d["link"] not in vistos:
            vistos.add(d["link"])
            out.append(d)
    return total, out


def _https(url):
    """Normaliza URL do documento para HTTPS sem porta 80."""
    return url.replace("http://gedoc.ifes.edu.br:80", "https://gedoc.ifes.edu.br")


def _nome_arquivo(doc):
    """Nome do PDF no formato YYYY_NUMERO_ASSUNTO.pdf, derivado do titulo."""
    titulo = doc["titulo"]

    # numero: primeiro numero apos "Nº"/"nº"
    mnum = re.search(r"N[ºo°]\s*(\d+)", titulo, re.I)
    numero = mnum.group(1) if mnum else "0"

    # ano: primeiro 19xx/20xx do titulo; senao, o ano da data (dd/mm/yyyy)
    mano = re.search(r"\b(19|20)\d{2}\b", titulo)
    if mano:
        ano = mano.group(0)
    else:
        md = re.search(r"(\d{4})\s*$", doc.get("data", ""))
        ano = md.group(1) if md else "0000"

    # assunto: remove prefixo "TIPO Nº NUM - YYYY -"
    assunto = re.sub(r"^\s*[A-Za-zÀ-ÿ]+\s*", "", titulo)          # tipo (PORTARIA/Despacho)
    assunto = re.sub(r"^\s*N[ºo°]\s*\d+\s*-?\s*", "", assunto, flags=re.I)  # Nº NUM
    assunto = re.sub(r"^\s*(19|20)\d{2}\s*-?\s*", "", assunto)    # ano
    assunto = assunto.strip(" -–—")

    nome = f"{ano}_{numero}_{assunto}"
    nome = re.sub(r'[/\\:*?"<>|\n\r\t]', "_", nome)               # chars ilegais
    nome = re.sub(r"\s+", " ", nome).strip().strip(".")
    return nome[:180] + ".pdf"


def baixar(sess, docs, pasta):
    """Baixa todos os PDFs para 'pasta'. Preenche d['arquivo']."""
    os.makedirs(pasta, exist_ok=True)
    for i, d in enumerate(docs, 1):
        url = _https(d["link"])
        r = sess.get(url, timeout=90)
        r.raise_for_status()
        nome = _nome_arquivo(d)
        with open(os.path.join(pasta, nome), "wb") as f:
            f.write(r.content)
        d["arquivo"] = nome
        print(f"  baixado {i:2d}/{len(docs)} ({len(r.content)//1024} KB) {nome}")


def gerar_html(docs, termo, total, arquivo, pasta_pdf):
    """Gera pagina HTML listando os documentos."""
    esc = lambda s: html.escape(str(s), quote=True)
    linhas = []
    for i, d in enumerate(docs, 1):
        pdf_local = ""
        if d.get("arquivo"):
            href = esc(f"{pasta_pdf}/{d['arquivo']}")
            pdf_local = f'<a class="btn" href="{href}" target="_blank">PDF baixado</a>'
        linhas.append(f"""      <tr>
        <td class="num">{i}</td>
        <td>
          <div class="titulo">{esc(d['titulo'])}</div>
          <div class="siapes">SIAPE: <b>{esc(termo)}</b></div>
        </td>
        <td class="data">{esc(d.get('data',''))}</td>
        <td class="acoes">
          {pdf_local}
          <a class="btn ghost" href="{esc(_https(d['link']))}" target="_blank">Original</a>
        </td>
      </tr>""")
    tabela = "\n".join(linhas)
    doc = f"""<!doctype html>
<html lang="pt-BR">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>GeDoc IFES - documentos SIAPE {esc(termo)}</title>
<style>
  :root {{ --azul:#0b5cad; --verde:#1f7a3d; --bg:#f4f6f9; --card:#fff; --linha:#e5e9ef; }}
  * {{ box-sizing:border-box; }}
  body {{ margin:0; font-family:system-ui,Segoe UI,Roboto,Arial,sans-serif; background:var(--bg); color:#1a2330; }}
  header {{ background:var(--azul); color:#fff; padding:24px 20px; }}
  header h1 {{ margin:0 0 4px; font-size:20px; }}
  header p {{ margin:0; opacity:.9; font-size:14px; }}
  .wrap {{ max-width:1000px; margin:20px auto; padding:0 16px; }}
  .resumo {{ display:flex; gap:16px; flex-wrap:wrap; margin-bottom:16px; }}
  .card {{ background:var(--card); border:1px solid var(--linha); border-radius:10px; padding:14px 18px; }}
  .card b {{ font-size:22px; color:var(--azul); }}
  table {{ width:100%; border-collapse:collapse; background:var(--card); border:1px solid var(--linha); border-radius:10px; overflow:hidden; }}
  th,td {{ padding:12px 14px; text-align:left; border-bottom:1px solid var(--linha); vertical-align:top; font-size:14px; }}
  th {{ background:#eef2f7; font-size:12px; text-transform:uppercase; letter-spacing:.03em; color:#5a6474; }}
  tr:last-child td {{ border-bottom:none; }}
  .num {{ color:#9aa4b2; width:34px; }}
  .titulo {{ font-weight:600; }}
  .siapes {{ color:#5a6474; font-size:12px; margin-top:3px; }}
  .data {{ white-space:nowrap; color:#5a6474; }}
  .acoes {{ white-space:nowrap; }}
  .btn {{ display:inline-block; margin:2px 0 2px 6px; padding:6px 10px; border-radius:6px; background:var(--verde); color:#fff; text-decoration:none; font-size:12px; font-weight:600; }}
  .btn.ghost {{ background:transparent; color:var(--azul); border:1px solid var(--azul); }}
  footer {{ text-align:center; color:#8a95a5; font-size:12px; margin:24px 0; }}
</style>
</head>
<body>
<header>
  <h1>Documentos GeDoc IFES</h1>
  <p>Busca por SIAPE <b>{esc(termo)}</b> &mdash; documentos que contem esse SIAPE no texto</p>
</header>
<div class="wrap">
  <div class="resumo">
    <div class="card"><b>{len(docs)}</b><br>documentos</div>
    <div class="card"><b>{total}</b><br>registros na busca</div>
  </div>
  <table>
    <thead><tr><th>#</th><th>Documento</th><th>Data</th><th>Acoes</th></tr></thead>
    <tbody>
{tabela}
    </tbody>
  </table>
  <footer>Gerado por buscar_gedoc.py &middot; fonte: gedoc.ifes.edu.br</footer>
</div>
</body>
</html>"""
    with open(arquivo, "w", encoding="utf-8") as f:
        f.write(doc)


def main():
    ap = argparse.ArgumentParser(description="Busca documentos no GeDoc IFES.")
    ap.add_argument("termo", help="palavra-chave (ex: 1998547)")
    ap.add_argument("--repositorio", default="1",
                    help="0=Boletim 1=GeDoc 2=Site (padrao 1)")
    ap.add_argument("--json", help="salva resultado em arquivo JSON")
    ap.add_argument("--html", help="gera pagina HTML (ex: index.html)")
    ap.add_argument("--baixar", metavar="PASTA",
                    help="baixa todos os PDFs para a pasta indicada")
    args = ap.parse_args()

    sess = requests.Session()
    sess.headers["User-Agent"] = "Mozilla/5.0 (gedoc-busca-script)"

    action, viewstate = novo_viewstate(sess)
    xml = buscar(sess, action, viewstate, args.termo, args.repositorio)
    total, docs = extrair(xml)

    if total is None:
        total = len(docs)

    # pagina o restante (10 por pagina)
    first = 10
    while len(docs) < total:
        xml = pagina(sess, action, viewstate, first)
        _, mais = extrair(xml)
        if not mais:
            break
        for d in mais:
            if d["link"] not in {x["link"] for x in docs}:
                docs.append(d)
        first += 10

    # regra: o documento DEVE conter o SIAPE informado no texto.
    # (pode ter outros SIAPEs tambem; basta conter o buscado.)
    termo = args.termo.strip()
    e_siape = termo.isdigit() and 5 <= len(termo) <= 8
    for d in docs:
        d["contem_siape"] = (termo in d["siapes"]) if e_siape else True

    validos = [d for d in docs if d["contem_siape"]]
    fora = [d for d in docs if not d["contem_siape"]]

    print(f"Resultado da pesquisa para '{termo}': {total} registro(s) brutos")
    if e_siape:
        print(f"Contendo o SIAPE {termo}: {len(validos)} | "
              f"sem esse SIAPE no texto: {len(fora)}\n")
    else:
        print()

    for i, d in enumerate(validos, 1):
        outros = [s for s in dict.fromkeys(d["siapes"]) if s != termo]
        extra = f"  (tambem: {', '.join(outros)})" if outros else ""
        print(f"{i:2d}. {d['titulo']}")
        print(f"    data: {d['data']} | SIAPE no texto: {termo}{extra}")
        print(f"    {d['link']}")

    if fora:
        print(f"\n--- {len(fora)} descartado(s): '{termo}' aparece mas NAO como SIAPE "
              f"(SIAPE do texto e outro) ---")
        for d in fora:
            sp = ", ".join(dict.fromkeys(d["siapes"])) or "nenhum"
            print(f" x  {d['titulo']}  [SIAPE no texto: {sp}]")

    if args.baixar:
        print(f"\nBaixando {len(validos)} PDF(s) em '{args.baixar}/'...")
        baixar(sess, validos, args.baixar)

    if args.html:
        pasta_pdf = args.baixar or "documentos"
        gerar_html(validos, termo, total, args.html, pasta_pdf)
        print(f"\nPagina gerada: {args.html}")

    if args.json:
        with open(args.json, "w", encoding="utf-8") as f:
            json.dump({"termo": termo, "total_bruto": total,
                       "total_com_siape": len(validos),
                       "documentos": validos, "descartados": fora},
                      f, ensure_ascii=False, indent=2)
        print(f"\nSalvo em {args.json}")


if __name__ == "__main__":
    main()
