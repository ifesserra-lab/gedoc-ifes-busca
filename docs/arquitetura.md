# Arquitetura — GeDoc IFES Toolkit

Documento para desenvolvedores/mantenedores. Cobre a estrutura do monorepo,
os alvos desktop e web, o deploy e o CI/CD, e a classificação por categorias.

## Visão geral

O mesmo núcleo Rust serve **dois alvos**: o app **desktop** (Tauri) e a
**API web** (axum). O frontend Vue é **compartilhado** — escolhe o transporte
em runtime (IPC no desktop, HTTP na web).

```text
                    ┌─────────────────────────────┐
                    │  core/  (gedocs-core, Rust)  │  domínio, serviços,
                    │  SEM Tauri                   │  ports, dto, use-cases
                    └───────────┬─────────────────┘
                        ┌───────┴────────┐
        reusa           │                │          reusa
   ┌────────────────────▼──┐        ┌────▼──────────────────┐
   │ src-tauri/ (desktop)  │        │ server/ (API axum)    │
   │ comandos #[tauri::…]  │        │ handlers HTTP + sessão │
   └───────────┬───────────┘        └───────────┬───────────┘
               │ IPC                              │ HTTP
   ┌───────────▼───────────────────────────────────────────┐
   │ app/ (Vue 3) — ipc.ts dual-mode (invoke | fetch)       │
   └───────────────────────────────────────────────────────┘
```

## Estrutura do monorepo

| Pasta | Papel |
|---|---|
| `core/` | Crate `gedocs-core` (Rust, **sem Tauri**): `domain`, `services`, `ports`, `dto`, `usecases`, `error`. Regra de negócio única. |
| `src-tauri/` | App desktop (Tauri 2). Só os comandos `#[tauri::command]` (Controller/IPC) que resolvem diretórios do app e delegam ao `gedocs-core`. |
| `server/` | API web (axum) que reusa `gedocs-core`. Handlers finos + sessão efêmera + TTL + CORS + rate limit. **Não** linka Tauri/WebKit. |
| `app/` | Frontend Vue 3 (Composition API + Pinia). `src/services/ipc.ts` é a porta única — dual-mode. |
| `config/` | `categoria.json` (categorias: nome + descrição) — fonte única da classificação. |
| `specs/` | Spec-kit: `003-versao-web`, `004-ordenar-por-ano`, `005-web-remove-crud-categorias`, `006-ia-prompt-categorias`. |

### Camadas (MVC / hexagonal)

- **Model/domínio** e **serviços/ports** vivem em `core` (sem I/O de UI).
- **Use-cases** (`core/src/usecases/*`) são puros e testáveis: recebem
  repositórios/portas e caminhos por parâmetro — **sem `AppHandle`**.
- **Controllers**: comandos Tauri (`src-tauri`) e handlers HTTP (`server`)
  são finos e só resolvem caminhos/sessão e chamam os use-cases.

## Frontend dual-mode

`app/src/services/ipc.ts` detecta o ambiente com `emTauri()`
(`'__TAURI_INTERNALS__' in window`):

- **desktop** → `invoke()` dos comandos Tauri;
- **web** → `fetch()` da API (mesma origem, ver deploy).

As assinaturas exportadas são idênticas nos dois modos, então stores e views
não mudam. `AppError` é serializado como `{tipo, mensagem}` — a função
`mensagemDeErro` traduz para texto amigável nos dois modos.

## API web (server/)

Endpoints sob `/api` (ver `specs/003-versao-web/contracts/http-api.md`):

| Método | Rota | Função |
|---|---|---|
| GET | `/api/health` | liveness |
| POST | `/api/buscar` | busca por SIAPE (keyword/llm) |
| POST | `/api/documento/baixar` | baixa PDF para a sessão |
| GET | `/api/documento/:siape/:arquivo` | serve um PDF da sessão |
| POST | `/api/relatorio` | gera o relatório da sessão |
| GET | `/api/relatorio/:siape` | serve o relatório HTML |
| GET | `/api/zip/:siape` | ZIP dos PDFs da sessão |

Removidos na web (spec 005): `GET`/`PUT /api/categorias` — sem CRUD de
categorias na web.

### Sessão efêmera + TTL (LGPD)

Sem login, cada visitante recebe um cookie opaco `gedocs_sid`
(`HttpOnly`; em produção `SameSite=None; Secure`). O servidor mapeia para
`<data>/sessions/<sid>/{documentos,relatorios,cache}`. Um job em background
varre e remove sessões inativas além do TTL (`GEDOCS_SESSION_TTL`, ~1h).
PDFs (PII de terceiros) ficam isolados por sessão; nada persiste entre
sessões. Sem PII em logs.

