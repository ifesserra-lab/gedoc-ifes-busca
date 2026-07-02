#!/usr/bin/env python3
"""Converte um arquivo Markdown em PDF.

Fluxo sem dependencias pesadas: Markdown -> HTML estilizado -> PDF via
Google Chrome em modo headless (--print-to-pdf).

Uso:
    python3 md_para_pdf.py resumo_1998547.md
    python3 md_para_pdf.py resumo_1998547.md --out resumo.pdf --titulo "Resumo"
"""
from __future__ import annotations

import argparse
import logging
import os
import subprocess
import sys
import tempfile

import markdown

log = logging.getLogger("md2pdf")

CHROME_CANDIDATOS = [
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    "google-chrome", "chromium", "chromium-browser",
]

CSS = """
  @page { size: A4; margin: 18mm 16mm; }
  * { box-sizing: border-box; }
  body { font-family: -apple-system, Segoe UI, Roboto, Arial, sans-serif;
         color: #1a2330; line-height: 1.5; font-size: 11pt; }
  h1 { font-size: 20pt; color: #0b5cad; border-bottom: 2px solid #0b5cad;
       padding-bottom: 6px; }
  h2 { font-size: 15pt; color: #0b5cad; margin-top: 22px;
       border-bottom: 1px solid #e5e9ef; padding-bottom: 4px;
       page-break-before: always; }
  h2:first-of-type { page-break-before: avoid; }
  h3 { font-size: 12pt; margin: 16px 0 4px; page-break-after: avoid; }
  p { margin: 4px 0 10px; }
  a { color: #0b5cad; text-decoration: none; }
  table { border-collapse: collapse; width: 100%; margin: 10px 0; }
  th, td { border: 1px solid #d7dde6; padding: 6px 10px; text-align: left; }
  th { background: #eef2f7; }
  td:last-child, th:last-child { text-align: right; }
  code { background: #f1f4f8; padding: 1px 5px; border-radius: 4px;
         font-size: 9.5pt; }
  h3 + p { color: #5a6474; font-size: 9.5pt; }
"""

HTML_TMPL = """<!doctype html>
<html lang="pt-BR"><head><meta charset="utf-8">
<title>{titulo}</title><style>{css}</style></head>
<body>{corpo}</body></html>"""


def achar_chrome() -> str:
    for c in CHROME_CANDIDATOS:
        if os.path.isfile(c):
            return c
        caminho = shutil_which(c)
        if caminho:
            return caminho
    raise RuntimeError("Chrome/Chromium nao encontrado para gerar o PDF.")


def shutil_which(cmd: str):
    from shutil import which
    return which(cmd)


def md_para_html(caminho_md: str, titulo: str) -> str:
    with open(caminho_md, encoding="utf-8") as f:
        texto = f.read()
    corpo = markdown.markdown(
        texto, extensions=["tables", "fenced_code", "md_in_html", "sane_lists"])
    return HTML_TMPL.format(titulo=titulo, css=CSS, corpo=corpo)


def html_para_pdf(html: str, saida: str) -> None:
    chrome = achar_chrome()
    with tempfile.NamedTemporaryFile("w", suffix=".html", delete=False,
                                     encoding="utf-8") as tmp:
        tmp.write(html)
        caminho_html = tmp.name
    try:
        cmd = [
            chrome, "--headless=new", "--disable-gpu", "--no-sandbox",
            "--no-pdf-header-footer",
            f"--print-to-pdf={os.path.abspath(saida)}",
            f"--virtual-time-budget=10000",
            f"file://{caminho_html}",
        ]
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
        if not os.path.exists(saida):
            raise RuntimeError(
                "Chrome nao gerou o PDF.\n" + proc.stderr[-500:])
    finally:
        os.unlink(caminho_html)


def parse_args(argv=None) -> argparse.Namespace:
    ap = argparse.ArgumentParser(description="Converte Markdown em PDF.")
    ap.add_argument("md", help="arquivo Markdown de entrada")
    ap.add_argument("--out", help="PDF de saida (padrao: mesmo nome .pdf)")
    ap.add_argument("--titulo", default="Documento", help="titulo da pagina")
    return ap.parse_args(argv)


def main(argv=None) -> int:
    args = parse_args(argv)
    logging.basicConfig(level=logging.INFO, format="%(message)s", stream=sys.stderr)

    saida = args.out or f"{os.path.splitext(args.md)[0]}.pdf"
    try:
        html = md_para_html(args.md, args.titulo)
        html_para_pdf(html, saida)
    except (RuntimeError, FileNotFoundError) as e:
        log.error("Erro: %s", e)
        return 1
    tam = os.path.getsize(saida) // 1024
    log.info("PDF gerado: %s (%d KB)", saida, tam)
    return 0


if __name__ == "__main__":
    sys.exit(main())
