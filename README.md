# GeDoc IFES Toolkit

Aplicativo **desktop** (Tauri 2 + Vue 3) para consultar documentos públicos do
[GeDoc do IFES](https://gedoc.ifes.edu.br/faces/pesquisarDocumentos/pesquisarHistorico.xhtml)
por matrícula **SIAPE**: busca com paginação completa, filtra os documentos que
realmente citam o SIAPE no texto, baixa os PDFs com nome padronizado, classifica
e resume com IA (opcional) e gera relatório + pacote ZIP — tudo por uma
interface única.

> O portal roda em **JSF/PrimeFaces**: a busca exige `ViewState`, sessão
> (`jsessionid`) e requisições AJAX parciais; os ids do formulário são
> descobertos dinamicamente em runtime (resistem a redeploy do servidor).

## Versão Web (online)

Além do app desktop, há uma **versão web** — a mesma busca por SIAPE direto no
navegador, sem instalar nada:

- **App:** <https://gedocs.vercel.app>
- **API:** <https://gedoc-search-api.onrender.com> (health: `/api/health`)

Uso interno, **sem login**: cada visitante recebe uma sessão efêmera; os PDFs
(dados pessoais de terceiros) ficam isolados por sessão e são apagados por TTL
(~1h) — conformidade com a LGPD. Frontend estático na **Vercel** (proxy
`/api/*` → Render, mesma origem → cookie de sessão first-party), API (Docker,
axum) no **Render**, reusando o mesmo núcleo Rust do desktop (crate
`gedocs-core`, sem Tauri).

- **Guia do usuário (web):** [docs/guia-usuario-web.md](docs/guia-usuario-web.md)
- **Arquitetura / deploy / CI-CD:** [docs/arquitetura.md](docs/arquitetura.md)
- Plano e decisões: [docs/plano-web.md](docs/plano-web.md) · specs em [specs/](specs/)

> O **relatório** consolida os resumos da IA — clicar em **Baixar relatório**
> executa a IA (resume os documentos) e gera; sem IA a busca/PDF/ZIP funcionam
> normalmente. No plano free do Render, a API hiberna após ~15 min ociosa: a
> primeira busca depois disso leva ~30–60 s (cold start).

## Download / Instaladores

Baixe o instalador pronto na
[página de releases](https://github.com/ifesserra-lab/gedoc-ifes-busca/releases/latest)
(sempre a versão mais recente) ou direto por sistema operacional (v0.1.0):

| Sistema | Arquivo | Download |
| --- | --- | --- |
| **macOS** (Apple Silicon) | `.dmg` | [gedocs_0.1.0_aarch64.dmg](https://github.com/ifesserra-lab/gedoc-ifes-busca/releases/download/v0.1.0/gedocs_0.1.0_aarch64.dmg) |
| macOS (app avulso) | `.app.tar.gz` | [gedocs_aarch64.app.tar.gz](https://github.com/ifesserra-lab/gedoc-ifes-busca/releases/download/v0.1.0/gedocs_aarch64.app.tar.gz) |
| **Windows** (x64) | `.exe` (NSIS) | [gedocs_0.1.0_x64-setup.exe](https://github.com/ifesserra-lab/gedoc-ifes-busca/releases/download/v0.1.0/gedocs_0.1.0_x64-setup.exe) |
| Windows (x64) | `.msi` | [gedocs_0.1.0_x64_en-US.msi](https://github.com/ifesserra-lab/gedoc-ifes-busca/releases/download/v0.1.0/gedocs_0.1.0_x64_en-US.msi) |
| **Linux** | `.AppImage` | [gedocs_0.1.0_amd64.AppImage](https://github.com/ifesserra-lab/gedoc-ifes-busca/releases/download/v0.1.0/gedocs_0.1.0_amd64.AppImage) |
| Linux (Debian/Ubuntu) | `.deb` | [gedocs_0.1.0_amd64.deb](https://github.com/ifesserra-lab/gedoc-ifes-busca/releases/download/v0.1.0/gedocs_0.1.0_amd64.deb) |
| Linux (Fedora/RHEL) | `.rpm` | [gedocs-0.1.0-1.x86_64.rpm](https://github.com/ifesserra-lab/gedoc-ifes-busca/releases/download/v0.1.0/gedocs-0.1.0-1.x86_64.rpm) |

> macOS: build para **Apple Silicon** (aarch64). No primeiro uso, se o Gatekeeper
> bloquear, abra em **Ajustes do Sistema → Privacidade e Segurança → Abrir mesmo
> assim**. Os instaladores são gerados automaticamente pela
> [CI de release](.github/workflows/release.yml) a cada tag `v*`.

## Funcionalidades

- **Buscar por SIAPE** — coleta todas as páginas, sem duplicatas.
- **Filtro anti-falso-positivo** — só lista documentos cujo texto contém o
  SIAPE (evita casar número de processo por coincidência).
- **Download organizado** — PDFs nomeados `AAAA_NUMERO_ASSUNTO.pdf`, sem
  sobrescrever; **barra de progresso** ao baixar todos.
- **Classificação por categoria** — `keyword` (grátis, instantâneo) ou **IA**
  (Mistral, guiada pelas descrições de `categoria.json`); cache por documento.
- **Resumo por documento** — fiel ao texto, via IA; cache por documento.
- **Relatório + ZIP** — relatório consolidado (HTML/Markdown, "Salvar como PDF"
  no navegador) agrupado por categoria + ZIP de todos os PDFs.
- **CRUD de categorias** — cadastrar/editar/remover (persiste fora do
  repositório); a busca usa as categorias atualizadas.
- **Tema claro/escuro**, design minimalista (WCAG AA), estados de UI dedicados
  (carregando/vazio/erro/sucesso) com feedback via toast.

## Stack e arquitetura

- **Backend**: Rust (Tauri 2) — arquitetura MVC/DDD, testável sem rede.
  - `src-tauri/src/domain/` — modelo e regras puras (SIAPE, Documento, nome de
    arquivo, categoria).
  - `src-tauri/src/services/` — orquestração (repositório GeDoc, filtro,
    classificador, resumidor, cache, relatório, ZIP, download).
  - `src-tauri/src/ports/` — contratos (Repository/Strategy): `HttpPort`,
    `GedocRepository`, `Classificador`, `ChatIa`. A infra concreta
    (`reqwest`, `MistralClient`) fica atrás desses ports — por isso todo teste
    roda **offline** com dublês.
  - `src-tauri/src/commands/` — fronteira IPC (`#[tauri::command]`).
- **Frontend**: Vue 3 (`<script setup>` + TS) + Pinia + Vue Router + Nuxt UI 4
  (Tailwind v4). Estado/ViewModel em `app/src/stores/`, telas em
  `app/src/views/`, wrappers IPC tipados em `app/src/services/ipc.ts`.

```
gedocs/
├── src-tauri/            # backend Rust (Tauri) — Model/Controller
│   ├── src/{domain,services,ports,commands}/
│   ├── capabilities/     # permissões (opener, escopo $APPDATA)
│   └── tests/            # testes de integração + fixtures
├── app/                  # frontend Vue (View/estado)
│   ├── src/{views,components,stores,services,router,assets}/
│   └── tests/            # Vitest
├── config/
│   ├── categoria.json    # nome + descrição das categorias (base do LLM)
│   └── .env.example      # modelo de credenciais (copie p/ .env)
├── specs/                # Spec-Driven Development (spec/plan/tasks por feature)
├── docs/                 # ontologia, análise/projeto
├── src/                  # CLI Python (legado/referência — ver abaixo)
└── .github/workflows/    # CI
```

## Rodar (desenvolvimento)

Requisitos: **Rust** (stable), **Node 20+**. No Linux, as dependências de
sistema do Tauri (webkit2gtk-4.1, gtk-3, etc. — ver `.github/workflows/ci.yml`).

```bash
# dependências do frontend
cd app && npm ci && cd ..

# app desktop (janela nativa) — a partir da raiz do repositório
./app/node_modules/.bin/tauri dev
```

Fluxo na tela: digite o SIAPE → **Pesquisar** → lista agrupada por categoria.
Cada documento tem botão **PDF** (baixa e abre). No cabeçalho: **Baixar todos os
PDFs** (com progresso), **Baixar relatório**, **Baixar ZIP**, e o toggle
**Classificar e resumir com IA**. A aba **Categorias** faz o CRUD.

### IA (opcional)

A classificação/resumo com IA usa a API da Mistral. Sem chave, o app funciona
normalmente no modo `keyword` (o toggle de IA degrada para keyword).

```bash
cp config/.env.example config/.env   # e preencha MISTRAL_KEY
```

A chave é lida em runtime (`MISTRAL_API_KEY` ou `MISTRAL_KEY`, do ambiente ou de
`config/.env`); nunca é versionada nem registrada em log.

## Testes e CI

```bash
cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
cd app && npm test && npx vue-tsc --noEmit
```

Todos os testes rodam **offline** (rede e IA são dubladas; disco usa diretório
temporário) — Princípio VII (TDD). A [CI](.github/workflows/ci.yml) roda esses
mesmos checks em cada pull request.

## Privacidade (LGPD)

Os PDFs baixados, resumos, relatórios e o cache contêm **dados pessoais de
terceiros** (nomes e SIAPEs de membros de comissões). Nada disso é versionado:

- Arquivos gerados vão para o **diretório de dados do app**
  (`app_data_dir`/`app_config_dir`), **fora do repositório**.
- `config/.env` (chave da API) e `data/` estão no
  [.gitignore](.gitignore); só `config/.env.example` e `config/categoria.json`
  (rótulos genéricos, sem PII) são rastreados.

Use apenas para consulta de documentos **públicos** do IFES e respeite a
legislação.

## Desenvolvimento orientado por especificação (SDD)

O projeto segue [Spec-Driven Development](https://github.com/github/spec-kit):
cada feature tem `spec.md` → `plan.md` → `tasks.md` em [specs/](specs/), sob a
[constituição](.specify/memory/constitution.md) do projeto (OO, TDD, código
pequeno, padrões, privacidade/LGPD, issue-first). Toda task vira issue no
GitHub antes da implementação.

## Legado: CLI Python (referência)

A implementação original em Python (`src/*.py`) fez o mesmo fluxo por linha de
comando e serve de **referência** para o backend Rust. Continua utilizável:

```bash
pip install requests markdown          # + poppler (pdftotext) e Chrome p/ PDF
cp config/.env.example config/.env     # MISTRAL_KEY p/ categorização/resumo LLM

# buscar + baixar + página HTML + JSON
python3 src/buscar_gedoc.py 1998547 --baixar data/docs --html data/index.html --json data/r.json
# categorizar (keyword|llm)  ·  resumir (agrupado por categoria)  ·  markdown -> PDF
python3 src/categorizar.py      --json data/r.json --pdfs data/docs --md data/cat.md --modo llm
python3 src/resumir_mistral.py  --json data/r.json --pdfs data/docs --out data/resumo.md
python3 src/md_para_pdf.py      data/resumo.md --out data/resumo.pdf
```

Detalhes de cada script (flags, cache, throttle) no cabeçalho de cada arquivo em
[src/](src/).
