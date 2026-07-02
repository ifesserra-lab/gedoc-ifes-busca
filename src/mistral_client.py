#!/usr/bin/env python3
"""Cliente minimo da API Mistral, compartilhado pelos scripts do projeto."""
from __future__ import annotations

import logging
import os
import time
from typing import Dict, List, Optional

import requests

log = logging.getLogger("mistral")

API_URL = "https://api.mistral.ai/v1/chat/completions"
MODELO_PADRAO = "mistral-small-latest"
TIMEOUT = 60
RETRIES = 4

# raiz do projeto (pai da pasta src/ onde este modulo reside)
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def _envs_candidatos(caminho: Optional[str]) -> List[str]:
    if caminho:
        return [caminho]
    return [os.path.join(ROOT, "config", ".env"),
            os.path.join(ROOT, ".env"),
            "config/.env", ".env"]


def carregar_env(caminho: Optional[str] = None) -> None:
    """Carrega variaveis de um .env simples (KEY=valor) para o ambiente.

    Procura em config/.env e .env (relativos a raiz do projeto e ao cwd).
    """
    for cand in _envs_candidatos(caminho):
        if not os.path.exists(cand):
            continue
        with open(cand, encoding="utf-8") as f:
            for linha in f:
                linha = linha.strip()
                if not linha or linha.startswith("#") or "=" not in linha:
                    continue
                chave, _, valor = linha.partition("=")
                os.environ.setdefault(chave.strip(), valor.strip().strip('"\''))
        return


def resolver_api_key() -> Optional[str]:
    """Retorna a chave da Mistral do ambiente (aceita MISTRAL_API_KEY/MISTRAL_KEY)."""
    carregar_env()
    return os.environ.get("MISTRAL_API_KEY") or os.environ.get("MISTRAL_KEY")


def chat(messages: List[Dict[str, str]], api_key: str,
         model: str = MODELO_PADRAO, max_tokens: int = 300,
         temperature: float = 0.2,
         response_format: Optional[dict] = None) -> str:
    """Chama o endpoint de chat e retorna o conteudo. Retry em erros transitorios."""
    corpo: dict = {
        "model": model,
        "temperature": temperature,
        "max_tokens": max_tokens,
        "messages": messages,
    }
    if response_format:
        corpo["response_format"] = response_format
    headers = {"Authorization": f"Bearer {api_key}",
               "Content-Type": "application/json"}

    for tentativa in range(1, RETRIES + 1):
        resp = requests.post(API_URL, json=corpo, headers=headers, timeout=TIMEOUT)
        if resp.status_code == 200:
            return resp.json()["choices"][0]["message"]["content"].strip()
        if resp.status_code in (429, 500, 502, 503, 504) and tentativa < RETRIES:
            espera = 2 ** tentativa
            log.warning("HTTP %s -- retry em %ds (%d/%d)",
                        resp.status_code, espera, tentativa, RETRIES)
            time.sleep(espera)
            continue
        resp.raise_for_status()
    raise RuntimeError("Falha ao chamar a API Mistral apos varias tentativas.")
