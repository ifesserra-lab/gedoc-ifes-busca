#!/usr/bin/env python3
"""Categoriza os documentos do GeDoc em Progressao, Comissao, Ferias e Outros.

Le o JSON gerado por `buscar_gedoc.py`, classifica cada documento por
palavras-chave no titulo e no trecho, organiza os PDFs em subpastas por
categoria (copiando, sem mover os originais) e gera um Markdown agrupado.

Uso:
    python3 categorizar.py \\
        --json resultado_1802019.json \\
        --pdfs documentos_1802019 \\
        --out categorizado_1802019 \\
        --md categorizado_1802019.md
"""
from __future__ import annotations

import argparse
import json
import logging
import os
import re
import shutil
import sys
import time
import unicodedata
from collections import defaultdict
from typing import Dict, List, Optional

import mistral_client as mc

log = logging.getLogger("categorizar")

# Ordem importa: a primeira categoria cujo padrao casar vence.
# Progressao antes de tudo; Comissao antes de Ferias (uma alteracao de
# comissao pode citar "substitui" um membro, mas continua sendo Comissao).
CATEGORIAS = [
    ("Progressao", re.compile(r"progress|merito")),
    ("Comissao", re.compile(r"comiss|comite|grupo de trabalho|\bgt\b|banca|equipe")),
    ("Ferias", re.compile(r"feria|substitui")),
]
OUTROS = "Outros"

THROTTLE = 1.2  # segundos entre chamadas LLM (respeita rate limit da Mistral)


def _norm(texto: str) -> str:
    """Minusculas sem acento, para casamento robusto de palavras-chave."""
    sem_acento = "".join(
        c for c in unicodedata.normalize("NFD", texto)
        if unicodedata.category(c) != "Mn")
    return sem_acento.lower()


def classificar(doc: dict) -> str:
    """Classificacao por palavras-chave (rapida, sem custo de API)."""
    alvo = _norm(f"{doc.get('titulo', '')} {doc.get('trecho', '')}")
    for nome, padrao in CATEGORIAS:
        if padrao.search(alvo):
            return nome
    return OUTROS


# --------------------------------------------------------------------------- #
# Classificacao por LLM, guiada por categoria.json
# --------------------------------------------------------------------------- #
def carregar_categorias(caminho: str) -> List[dict]:
    """Le categoria.json: [{"nome": ..., "descricao": ...}, ...]."""
    with open(caminho, encoding="utf-8") as f:
        dados = json.load(f)
    cats = dados.get("categorias", dados) if isinstance(dados, dict) else dados
    if not cats:
        raise ValueError(f"{caminho} nao contem categorias.")
    return cats


def _fallback(nomes: List[str]) -> str:
    for n in nomes:
        if _norm(n) == "outros":
            return n
    return nomes[-1]


def classificar_llm(doc: dict, categorias: List[dict], api_key: str,
                    modelo: str) -> str:
    """Classifica o doc em UMA categoria usando as descricoes do categoria.json."""
    nomes = [c["nome"] for c in categorias]
    definicoes = "\n".join(f"- {c['nome']}: {c['descricao']}" for c in categorias)
    sistema = (
        "Voce classifica documentos administrativos do IFES em exatamente UMA "
        "categoria da lista fornecida. Baseie-se nas descricoes. Responda apenas "
        'em JSON: {"categoria": "<nome exato da categoria>"}.')
    usuario = (
        f"Categorias disponiveis:\n{definicoes}\n\n"
        f"Documento:\nTitulo: {doc.get('titulo', '')}\n"
        f"Trecho: {doc.get('trecho', '')[:1500]}")
    resposta = mc.chat(
        [{"role": "system", "content": sistema},
         {"role": "user", "content": usuario}],
        api_key, modelo, max_tokens=40, temperature=0,
        response_format={"type": "json_object"})
    try:
        cat = str(json.loads(resposta).get("categoria", "")).strip()
    except (json.JSONDecodeError, AttributeError):
        cat = ""
    return cat if cat in nomes else _fallback(nomes)


def classificar_docs(docs: List[dict], categorias: Optional[List[dict]],
                     modo: str, api_key: Optional[str], modelo: str,
                     cache: Dict[str, str],
                     cache_path: Optional[str]) -> None:
    """Preenche doc['_categoria'] em cada doc, conforme o modo escolhido."""
    usar_llm = modo == "llm"
    for i, d in enumerate(docs, 1):
        link = d["link"]
        if link in cache:
            d["_categoria"] = cache[link]
            continue
        if usar_llm:
            log.info("classificando %d/%d: %s", i, len(docs), d["titulo"][:55])
            cat = classificar_llm(d, categorias, api_key, modelo)
            time.sleep(THROTTLE)  # evita 429 (rate limit)
        else:
            cat = classificar(d)
        d["_categoria"] = cat
        cache[link] = cat
        if cache_path:
            with open(cache_path, "w", encoding="utf-8") as f:
                json.dump(cache, f, ensure_ascii=False, indent=2)


def _pasta_segura(nome: str) -> str:
    """Nome de pasta sem acento/espaco (Progressao, Comissao, ...)."""
    return _norm(nome).replace(" ", "_")