### Anti-abuso

Rate limit por IP (via `X-Forwarded-For`, respeitando o proxy) + limite de
tamanho de corpo. Ao exceder → 429 com mensagem amigável.

## Deploy

- **Frontend → Vercel** (estático). O `vercel.json` faz **proxy** de
  `/api/*` para a API no Render. Assim front e API ficam na **mesma origem**
  do ponto de vista do navegador → o cookie de sessão é **first-party** e
  funciona em qualquer navegador (Safari/Chrome bloqueiam cookies de
  terceiros; por isso o proxy é essencial).
- **API → Render** (Docker). `server/Dockerfile` builda `gedocs-core` +
  `server` (sem WebKit → imagem pequena). O Render injeta `PORT`; o servidor
  faz bind nele. `render.yaml` é o blueprint.

URLs de produção:
- App: <https://gedocs.vercel.app>
- API: <https://gedoc-search-api.onrender.com>

### Variáveis de ambiente (API)

| Var | Uso |
|---|---|
| `GEDOCS_DATA_DIR` | raiz efêmera das sessões (tmpfs) |
| `GEDOCS_SESSION_TTL` | TTL da sessão em segundos (padrão 3600) |
| `GEDOCS_SECURE_COOKIE` | `true` em produção (cookie `Secure`) |
| `GEDOCS_CORS_ORIGIN` | origem permitida (acesso direto à API) |
| `GEDOCS_RATE_LIMIT` | máx. requisições/IP por minuto |
| `MISTRAL_API_KEY` | chave da IA (opcional; só no servidor) |
| `PORT` | injetada pelo Render; usada se `GEDOCS_BIND` ausente |

## CI/CD (GitHub Actions)

- **`.github/workflows/ci.yml`** — em push/PR, roda `fmt` + `clippy -D
  warnings` + `test` para `core`, `server` e `src-tauri`, e `vitest` +
  `vue-tsc` para `app`.
- **`.github/workflows/deploy.yml`** — após o **CI concluir com sucesso em
  `main`** (`workflow_run`), dispara o deploy: novo deploy da API no Render
  (via API) + `vercel --prod` do frontend. Secrets: `RENDER_API_KEY`,
  `RENDER_SERVICE_ID`, `VERCEL_TOKEN`, `VERCEL_ORG_ID`, `VERCEL_PROJECT_ID`.

Fluxo padrão: branch → PR → CI verde → merge (squash) em `main` → deploy
automático → validar no ar. Branches são deletados após o merge.

## Classificação por categorias

`config/categoria.json` (nome + descrição) é a **fonte única** de categorias.

- **Palavra-chave** (default): casa nome/descrição no título/trecho do
  documento (`ports::classificador::ClassificadorPalavraChave`).
- **IA** (`ClassificadorLlm`): `montar_prompt` injeta **os nomes e descrições
  das categorias no prompt** e pede exatamente uma em JSON; `extrair_categoria`
  valida a resposta contra o conjunto — qualquer nome fora da lista (ou JSON
  inválido) cai em **Outros** (R4). Falha de IA em um documento cai para
  palavra-chave naquele documento (R11), sem abortar o lote.

Os resultados são agrupados por categoria na ordem do `category.json` e, dentro
de cada grupo, **ordenados por ano** (decrescente; sem data ao fim) — spec 004.
Grupos vazios são omitidos. Editar o `category.json` reflete na próxima busca
(após deploy), sem mudança de código (Princípio IV).

## Testes

- **Rust** (`cargo test`): domínio, serviços, use-cases e classificação em
  `core` (sem rede, dublês de porta); integração dos endpoints em
  `server/tests/api.rs` (sessão/isolamento, TTL, relatório, zip, rate limit —
  sem rede).
- **Frontend** (`vitest` + `vue-tsc`): adaptador `ipc.ts` e componentes.

Princípio VII (TDD): comportamento novo nasce de um teste; o CI barra o merge
sem os testes verdes.

## Referências

- Plano e decisões da web: [plano-web.md](plano-web.md), [brief-web.md](brief-web.md)
- Specs: [`specs/003-versao-web`](../specs/003-versao-web/),
  [`004`](../specs/004-ordenar-por-ano/),
  [`005`](../specs/005-web-remove-crud-categorias/),
  [`006`](../specs/006-ia-prompt-categorias/)
- Contrato da API: [`specs/003-versao-web/contracts/http-api.md`](../specs/003-versao-web/contracts/http-api.md)
