# Quickstart — Versão Web (validação)

Guia para rodar e validar a versão web de ponta a ponta. Não contém
implementação — só como subir e conferir os cenários do [spec.md](./spec.md).

## Pré-requisitos

- Rust 1.80+ e Node 18+ instalados.
- `config/categoria.json` presente (semente das categorias).
- (Opcional, modo IA) `MISTRAL_API_KEY` no ambiente do servidor.

## Subir localmente

### 1. API (Rust/axum)

```bash
# variáveis de ambiente da API
export GEDOCS_DATA_DIR="$(mktemp -d)"          # armazenamento efêmero
export GEDOCS_CORS_ORIGIN="http://localhost:5173"
export GEDOCS_SESSION_TTL="3600"                # 1h (segundos)
# export MISTRAL_API_KEY="..."                  # opcional (modo IA)

cargo run -p gedocs-server                      # sobe em http://localhost:8787
```

### 2. Frontend (Vue/Vite) em modo web

```bash
cd app
echo 'VITE_API_URL=http://localhost:8787' > .env.local
npm install
npm run dev                                     # http://localhost:5173
```

Abrir `http://localhost:5173` no navegador (modo web — sem Tauri).

> Desktop continua via `tauri dev` (usa `invoke`, ignora `VITE_API_URL`).

## Cenários de validação

| # | US | Ação | Esperado |
|---|---|---|---|
| 1 | US1 | Buscar um SIAPE válido | documentos agrupados por categoria + total |
| 2 | US1 | Buscar SIAPE inválido | mensagem amigável; nenhuma coleta |
| 3 | US3 | Baixar um documento | `{arquivo}` retornado; abre em nova aba |
| 4 | US4 | Buscar no modo IA sem chave | cai para palavra-chave; busca não falha |
| 5 | US5 | Gerar relatório | HTML consolidado abre em nova aba |
| 6 | US6 | Baixar ZIP após baixar PDFs | download do `.zip` |
| 7 | US6 | Pedir ZIP sem PDFs | erro amigável orientando baixar |
| 8 | US7 | Salvar categorias (nome vazio/dup) | rejeitado; nada gravado |
| 9 | US2 | Abrir em 2 navegadores/sessões e baixar | uma sessão não acessa arquivos da outra |
| 10 | US2 | Aguardar > TTL e conferir | arquivos da sessão removidos |

## Checagens rápidas (curl)

```bash
curl -s localhost:8787/api/health
curl -s -c cookies.txt -X POST localhost:8787/api/buscar \
  -H 'Content-Type: application/json' \
  -d '{"siape":"1998547","modo":"keyword"}' | head
# reusar cookies.txt garante a MESMA sessão nas próximas chamadas
```

## Testes automatizados

```bash
cargo test -p gedocs-server      # integração dos endpoints (dublês, sem rede)
cd app && npm test               # vitest do adaptador ipc.ts dual-mode
```

## Deploy (resumo)

- Frontend → Vercel (`vercel.json`; env `VITE_API_URL` = URL da API).
- API → container (Fly/Railway) via `server/Dockerfile`; secret
  `MISTRAL_API_KEY`; envs `GEDOCS_CORS_ORIGIN`, `GEDOCS_DATA_DIR`,
  `GEDOCS_SESSION_TTL`. Detalhes em [docs/plano-web.md](../../docs/plano-web.md).
