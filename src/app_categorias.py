#!/usr/bin/env python3
"""Servidor minimo para cadastrar categorias em config/categoria.json.

Sem dependencias externas (usa http.server da stdlib). Serve a pagina de
cadastro e persiste as categorias no arquivo real usado pela pipeline.

Uso:
    python3 src/app_categorias.py           # http://127.0.0.1:8000
    python3 src/app_categorias.py --port 9000
"""
from __future__ import annotations

import argparse
import json
import os
import webbrowser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CATEGORIA_JSON = os.path.join(ROOT, "config", "categoria.json")
PAGINA_HTML = os.path.join(ROOT, "prototipo", "categorias_app.html")


def ler_categorias() -> list:
    if not os.path.exists(CATEGORIA_JSON):
        return []
    with open(CATEGORIA_JSON, encoding="utf-8") as f:
        dados = json.load(f)
    return dados.get("categorias", []) if isinstance(dados, dict) else dados


def gravar_categorias(cats: list) -> None:
    os.makedirs(os.path.dirname(CATEGORIA_JSON), exist_ok=True)
    with open(CATEGORIA_JSON, "w", encoding="utf-8") as f:
        json.dump({"categorias": cats}, f, ensure_ascii=False, indent=2)
        f.write("\n")


def _validar(payload) -> list:
    """Garante uma lista de {nome, descricao} com nome nao vazio."""
    if isinstance(payload, dict):
        payload = payload.get("categorias", [])
    if not isinstance(payload, list):
        raise ValueError("esperado uma lista de categorias")
    limpo = []
    for item in payload:
        nome = str(item.get("nome", "")).strip()
        if not nome:
            raise ValueError("categoria sem nome")
        limpo.append({"nome": nome,
                      "descricao": str(item.get("descricao", "")).strip()})
    return limpo


class Handler(BaseHTTPRequestHandler):
    def _json(self, code: int, obj) -> None:
        corpo = json.dumps(obj, ensure_ascii=False).encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(corpo)))
        self.end_headers()
        self.wfile.write(corpo)

    def do_GET(self):
        if self.path in ("/", "/index.html"):
            try:
                with open(PAGINA_HTML, "rb") as f:
                    corpo = f.read()
            except FileNotFoundError:
                self._json(500, {"erro": f"pagina nao encontrada: {PAGINA_HTML}"})
                return
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(corpo)))
            self.end_headers()
            self.wfile.write(corpo)
        elif self.path == "/api/categorias":
            self._json(200, {"categorias": ler_categorias()})
        else:
            self._json(404, {"erro": "rota nao encontrada"})

    def do_POST(self):
        if self.path != "/api/categorias":
            self._json(404, {"erro": "rota nao encontrada"})
            return
        tamanho = int(self.headers.get("Content-Length", 0))
        bruto = self.rfile.read(tamanho) if tamanho else b"{}"
        try:
            cats = _validar(json.loads(bruto.decode("utf-8")))
        except (json.JSONDecodeError, ValueError, AttributeError) as e:
            self._json(400, {"erro": str(e)})
            return
        gravar_categorias(cats)
        self._json(200, {"ok": True, "total": len(cats),
                         "arquivo": os.path.relpath(CATEGORIA_JSON, ROOT)})

    def log_message(self, *args):  # silencia log padrao ruidoso
        pass


def main() -> int:
    ap = argparse.ArgumentParser(description="Cadastro de categorias (web).")
    ap.add_argument("--port", type=int, default=8000)
    ap.add_argument("--no-browser", action="store_true")
    args = ap.parse_args()

    endereco = ("127.0.0.1", args.port)
    servidor = ThreadingHTTPServer(endereco, Handler)
    url = f"http://{endereco[0]}:{endereco[1]}"
    print(f"Cadastro de categorias em {url}")
    print(f"Gravando em: {CATEGORIA_JSON}")
    print("Ctrl+C para parar.")
    if not args.no_browser:
        webbrowser.open(url)
    try:
        servidor.serve_forever()
    except KeyboardInterrupt:
        print("\nEncerrado.")
    finally:
        servidor.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
