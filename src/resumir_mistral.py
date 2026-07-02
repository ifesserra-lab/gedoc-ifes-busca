#!/usr/bin/env python3
"""Resume documentos do GeDoc usando a API da Mistral e gera um Markdown.

Le o JSON produzido por `buscar_gedoc.py` (metadados) e os PDFs baixados,
extrai o texto de cada PDF (via `pdftotext`) e pede um resumo curto a Mistral,
citando cada documento no Markdown de saida.

Requer a variavel de ambiente MISTRAL_API_KEY.

Uso:
    export MISTRAL_API_KEY=...
    python3 resumir_mistral.py \\
        --json resultado_1802019.json \\
        --pdfs documentos_1802019 \\
        --out resumo_1802019.md
"""
from __future__ import annotations

import argparse
import json
import logging
import os
import subprocess
import sys
from collections import defaultdict
from typing import Dict, List, Optional

import mistral_client as mc
from categorizar import carregar_categorias, classificar_docs

log = logging.getLogger("resumir")

MAX_CHARS = 6000  # limite de texto enviado por documento

SISTEMA = (
    "Voce resume documentos administrativos do IFES (portarias, despachos). "
    "Escreva 2 a 3 frases objetivas em portugues, informando o que o documento "
    "determina, o orgao/campus, pessoas ou comissoes envolvidas, datas e a "
    "finalidade. Nao invente dados que nao estejam no texto. Nao repita o titulo."
)


def extrair_texto(caminho_pdf: str) -> str:
    """Extrai texto de um PDF via pdftotext; retorna '' em caso de falha."""
    try:
        saida = subprocess.run(
            ["pdftotext", "-layout", caminho_pdf, "-"],
            capture_output=True, text=True, timeout=30, check=True)
        return saida.stdout.strip()
    except (subprocess.SubprocessError, FileNotFoundError) as e:
        log.warning("pdftotext falhou em %s: %s", caminho_pdf, e)
        return ""


def resumir(texto: str, api_key: str, modelo: str) -> str:
    """Pede um resumo curto a Mistral."""
    return mc.chat(
        [{"role": "system", "content": SISTEMA},
         {"role": "user", "content": texto[:MAX_CHARS]}],
        api_key, modelo, max_tokens=300, temperature=0.2)


def carregar_docs(caminho_json: str) -> tuple:
    with open(caminho_json, encoding="utf-8") as f:
        dados = json.load(f)
    return dados.get("termo", ""), dados.get("documentos", [])


def _carregar_cache(caminho: Optional[str]) -> Dict[str, str]:
    if caminho and os.path.exists(caminho):
        with open(caminho, encoding="utf-8") as f:
            return json.load(f)
    return {}


def _salvar_cache(cache: Dict[str, str], caminho: Optional[str]) -> None:
    if caminho:
        with open(caminho, "w", encoding="utf-8") as f:
            json.dump(cache, f, ensure_ascii=False, indent=2)


def resolver_resumo(doc: dict, cache: Dict[str, str], api_key: str,
                    modelo: str, pasta_pdf: str) -> str:
    """Retorna o resumo do doc, do cache ou chamando Mistral."""
    link = doc["link"]
    if link in cache:
        return cache[link]
    texto = ""
    if doc.get("arquivo"):
        texto = extrair_texto(os.path.join(pasta_pdf, doc["arquivo"]))
    if not texto:
        texto = doc.get("trecho", "")  # fallback: snippet da busca
    try:
        resumo = resumir(texto, api_key, modelo) if texto else "_(sem texto)_"
    except Exception as e:  # nao aborta o lote inteiro por 1 doc
        log.error("erro em '%s': %s", doc.get("titulo", "")[:50], e)
        resumo = f"_(falha ao resumir: {e})_"
    cache[link] = resumo
    return resumo


def _link_https(link: str) -> str:
    return link.replace("http://gedoc.ifes.edu.br:80", "https://gedoc.ifes.edu.br")


