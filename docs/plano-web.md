# Plano — Versão Web do GeDoc IFES Toolkit

Objetivo: publicar uma versão **web** do app (hoje desktop Tauri) com o
frontend hospedado na **Vercel**, reaproveitando o máximo do código Rust
existente.

> **Status (implementado — inclui A2):** spec-kit completo em
> [`specs/003-versao-web/`](../specs/003-versao-web/). Núcleo extraído para
> [`core/`](../core/) (`gedocs-core`, **sem Tauri**): domínio/serviços/ports/
> use-cases/dto. [`src-tauri/`](../src-tauri/) virou wrapper fino (comandos
> Tauri → use-cases do core); [`server/`](../server/) (axum) depende **só do
> core** — `cargo tree` confirma **zero tauri/wry/webkit**, então o
> [`Dockerfile`](../server/Dockerfile) não instala mais WebKit (imagem
> pequena). Testes verdes: `core` (unit + integração), `server` 8/8 (sem
> rede), `ipc.ts` 7/7 (vitest); desktop `src-tauri` compila. Smoke local OK
> (health / SIAPE inválido / busca real / categorias + cookie). Deploy:
> [`vercel.json`](../vercel.json), [`Dockerfile`](../server/Dockerfile),
> [`fly.toml`](../server/fly.toml) e [`render.yaml`](../render.yaml).
>
> **No ar (produção):** frontend na **Vercel** <https://gedocs.vercel.app> +
> API (Docker) no **Render** <https://gedoc-search-api.onrender.com>, com
> **CI→deploy** automático a cada push em `main` ([deploy.yml](../.github/workflows/deploy.yml)).
> Testado ponta-a-ponta (busca, IA 28/28 resumos, PDF, relatório, ZIP,
> sessão cross-site). O **relatório** é habilitado na tela **só no modo IA**
> (consolida os resumos). Pendente: teste do modo IA via HTTP
> (`tasks.md` T024). 42 issues: #34–#75.

## 1. Ponto de partida (o que já existe)

- **Frontend** (`app/`): Vue 3 + Vite + Nuxt UI + Pinia + vue-router.
  Tecnologia 100% web — compila para estático (`app/dist`). Deploy na
  Vercel é trivial.
- **Backend** (`src-tauri/src/`): Rust, arquitetura hexagonal limpa
  (`domain` / `services` / `ports` / `commands`). As funções de caso de
  uso já são **puras** — recebem caminhos/repositórios por parâmetro e
  **não dependem de `AppHandle`**:
  - `commands::buscar::executar(...)`
  - `commands::documento::executar_download(...)`, `resolver_caminho_abertura(...)`
  - `commands::exportar::executar_gerar_relatorio(...)`
  - `services::empacotador::montar_zip(...)`, `services::categorias::*`

  Só os wrappers `#[tauri::command]` e os resolvedores de diretório
  (`app_data_dir` / `app_config_dir`) conhecem o Tauri. **Reuso é barato.**

## 2. Decisão de arquitetura

**Path A — Reusar Rust + frontend na Vercel** (escolhido).

```
┌─────────────┐   HTTPS/JSON   ┌──────────────────────┐
│  Vue (SPA)  │ ─────────────▶ │  API axum (Rust)     │
│  Vercel     │ ◀───────────── │  reusa gedocs_lib    │
│  estático   │   CORS         │  container (Fly/…)   │
└─────────────┘                └──────────┬───────────┘
                                          │ scrape / IA / PDF
                                          ▼
                                   Portal GeDoc + Mistral
```

Por que a API **não** fica na Vercel: o portal exige scraping HTTP
(bloqueado por CORS no browser), a chave de IA nunca pode ir ao browser,
e o processamento (scrape + LLM + extração de PDF) estoura os timeouts de
serverless. A API roda como container persistente; a Vercel serve só o
estático.

### Sub-decisão: como reusar o Rust (impacta peso do build)

| Opção | Esforço | Custo |
|---|---|---|
| **A1 — Crate `server/` com `path`-dep de `gedocs_lib`** | Baixo | Build da API compila o Tauri (webkit no Docker). Rápido de subir. |
| **A2 — Extrair `gedocs-core` (sem Tauri)** | Médio | API linka só o núcleo enxuto (sem webkit). Correto a longo prazo. |

Recomendação: **começar em A1** (entregar web funcionando), migrar para
**A2** quando estabilizar. A2 = tornar `tauri` opcional no `src-tauri`,
mover os use-cases puros + DTOs para fora dos módulos `commands`.

## 3. Mapa: comando IPC → endpoint web → mudança de comportamento

| IPC (desktop) | Endpoint web | Mudança no web |
|---|---|---|
| `buscar_por_siape` | `POST /api/buscar` | Igual. Servidor faz scrape + classifica. |
| `baixar_documento` | `POST /api/documento/baixar` | Salva no data dir do servidor, devolve nome. Sem FS do usuário. |
| `abrir_documento` | `GET /api/documento/:siape/:arquivo` | Browser abre o PDF em nova aba (não "abre com app do SO"). |
| `gerar_relatorio` | `POST /api/relatorio` + `GET /api/relatorio/:siape` | Servidor gera HTML; browser abre em nova aba. |
| `baixar_zip` | `GET /api/zip/:siape` | Servidor monta ZIP e faz stream como download. |
| `listar_categorias` | `GET /api/categorias` | Lê do data dir do servidor (ver §4). |
| `salvar_categorias` | `PUT /api/categorias` | Grava no data dir do servidor (ver §4). |
| — | `GET /api/health` | Health check p/ o container. |

