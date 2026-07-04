#!/usr/bin/env python3
"""Sistema completo GeDoc IFES (web).

Junta toda a pipeline por tras de uma UI web, sem dependencias externas
(usa http.server da stdlib e reaproveita os modulos do projeto):

  - Busca por SIAPE -> download -> classificacao (LLM) -> resumo -> PDF
  - CRUD de categorias (config/categoria.json)
  - Download do PDF do resumo e ZIP dos documentos

Uso:
    python3 src/app.py            # http://127.0.0.1:8000
    python3 src/app.py --port 9000
"""
from __future__ import annotations

import argparse
import json
import os
import re
import webbrowser
import zipfile
from dataclasses import asdict
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse

import mistral_client as mc
import md_para_pdf
import resumir_mistral
from buscar_gedoc import GedocClient, baixar, filtrar_por_siape
from categorizar import carregar_categorias, classificar_docs

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "data")
CONFIG = os.path.join(ROOT, "config")
PROTO = os.path.join(ROOT, "prototipo")
CATEGORIA_JSON = os.path.join(CONFIG, "categoria.json")

_SIAPE_RE = re.compile(r"^\d{5,8}$")


# --------------------------------------------------------------------------- #
# Categorias (CRUD)
# --------------------------------------------------------------------------- #
def ler_categorias() -> list:
    if not os.path.exists(CATEGORIA_JSON):
        return []
    with open(CATEGORIA_JSON, encoding="utf-8") as f:
        d = json.load(f)
    return d.get("categorias", []) if isinstance(d, dict) else d


def gravar_categorias(payload) -> list:
    if isinstance(payload, dict):
        payload = payload.get("categorias", [])
    if not isinstance(payload, list):
        raise ValueError("esperado lista de categorias")
    limpo = []
    for it in payload:
        nome = str(it.get("nome", "")).strip()
        if not nome:
            raise ValueError("categoria sem nome")
        limpo.append({"nome": nome, "descricao": str(it.get("descricao", "")).strip()})
    os.makedirs(CONFIG, exist_ok=True)
    with open(CATEGORIA_JSON, "w", encoding="utf-8") as f:
        json.dump({"categorias": limpo}, f, ensure_ascii=False, indent=2)
        f.write("\n")
    return limpo


# --------------------------------------------------------------------------- #
# Pipeline completa
# --------------------------------------------------------------------------- #
def _paths(siape: str) -> dict:
    return {
        "pasta": os.path.join(DATA, f"documentos_{siape}"),
        "json": os.path.join(DATA, f"resultado_{siape}.json"),
        "classes": os.path.join(DATA, f"classificacao_{siape}.json"),
        "resumo_cache": os.path.join(DATA, f"resumos_{siape}_cache.json"),
        "md": os.path.join(DATA, f"resumo_{siape}.md"),
        "pdf": os.path.join(DATA, f"resumo_{siape}.pdf"),
        "zip": os.path.join(DATA, f"documentos_{siape}.zip"),
    }


def _buscar_e_baixar(siape: str, p: dict) -> None:
    """Executa a busca + download apenas se ainda nao houver resultado."""
    if os.path.exists(p["json"]) and os.path.isdir(p["pasta"]):
        return
    client = GedocClient()
    client.abrir()
    total, docs = client.coletar(siape)
    filtrar_por_siape(docs, siape)
    validos = [d for d in docs if d.contem_siape]
    baixar(client, validos, p["pasta"])
    os.makedirs(DATA, exist_ok=True)
    with open(p["json"], "w", encoding="utf-8") as f:
        json.dump({"termo": siape, "total_bruto": total,
                   "total_com_siape": len(validos),
                   "documentos": [asdict(d) for d in validos],
                   "descartados": [asdict(d) for d in docs if not d.contem_siape]},
                  f, ensure_ascii=False, indent=2)


def run_pipeline(siape: str) -> dict:
    """Roda tudo (idempotente via caches) e devolve dados para a UI."""
    p = _paths(siape)
    api_key = mc.resolver_api_key()
    modelo = mc.MODELO_PADRAO

    _buscar_e_baixar(siape, p)
    termo, docs = resumir_mistral.carregar_docs(p["json"])

    categorias = carregar_categorias(CATEGORIA_JSON)
    ordem = [c["nome"] for c in categorias]
    modo = "llm" if api_key else "keyword"

    classes = resumir_mistral._carregar_cache(p["classes"])
    classificar_docs(docs, categorias, modo, api_key, modelo, classes, p["classes"])

    # resumos + markdown agrupado + PDF
    cache = resumir_mistral._carregar_cache(p["resumo_cache"])
    if api_key:
        resumir_mistral.gerar_markdown(docs, termo, p["pasta"], api_key, modelo,
                                       p["md"], cache, p["resumo_cache"], ordem)
        try:
            html = md_para_pdf.md_para_html(p["md"], f"Resumo GeDoc - SIAPE {termo}")
            md_para_pdf.html_para_pdf(html, p["pdf"])
        except Exception:  # PDF e opcional; nao derruba a resposta
            pass
    else:
        for d in docs:
            d["_resumo"] = d.get("trecho", "")

    # monta resposta agrupada
    grupos: dict = {}
    for d in docs:
        grupos.setdefault(d.get("_categoria", "Outros"), []).append(d)
    ordem = ordem + [c for c in grupos if c not in ordem]

    saida_cats = []
    for cat in ordem:
        if cat not in grupos:
            continue
        itens = [{
            "titulo": d["titulo"],
            "data": d.get("data", ""),
            "link": resumir_mistral._link_https(d["link"]),
            "arquivo": d.get("arquivo"),
            "resumo": d.get("_resumo", ""),
        } for d in grupos[cat]]
        saida_cats.append({"categoria": cat, "qtd": len(itens), "itens": itens})

    return {
        "termo": termo,
        "total": len(docs),
        "tem_pdf": os.path.exists(p["pdf"]),
        "modo": modo,
        "categorias": saida_cats,
    }