def gerar_markdown(docs: List[dict], termo: str, pasta_pdf: str,
                   api_key: str, modelo: str, saida: str,
                   cache: Dict[str, str], cache_path: Optional[str],
                   ordem: List[str]) -> None:
    """Resume cada doc (usando cache) e escreve o Markdown agrupado por categoria.

    Assume que cada doc ja possui `_categoria` (definido por classificar_docs).
    """
    for i, d in enumerate(docs, 1):
        log.info("resumindo %2d/%d: %s", i, len(docs), d["titulo"][:60])
        d["_resumo"] = resolver_resumo(d, cache, api_key, modelo, pasta_pdf)
        _salvar_cache(cache, cache_path)  # incremental

    grupos: Dict[str, List[dict]] = defaultdict(list)
    for d in docs:
        grupos[d.get("_categoria", "Outros")].append(d)
    ordem = ordem + [c for c in grupos if c not in ordem]

    partes = [
        f"# Resumo dos documentos por categoria - SIAPE {termo}",
        "",
        f"Total: **{len(docs)}** documentos. "
        f"Resumos gerados por Mistral (`{modelo}`) a partir do texto dos PDFs.",
        "",
        "| Categoria | Qtd |",
        "| --- | ---: |",
    ]
    for cat in ordem:
        if grupos.get(cat):
            partes.append(f"| [{cat}](#{_ancora(cat)}) | {len(grupos[cat])} |")
    partes.append(f"| **Total** | **{len(docs)}** |")
    partes.append("")

    for cat in ordem:
        docs_cat = grupos.get(cat, [])
        if not docs_cat:
            continue
        partes.append(f'<a id="{_ancora(cat)}"></a>')
        partes.append(f"## {cat} ({len(docs_cat)})")
        partes.append("")
        for j, d in enumerate(sorted(docs_cat, key=lambda x: x.get("arquivo", "")), 1):
            partes.append(f"### {j}. {d['titulo']}")
            partes.append("")
            meta = (f"**Data:** {d.get('data', '-')} · **SIAPE:** {termo} · "
                    f"[Original]({_link_https(d['link'])})")
            if d.get("arquivo"):
                meta += f" · Arquivo: `{d['arquivo']}`"
            partes.append(meta)
            partes.append("")
            partes.append(d["_resumo"])
            partes.append("")

        with open(saida, "w", encoding="utf-8") as f:  # incremental
            f.write("\n".join(partes))

    log.info("Markdown salvo em %s", saida)


def _ancora(nome: str) -> str:
    return nome.lower().replace(" ", "-")


def parse_args(argv=None) -> argparse.Namespace:
    ap = argparse.ArgumentParser(description="Resume documentos do GeDoc com Mistral.")
    ap.add_argument("--json", required=True, help="JSON gerado por buscar_gedoc.py")
    ap.add_argument("--pdfs", required=True, help="pasta com os PDFs baixados")
    ap.add_argument("--out", required=True, help="arquivo Markdown de saida")
    ap.add_argument("--model", default=mc.MODELO_PADRAO,
                    help=f"modelo (padrao {mc.MODELO_PADRAO})")
    ap.add_argument("--limit", type=int, help="processa apenas os N primeiros (teste)")
    ap.add_argument("--cache", help="JSON de cache dos resumos")
    ap.add_argument("--categorias",
                    default=os.path.join(mc.ROOT, "config", "categoria.json"),
                    help="JSON de categorias (nome+descricao)")
    ap.add_argument("--modo", choices=("keyword", "llm"), default="llm",
                    help="classificacao por keyword ou llm (padrao llm)")
    ap.add_argument("--classificacao", help="JSON de cache das classificacoes")
    return ap.parse_args(argv)


def main(argv=None) -> int:
    args = parse_args(argv)
    logging.basicConfig(level=logging.INFO, format="%(message)s", stream=sys.stderr)

    api_key = mc.resolver_api_key()
    if not api_key:
        log.error("Defina MISTRAL_API_KEY (ou MISTRAL_KEY) no ambiente ou no .env")
        return 1

    termo, docs = carregar_docs(args.json)
    if not docs:
        log.error("Nenhum documento no JSON %s", args.json)
        return 1
    if args.limit:
        docs = docs[:args.limit]

    # categorias + classificacao (cache compartilhado com categorizar.py)
    categorias = carregar_categorias(args.categorias)
    ordem = [c["nome"] for c in categorias]
    class_path = args.classificacao or f"{os.path.splitext(args.out)[0]}_classes.json"
    classes = _carregar_cache(class_path)
    classificar_docs(docs, categorias, args.modo, api_key, args.model,
                     classes, class_path)

    # resumos (cache proprio)
    cache_path = args.cache or f"{os.path.splitext(args.out)[0]}_cache.json"
    cache = _carregar_cache(cache_path)
    log.info("cache: %d resumo(s) reaproveitado(s)", len(cache))

    gerar_markdown(docs, termo, args.pdfs, api_key, args.model,
                   args.out, cache, cache_path, ordem)
    return 0


if __name__ == "__main__":
    sys.exit(main())
