# Research — Versão Web (Fase 0)

Resolve as incógnitas técnicas do [plan.md](./plan.md). Formato por item:
Decisão / Justificativa / Alternativas.

## R1. Estratégia de reuso do núcleo Rust

- **Decisão**: A1 — criar crate `server/` com `path`-dep de
  `../src-tauri` (`gedocs_lib`) e chamar os use-cases puros já existentes.
- **Justificativa**: os use-cases (`executar`, `executar_download`,
  `resolver_caminho_abertura`, `executar_gerar_relatorio`,
  `montar_zip`, `services::categorias::*`) não dependem de `AppHandle` —
  reuso imediato, zero duplicação de regra (Princípios V/VIII). Entrega a
  web rápido.
- **Alternativas**: A2 — extrair `gedocs-core` sem Tauri (imagem menor,
  sem webkit), porém exige refatorar `src-tauri` (tornar `tauri` opcional,
  mover DTOs/use-cases). Fica como follow-up (Fase 6 do plano-web).
  Custo do A1: o build da API compila o Tauri e o runtime precisa das
  libs (webkit) no container.

## R2. Mecanismo de sessão sem login

- **Decisão**: cookie opaco `gedocs_sid` (valor aleatório), `HttpOnly`.
  Em produção o front (Vercel) e a API (Render/Fly) ficam em domínios
  diferentes → o cookie precisa ser `SameSite=None; Secure` para voltar no
  `fetch` cross-site (com `Lax` o navegador não o envia e a sessão nunca
  persiste). Em dev local (same-site) usa `SameSite=Lax`. Emitido no 1º
  acesso; mapeia para `<data>/sessions/<sid>/`.
- **Justificativa**: sem login, o cookie é o portador natural da
  identidade efêmera; opaco e `HttpOnly` evita manipulação/XSS. Isola PII
  por sessão (Princípio II, FR-011).
- **Alternativas**: token em `localStorage` (exposto a XSS — rejeitado);
  header custom por request (mais fricção no frontend, sem ganho).

## R3. Expiração e limpeza (TTL)

- **Decisão**: TTL de **1h de inatividade** (FR-012). Cada request
  atualiza `ultima_atividade` da sessão. Uma tarefa em background varre os
  diretórios de sessão a cada ~10 min e remove os que passaram do TTL.
- **Justificativa**: sliding TTL casa com "inatividade"; varredura
  periódica é simples e suficiente (Princípio VIII). Nada persiste entre
  sessões.
- **Alternativas**: limpeza on-access apenas (dir órfão de sessão nunca
  mais acessada nunca seria limpo — rejeitado); cron externo (mais
  infra — desnecessário).

## R4. Armazenamento efêmero

- **Decisão**: `GEDOCS_DATA_DIR` aponta para diretório temporário do
  container (tmpfs/efêmero), **sem volume persistente**. `categoria.json`
  global vive em `<data>/categoria.json` (semeado de
  `config/categoria.json`).
- **Justificativa**: alinha com "efêmero por sessão + TTL"; simplifica
  deploy; reduz superfície de retenção de PII.
- **Alternativas**: volume persistente / object storage (R2/S3) — fora de
  escopo v1 (spec §Out of Scope).

## R5. CORS

- **Decisão**: `tower-http` `CorsLayer` restrito à origem do frontend
  (`GEDOCS_CORS_ORIGIN` = domínio Vercel), com `credentials` habilitado
  (para enviar o cookie de sessão).
- **Justificativa**: menor privilégio (Princípio II); cookie de sessão
  exige `Access-Control-Allow-Credentials` + origem explícita (não `*`).
- **Alternativas**: CORS aberto (`*`) — incompatível com cookies
  credenciados e inseguro (rejeitado).

## R6. Proteção contra abuso (rate limit + tamanho)

- **Decisão**: rate limit por origem (ex.: `tower_governor`) + limite de
  tamanho de corpo (`tower-http` `RequestBodyLimitLayer`). Exceder →
  erro amigável, sem processar (FR-016, SC-007).
- **Justificativa**: sem login, a API está exposta; scraping/IA são caros.
- **Alternativas**: nada (rejeitado pela clarificação); WAF externo (fora
  de escopo v1).

## R7. Contrato de erro

- **Decisão**: reusar `AppError` serializado como `{tipo, mensagem}`
  (já é `#[serde(tag="tipo", content="mensagem")]`). Mapear para status:
  400 (SIAPE inválido / categoria sem nome / nome duplicado),
  502 (falha portal / IA), 501 (não implementado), 500 (falha arquivo),
  429 (rate limit), 413 (corpo grande).
- **Justificativa**: mantém o mesmo contrato do desktop — `ipc.ts::
  mensagemDeErro` funciona sem alteração (FR-014).
- **Alternativas**: novo formato de erro web (duplicaria mapeamento —
  rejeitado).

## R8. Frontend dual-mode (desktop + web)

- **Decisão**: `ipc.ts` detecta `'__TAURI_INTERNALS__' in window`. Desktop
  → `invoke()`. Web → `fetch(VITE_API_URL, { credentials: 'include' })`.
  Assinaturas exportadas inalteradas.
- **Justificativa**: um só frontend, sem duplicar lógica (Princípio V);
  stores/views não mudam (FR-015).
- **Alternativas**: dois frontends / build flags — mais manutenção
  (rejeitado).

## R9. Entrega de arquivos no navegador

- **Decisão**: `abrir_documento` → `GET /api/documento/:siape/:arquivo`
  aberto em nova aba (browser exibe o PDF). `gerar_relatorio` → gera no
  servidor e `GET /api/relatorio/:siape` abre o HTML em nova aba.
  `baixar_zip` → `GET /api/zip/:siape` com `Content-Disposition:
  attachment` (download).
- **Justificativa**: substitui "abrir com app do SO" pelo comportamento
  nativo do navegador; simples e sem plugin.
- **Alternativas**: streaming/base64 inline (pior para arquivos grandes —
  rejeitado).

## R10. Deploy

- **Decisão**: frontend estático na **Vercel** (`vercel.json` com
  build/estático + rewrite SPA); API em **container** (Fly.io/Railway) via
  `server/Dockerfile` (multi-stage; runtime com libs necessárias para A1)
  + `fly.toml`. Secrets: `MISTRAL_API_KEY`. Envs: `GEDOCS_CORS_ORIGIN`,
  `GEDOCS_DATA_DIR`, `GEDOCS_SESSION_TTL`.
- **Justificativa**: Vercel serverless não roda a API Rust persistente
  (scraping/IA/timeout); separar responsabilidades é o caminho limpo.
- **Alternativas**: tudo em serverless (rewrite TS + timeouts —
  rejeitado); tudo num container servindo também o estático (possível,
  mas perde a CDN/DX da Vercel).

## Incógnitas remanescentes

Nenhuma bloqueante. Tudo com decisão default registrada; ajustes finos
(intervalo exato de varredura, limites numéricos de rate) ficam para a
implementação/tuning.
