# Implementation Plan: Versão Web (uso interno, sem login, sessão efêmera + TTL)

**Branch**: `003-versao-web` | **Date**: 2026-07-08 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/003-versao-web/spec.md`

Apoio: [docs/plano-web.md](../../docs/plano-web.md), [docs/brief-web.md](../../docs/brief-web.md).

## Summary

Publicar uma versão web do app (hoje desktop Tauri) reaproveitando o
núcleo Rust existente. O frontend Vue é compilado estático e hospedado na
Vercel; uma API HTTP (axum) reusa os use-cases puros de `gedocs_lib`
(`executar`, `executar_download`, `executar_gerar_relatorio`,
`montar_zip`, `categorias::*`) e expõe endpoints que espelham os comandos
IPC. Sem login: cada visitante recebe uma sessão efêmera; os PDFs (PII de
terceiros) são isolados por sessão e apagados por TTL de 1h. Categorias
são configuração global. A API roda em container (Fly/Railway); a Vercel
só serve o estático. O mesmo `ipc.ts` escolhe o transporte em runtime
(`invoke` no desktop, `fetch` no web), mantendo stores/views inalteradas.

## Technical Context

**Language/Version**: Rust 1.80+ (API); TypeScript 5 + Vue 3.5 (frontend).

**Primary Dependencies**: API — `axum`, `tower-http` (cors + limit),
`tokio`, e `gedocs_lib` (reuso do núcleo). Frontend — Vue 3, Vite, Nuxt
UI, Pinia, vue-router (já existentes).

**Storage**: efêmero por sessão em diretório temporário do servidor
(`<data>/sessions/<sid>/{documentos,relatorios,cache}`); sem banco.
`categoria.json` global no data dir, semeado de `config/categoria.json`.

**Testing**: `cargo test` (unit + integração dos handlers, com dublês —
sem rede real, Princípio VII); `vitest` (adaptador `ipc.ts` dual-mode).

**Target Platform**: API em container Linux; frontend estático em
navegadores modernos (Vercel) e reutilizado pelo app desktop Tauri.

**Project Type**: web (frontend + backend), reusando o núcleo do desktop.

**Performance Goals**: latência dominada pelo portal/IA; UI responsiva com
os 5 estados (idle/loading/vazio/erro/sucesso). Rate limit contém abuso.

**Constraints**: sem persistência de longo prazo; TTL de sessão = 1h de
inatividade (configurável); chave de IA só no servidor; CORS restrito ao
domínio Vercel; sem PII em logs (Princípio II).

**Scale/Scope**: uso interno; dezenas de usuários simultâneos; 7 user
stories, ~8 endpoints.

## Constitution Check

*GATE: passar antes da Fase 0; reavaliar após a Fase 1.*

| Princípio | Status | Como o plano atende |
|---|---|---|
| I. Fidelidade à fonte | PASS | reusa o mesmo scraping/parse; conteúdo inalterado; degradação segura já no núcleo. |
| II. Privacidade/LGPD (NN) | PASS | sessão efêmera isolada + TTL 1h; PDFs em dir efêmero (fora do VCS); sem PII em logs; chave IA server-only. |
| III. Reprodutibilidade/cache | PASS | reusa o cache por link (classificação/resumo). |
| IV. Config sobre código | PASS | TTL, CORS, rate limit, chave IA via env/config; categorias em `categoria.json`. |
| V. Camadas/DRY | PASS | API fina (controller HTTP) sobre o núcleo; zero regra de negócio duplicada. |
| VI. Orientação a objetos | PASS | reusa entidades/serviços de domínio; nada de regra na borda HTTP. |
| VII. TDD (NN) | PASS | testes de handler/adaptador antes do código; dublês, sem rede. |
| VIII. Código pequeno | PASS | handlers curtos delegando aos use-cases; YAGNI (sem banco/auth). |
| IX. Padrões/MVC | PASS | controller HTTP (axum) análogo aos `#[tauri::command]`; View Vue única; estado em Pinia. |
| X. Issue-first (NN) | PASS (processo) | tasks viram issues (`/speckit-taskstoissues`) antes de implementar. |
| XI. Agentes especializados | PASS (processo) | Rust/axum → `tauri-mvc-expert`; qualquer UI → `ui-ux-designer`; PR → `pr-reviewer`. |
| XII. Qualidade UI/UX | PASS | reusa design system e os 5 estados; mudança é de transporte, não de telas. |

**Resultado do gate**: PASS. Violação de "stack alvo = desktop" é
extensão justificada (ver Complexity Tracking), não quebra de princípio.

## Project Structure

### Documentation (this feature)

```text
specs/003-versao-web/
├── plan.md              # este arquivo
├── research.md          # Fase 0
├── data-model.md        # Fase 1
├── quickstart.md        # Fase 1
├── contracts/
│   └── http-api.md      # Fase 1 — contrato dos endpoints
├── checklists/
│   └── requirements.md  # do /speckit-specify
└── tasks.md             # /speckit-tasks (ainda não)
```

### Source Code (repository root)

```text
app/                     # frontend Vue (existente) — serve desktop + web
├── src/
│   ├── services/ipc.ts  # ALTERAR: dual-mode (invoke | fetch) por runtime
│   └── ...              # stores/views inalteradas
├── .env.example         # NOVO: VITE_API_URL
└── tests/               # vitest: testes do adaptador dual-mode

src-tauri/               # núcleo Rust (existente) — reusado como lib
└── src/                 # domain/services/ports/commands (sem mudança na v1)

server/                  # NOVO — API HTTP (axum) que reusa gedocs_lib
├── Cargo.toml           # path-dep de ../src-tauri (opção A1)
├── src/
│   ├── main.rs          # bootstrap axum, rotas, estado, camadas
│   ├── rotas.rs         # handlers finos → use-cases do núcleo
│   ├── sessao.rs        # middleware de sessão (cookie) + resolução de dir
│   ├── ttl.rs           # job de limpeza por TTL
│   └── erro.rs          # AppError -> resposta HTTP ({tipo,mensagem})
├── tests/               # cargo test: integração dos endpoints (dublês)
├── Dockerfile           # multi-stage; runtime com libs necessárias
└── fly.toml             # exemplo de deploy do container

vercel.json              # NOVO (raiz) — build/estático do frontend + SPA
```

**Structure Decision**: manter o monorepo atual (`app/` + `src-tauri/`) e
adicionar um crate `server/` que **reusa** `gedocs_lib` como biblioteca
(opção A1 — ver research.md). Frontend inalterado salvo `ipc.ts`. Deploy
dividido: Vercel (estático) + container (API).

## Complexity Tracking

| Violação | Por que é necessária | Alternativa simples rejeitada porque |
|---|---|---|
| Novo alvo além do desktop (API axum + container + Vercel) | Levar o app à web exige um backend HTTP (scraping por CORS, chave IA server-only, timeouts de serverless) | "Só publicar o frontend" não funciona: as chamadas de IPC não existem no browser. |
| Crate `server/` linka o Tauri (A1) | Reuso imediato do núcleo sem refatorar; entrega a web rápido | A2 (extrair `gedocs-core` sem Tauri) é mais limpo mas atrasa a v1; fica como follow-up documentado. |
| Middleware de sessão + job de TTL | Sem login, é o que garante isolamento/expiração de PII (Princípio II) | "Dir único compartilhado" comingla PII de terceiros — viola LGPD. |
