# Quickstart — Ordenar portarias por ano (validação)

Como validar que os documentos saem ordenados por ano (desc; sem data ao fim;
estável).

## Testes automatizados (núcleo)

```bash
cargo test --manifest-path core/Cargo.toml montar_resultado
# ou a suíte inteira do núcleo:
cargo test --manifest-path core/Cargo.toml
```

Casos cobertos (ver `tasks.md`):
- anos variados → saída decrescente por ano;
- documentos sem data → ao final;
- empate de ano → data completa decrescente / estável;
- contagem por categoria e total inalterados.

## Verificação ponta-a-ponta

### Web (produção/local)
Buscar um SIAPE com documentos de anos variados (ex.: `1998547`, `1545450`) e
conferir, em cada categoria, do ano mais recente ao mais antigo:

```bash
curl -s -X POST "$API/api/buscar" -H 'Content-Type: application/json' \
  -d '{"siape":"1998547","modo":"keyword"}' \
| python3 -c 'import sys,json; d=json.load(sys.stdin)
for c in d["categorias"]:
    anos=[(it.get("data") or "")[-4:] for it in c["itens"]]
    print(c["categoria"], anos)'
# Esperado: em cada linha, anos em ordem decrescente; vazios ("") ao final.
```

### Desktop (Tauri)
`tauri dev` → buscar o SIAPE → conferir a mesma ordem por ano na lista
(mesma `ResultadoView` do núcleo).

## Critério de aceite (resumo)

- Mais recente no topo de cada categoria (SC-003).
- 100% dos com data em ordem decrescente (SC-001); sem data ao fim (SC-002).
- Contagem por categoria e total idênticos (SC-004).