Erros: a API serializa `AppError` como `{tipo, mensagem}` — o mesmo
formato que `ipc.ts::mensagemDeErro` já entende. Zero mudança no
tratamento de erro do frontend.

## 4. Decisões travadas (v1)

1. **Acesso: interno, sem login (v1).** Restrito por rede/URL, sem auth.
   Estado global compartilhado é aceitável na v1. Auth/SSO fica para
   depois se abrir ao público.

2. **Persistência: efêmera por sessão + TTL.** Sem disco persistente.
   Cada sessão (cookie `gedocs_sid`) tem seu diretório
   `<data>/sessions/<sid>/{documentos,relatorios,cache}` que:
   - isola os PDFs (PII de terceiros) de sessões diferentes — mitiga
     LGPD mesmo sem login;
   - é varrido por um job de TTL (ex.: 1h de inatividade).
   Consequência: `baixar_zip` só empacota o que foi baixado **na mesma
   sessão** (re-baixar se expirou) — comportamento aceito.

3. **Categorias: arquivo global no servidor.** Config compartilhada única
   (`<data>/categoria.json`), semeada de `config/categoria.json`. Sem
   isolamento por usuário na v1 (sem login).

4. **Retenção/LGPD.** TTL de sessão (default 1h) + varredura periódica
   limpam os PDFs. Nenhum PDF sobrevive ao fim da sessão. Chave de IA só
   como secret no servidor.

## 5. Fases de desenvolvimento

### Fase 0 — Decisões (§4) + spec
- Responder as 4 perguntas do §4.
- Registrar em `specs/` (o repo usa spec-kit) uma spec da versão web.

### Fase 1 — API HTTP (backend)
- Criar crate `server/` (axum) reusando `gedocs_lib` (opção A1).
- Estado: `AppState { data_dir, seed_categorias }` via env
  (`GEDOCS_DATA_DIR`, `GEDOCS_CORS_ORIGIN`, `MISTRAL_API_KEY`).
- Implementar os 8 endpoints do §3 (handlers finos → use-cases puros).
- `AppError` → resposta HTTP (400 SIAPE inválido, 502 portal/IA, 500 arquivo).
- CORS via `tower-http` restrito ao domínio Vercel.
- Testes de integração dos endpoints.

### Fase 2 — Frontend dual-mode
- Refatorar `app/src/services/ipc.ts`: detectar ambiente
  (`'__TAURI_INTERNALS__' in window`) → `invoke()` no desktop,
  `fetch(VITE_API_URL)` no web. Assinaturas exportadas inalteradas — as
  stores/views não mudam.
- `abrir_documento`/`gerar_relatorio` → `window.open` da URL da API.
- `baixar_zip` → fetch + blob + download.
- `app/.env.example` com `VITE_API_URL`.

### Fase 3 — Deploy
- **Vercel** (frontend): `vercel.json` (`installCommand`/`buildCommand`
  em `app/`, `outputDirectory: app/dist`, rewrite SPA → `index.html`);
  env `VITE_API_URL` = URL da API.
- **API** (container): `server/Dockerfile` (multi-stage Rust; runtime com
  libs webkit se A1) + `fly.toml`/Railway; `GEDOCS_DATA_DIR` em
  tmpfs/efêmero (sem volume — §4.2); secret `MISTRAL_API_KEY`;
  `GEDOCS_CORS_ORIGIN` = domínio Vercel; `GEDOCS_SESSION_TTL` (default 1h).

### Fase 4 — Segurança / LGPD / hardening
- CORS estrito, rate limit por IP, limite de tamanho de body.
- Chave de IA só como secret no servidor (nunca no bundle).
- Job de limpeza do data dir (TTL) + política de retenção.
- (Se necessário) auth/SSO na frente do frontend e da API.

### Fase 5 — CI/CD
- Workflow: build+test do `server/`, build da imagem, deploy no
  Fly/Railway; deploy do frontend automático pela Vercel (git).
- Reaproveitar o `.github/workflows` de release já existente.

### Fase 6 — (opcional) Migrar para A2
- Extrair `gedocs-core` sem Tauri → API sem webkit, imagem menor.

## 6. Estimativa (grosseira)

| Fase | Esforço |
|---|---|
| 0 Decisões + spec | 0,5 dia |
| 1 API axum | 1,5–2 dias |
| 2 Frontend dual-mode | 0,5–1 dia |
| 3 Deploy (Vercel + container) | 1 dia |
| 4 Segurança/LGPD | 1 dia |
| 5 CI/CD | 0,5 dia |
| 6 Core-extraction (opcional) | 1–1,5 dia |

MVP navegável (Fases 1–3): **~4 dias**.

## 7. Riscos

- **Timeout de scrape/IA**: buscas longas. Mitigar com timeouts e,
  se preciso, resposta em streaming/polling.
- **Sessão expira no meio do fluxo**: `baixar_zip` acha o dir vazio →
  re-baixar. Erro amigável já coberto por `montar_zip`.
- **Peso do build (A1)**: webkit no Docker. Some com A2.
- **Sem auth (v1)**: URL vazada = acesso. Restringir por rede/VPN e/ou
  colocar auth cedo se necessário.

## 8. Próximo passo

Decisões travadas (§4). Próximo: **Fase 1 — crate `server/` (axum)**,
com diretório de dados por sessão (cookie `gedocs_sid`) + TTL. Esboço já
iniciado; retomo quando autorizar.