def organizar(grupos: Dict[str, List[dict]], pasta_pdf: str,
              destino: str) -> Dict[str, int]:
    """Copia os PDFs para destino/<Categoria>/. Retorna copiados por categoria."""
    copiados: Dict[str, int] = defaultdict(int)
    for categoria, docs in grupos.items():
        sub = os.path.join(destino, _pasta_segura(categoria))
        os.makedirs(sub, exist_ok=True)
        for d in docs:
            arq = d.get("arquivo")
            if not arq:
                continue
            origem = os.path.join(pasta_pdf, arq)
            if not os.path.exists(origem):
                log.warning("PDF ausente: %s", origem)
                continue
            shutil.copy2(origem, os.path.join(sub, arq))
            copiados[categoria] += 1
    return copiados


def gerar_markdown(grupos: Dict[str, List[dict]], termo: str,
                   arquivo: str, ordem: List[str]) -> None:
    total = sum(len(v) for v in grupos.values())
    # inclui eventuais categorias fora da ordem informada
    ordem = ordem + [c for c in grupos if c not in ordem]

    linhas = [
        f"# Documentos por categoria - SIAPE {termo}",
        "",
        f"Total: **{total}** documentos.",
        "",
        "| Categoria | Qtd |",
        "| --- | ---: |",
    ]
    for cat in ordem:
        if grupos.get(cat):
            linhas.append(f"| {cat} | {len(grupos[cat])} |")
    linhas.append("")

    for cat in ordem:
        docs = grupos.get(cat, [])
        if not docs:
            continue
        linhas.append(f"## {cat} ({len(docs)})")
        linhas.append("")
        for d in sorted(docs, key=lambda x: x.get("arquivo", "")):
            link = d["link"].replace("http://gedoc.ifes.edu.br:80",
                                     "https://gedoc.ifes.edu.br")
            linhas.append(f"- [{d['titulo']}]({link}) — {d.get('data', '-')}")
        linhas.append("")

    with open(arquivo, "w", encoding="utf-8") as f:
        f.write("\n".join(linhas))


def parse_args(argv=None) -> argparse.Namespace:
    ap = argparse.ArgumentParser(description="Categoriza documentos do GeDoc.")
    ap.add_argument("--json", required=True, help="JSON gerado por buscar_gedoc.py")
    ap.add_argument("--pdfs", help="pasta com os PDFs (para organizar em subpastas)")
    ap.add_argument("--out", help="pasta destino das subpastas por categoria")
    ap.add_argument("--md", help="arquivo Markdown de saida")
    ap.add_argument("--categorias",
                    default=os.path.join(mc.ROOT, "config", "categoria.json"),
                    help="JSON com nome+descricao das categorias (modo llm)")
    ap.add_argument("--modo", choices=("keyword", "llm"), default="llm",
                    help="keyword (regex, gratis) ou llm (usa categoria.json)")
    ap.add_argument("--model", default=mc.MODELO_PADRAO,
                    help=f"modelo Mistral (padrao {mc.MODELO_PADRAO})")
    ap.add_argument("--cache", help="JSON de cache das classificacoes")
    return ap.parse_args(argv)


def main(argv=None) -> int:
    args = parse_args(argv)
    logging.basicConfig(level=logging.INFO, format="%(message)s", stream=sys.stderr)

    with open(args.json, encoding="utf-8") as f:
        dados = json.load(f)
    termo = dados.get("termo", "")
    docs = dados.get("documentos", [])
    if not docs:
        log.error("Nenhum documento no JSON.")
        return 1

    # define categorias e modo
    modo = args.modo
    categorias: Optional[List[dict]] = None
    ordem = ["Progressao", "Comissao", "Ferias", OUTROS]
    api_key = None
    if modo == "llm":
        if not os.path.exists(args.categorias):
            log.error("categoria.json nao encontrado: %s", args.categorias)
            return 1
        categorias = carregar_categorias(args.categorias)
        ordem = [c["nome"] for c in categorias]
        api_key = mc.resolver_api_key()
        if not api_key:
            log.error("Modo llm requer MISTRAL_API_KEY/MISTRAL_KEY (env ou .env).")
            return 1

    cache_path = args.cache or (
        f"{os.path.splitext(args.md)[0]}_catcache.json" if args.md
        else "classificacao_cache.json")
    cache: Dict[str, str] = {}
    if os.path.exists(cache_path):
        with open(cache_path, encoding="utf-8") as f:
            cache = json.load(f)

    classificar_docs(docs, categorias, modo, api_key, args.model,
                     cache, cache_path)

    grupos: Dict[str, List[dict]] = defaultdict(list)
    for d in docs:
        grupos[d["_categoria"]].append(d)

    print(f"Categorizacao ({modo}) - SIAPE {termo} ({len(docs)} docs)")
    for cat in ordem + [c for c in grupos if c not in ordem]:
        if grupos.get(cat):
            print(f"  {cat:14s}: {len(grupos[cat])}")

    if args.pdfs and args.out:
        copiados = organizar(grupos, args.pdfs, args.out)
        log.info("PDFs copiados: %s", dict(copiados))
    if args.md:
        gerar_markdown(grupos, termo, args.md, ordem)
        log.info("Markdown: %s", args.md)
    return 0


if __name__ == "__main__":
    sys.exit(main())