# --------------------------------------------------------------------------- #
# HTTP
# --------------------------------------------------------------------------- #
class Handler(BaseHTTPRequestHandler):
    def _bytes(self, code, corpo, content_type, extra=None):
        self.send_response(code)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(corpo)))
        for k, v in (extra or {}).items():
            self.send_header(k, v)
        self.end_headers()
        self.wfile.write(corpo)

    def _json(self, code, obj):
        self._bytes(code, json.dumps(obj, ensure_ascii=False).encode("utf-8"),
                    "application/json; charset=utf-8")

    def _arquivo(self, caminho, content_type, download_nome=None):
        if not os.path.exists(caminho):
            self._json(404, {"erro": "arquivo nao encontrado"})
            return
        with open(caminho, "rb") as f:
            corpo = f.read()
        extra = {}
        if download_nome:
            extra["Content-Disposition"] = f'attachment; filename="{download_nome}"'
        self._bytes(200, corpo, content_type, extra)

    def _pagina(self, nome):
        self._arquivo(os.path.join(PROTO, nome), "text/html; charset=utf-8")

    def do_GET(self):
        u = urlparse(self.path)
        q = parse_qs(u.query)
        if u.path in ("/", "/index.html"):
            self._pagina("app.html")
        elif u.path == "/categorias":
            self._pagina("categorias_app.html")
        elif u.path == "/api/categorias":
            self._json(200, {"categorias": ler_categorias()})
        elif u.path == "/api/pdf":
            siape = self._siape(q)
            if siape:
                self._arquivo(_paths(siape)["pdf"], "application/pdf",
                              f"resumo_{siape}.pdf")
        elif u.path == "/api/zip":
            siape = self._siape(q)
            if siape:
                self._zip(siape)
        elif u.path == "/api/doc":
            siape = self._siape(q)
            nome = (q.get("arquivo") or [""])[0]
            if siape and nome and "/" not in nome and "\\" not in nome:
                self._arquivo(os.path.join(_paths(siape)["pasta"], nome),
                              "application/pdf")
            else:
                self._json(400, {"erro": "parametros invalidos"})
        else:
            self._json(404, {"erro": "rota nao encontrada"})

    def do_POST(self):
        u = urlparse(self.path)
        tam = int(self.headers.get("Content-Length", 0))
        bruto = self.rfile.read(tam) if tam else b"{}"
        try:
            corpo = json.loads(bruto.decode("utf-8") or "{}")
        except json.JSONDecodeError:
            self._json(400, {"erro": "JSON invalido"})
            return

        if u.path == "/api/categorias":
            try:
                cats = gravar_categorias(corpo)
            except ValueError as e:
                self._json(400, {"erro": str(e)})
                return
            self._json(200, {"ok": True, "total": len(cats)})
        elif u.path == "/api/buscar":
            siape = str(corpo.get("siape", "")).strip()
            if not _SIAPE_RE.match(siape):
                self._json(400, {"erro": "SIAPE invalido (5 a 8 digitos)"})
                return
            try:
                self._json(200, run_pipeline(siape))
            except Exception as e:
                self._json(500, {"erro": f"{type(e).__name__}: {e}"})
        else:
            self._json(404, {"erro": "rota nao encontrada"})

    def _siape(self, q):
        siape = (q.get("siape") or [""])[0]
        if _SIAPE_RE.match(siape):
            return siape
        self._json(400, {"erro": "SIAPE invalido"})
        return None

    def _zip(self, siape):
        p = _paths(siape)
        if not os.path.isdir(p["pasta"]):
            self._json(404, {"erro": "sem documentos"})
            return
        with zipfile.ZipFile(p["zip"], "w", zipfile.ZIP_DEFLATED) as z:
            for nome in sorted(os.listdir(p["pasta"])):
                if nome.lower().endswith(".pdf"):
                    z.write(os.path.join(p["pasta"], nome), nome)
        self._arquivo(p["zip"], "application/zip", f"documentos_{siape}.zip")

    def log_message(self, *a):
        pass


def main() -> int:
    ap = argparse.ArgumentParser(description="Sistema completo GeDoc IFES (web).")
    ap.add_argument("--port", type=int, default=8000)
    ap.add_argument("--no-browser", action="store_true")
    args = ap.parse_args()

    srv = ThreadingHTTPServer(("127.0.0.1", args.port), Handler)
    url = f"http://127.0.0.1:{args.port}"
    print(f"Sistema GeDoc IFES em {url}")
    print(f"Chave Mistral: {'OK' if mc.resolver_api_key() else 'AUSENTE (modo keyword)'}")
    print("Ctrl+C para parar.")
    if not args.no_browser:
        webbrowser.open(url)
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        print("\nEncerrado.")
    finally:
        srv.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
