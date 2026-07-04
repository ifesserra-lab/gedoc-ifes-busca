# GeDoc IFES Toolkit — Épicos, User Stories e Tasks

**Versão:** 2.0.0 · **Data:** 2026-07-04 · **Origem:** `docs/ontology.yaml`

Backlog no formato **Epic → User Story → Task**. Cada User Story (US) é uma
fatia de valor priorizada (P1/P2/P3), independentemente testável, com critérios
de aceite em **Given/When/Then**. As Tasks são o trabalho técnico de cada US,
rastreável às regras de domínio (R1–R10) e módulos (`src/`).

> **Conceitos:** *Epic* = tema amplo (várias US). *User Story* = necessidade do
> usuário, pequena e entregável, com valor claro. *Task* = passo técnico de uma US.

## Sumário

- [Personas](#personas)
- [Mapa de épicos e US](#mapa-de-épicos-e-us)
- [EP1 — Consulta de documentos](#ep1--consulta-de-documentos)
- [EP2 — Categorização](#ep2--categorização)
- [EP3 — Síntese e relatório](#ep3--síntese-e-relatório)
- [EP4 — Interface web](#ep4--interface-web)
- [EP5 — Administração de categorias](#ep5--administração-de-categorias)
- [Requisitos não-funcionais](#requisitos-não-funcionais-transversais)
- [Success Criteria](#success-criteria-mensuráveis)
- [Rastreabilidade](#rastreabilidade)

## Personas

| Persona | Descrição | Objetivo |
| --- | --- | --- |
| **Analista** | Consulta documentos de um servidor por SIAPE. | Localizar, entender e baixar atos rapidamente. |
| **Admin de categorias** | Define critérios de classificação. | Ajustar categorias sem depender de dev. |
| **Mantenedor** | Desenvolve o sistema. | Robustez, privacidade, reprodutibilidade. |

## Mapa de épicos e US

| Epic | User Stories | Prioridade |
| --- | --- | --- |
| EP1 Consulta de documentos | US1.1 Buscar · US1.2 Filtrar por SIAPE · US1.3 Baixar PDFs | P1, P1, P2 |
| EP2 Categorização | US2.1 Classificar · US2.2 Classificar via LLM (config) | P2, P2 |
| EP3 Síntese e relatório | US3.1 Resumir doc · US3.2 Relatório PDF | P2, P3 |
| EP4 Interface web | US4.1 Buscar no navegador · US4.2 Baixar relatório/arquivos | P1, P2 |
| EP5 Administração de categorias | US5.1 CRUD de categorias | P3 |

---

## EP1 — Consulta de documentos

Tema: recuperar do GeDoc todos os documentos de um servidor e deixá-los locais.
**Entidades:** ResultadoBusca, Documento, Servidor, Repositorio.

### US1.1 — Buscar e coletar todos os documentos (P1)

> **Como** Analista, **quero** buscar por SIAPE e obter todos os documentos,
> **para** não precisar paginar manualmente no portal.

**Independente/testável:** com um SIAPE conhecido, retorna a lista completa.

**Acceptance**
1. **Given** um SIAPE válido, **When** submeto a busca, **Then** o total coletado
   é igual ao total informado pelo portal (todas as páginas).
2. **Given** o portal em JSF, **When** a página muda de layout de IDs, **Then** o
   sistema descobre os IDs em runtime e a busca funciona (R8).

| Task | Descrição | Regra/Fonte |
| --- | --- | --- |
| T1.1.1 | Abrir sessão + descobrir IDs/ViewState. | R8 · `GedocClient.abrir` |
| T1.1.2 | Submeter busca AJAX e percorrer paginação. | `buscar/pagina` |
| T1.1.3 | Deduplicar por `link` e persistir `resultado_<siape>.json`. | `coletar` |

### US1.2 — Filtrar por SIAPE (P1)

> **Como** Analista, **quero** ver só documentos que citam o SIAPE, **para**
> excluir falsos-positivos (número usado como processo).

**Acceptance**
1. **Given** um documento cujo trecho contém o SIAPE, **When** filtro, **Then**
   ele entra em `documentos` (R2).
2. **Given** um documento sem o SIAPE no trecho, **When** filtro, **Then** vai
   para `descartados` e a contagem reflete isso.
3. **Given** um termo não numérico, **When** busco, **Then** é rejeitado (R10).

| Task | Descrição | Regra/Fonte |
| --- | --- | --- |
| T1.2.1 | Validar termo `^[0-9]{5,8}$`. | R10 |
| T1.2.2 | Extrair `siapes[]` do trecho. | `_RE_SIAPE` |
| T1.2.3 | Marcar `contem_siape` (termo em qualquer ponto do trecho). | R2 · `filtrar_por_siape` |

### US1.3 — Baixar PDFs organizados (P2)

> **Como** Analista, **quero** baixar os PDFs com nomes padronizados, **para**
> arquivar e localizar facilmente.

**Acceptance**
1. **Given** documentos válidos, **When** baixo, **Then** todos os arquivos são
   `%PDF` válidos em `data/documentos_<siape>/`.
2. **Given** dois documentos com mesmo nome derivado, **When** baixo, **Then** o
   segundo recebe sufixo (nenhum sobrescrito).

| Task | Descrição | Regra/Fonte |
| --- | --- | --- |
| T1.3.1 | Baixar PDF de cada documento. | `baixar` |
| T1.3.2 | Nomear `AAAA_NUMERO_ASSUNTO.pdf`. | R3 · `nome_arquivo` |
| T1.3.3 | Tratar colisão de nomes. | R3 |

---

## EP2 — Categorização

Tema: rotular cada documento por tipo de ato. **Entidades:** Documento, Categoria, Cache.

### US2.1 — Classificar documentos (P2)

> **Como** Analista, **quero** os documentos classificados por categoria,
> **para** navegar por tipo de ato.

**Acceptance**
1. **Given** um documento, **When** classifico, **Then** recebe exatamente uma
   categoria (R4).
2. **Given** modo `keyword`, **When** classifico, **Then** nenhuma chamada de API
   é feita.

| Task | Descrição | Regra/Fonte |
| --- | --- | --- |
| T2.1.1 | Classificar por regex (modo keyword). | `classificar` |
| T2.1.2 | Atribuir `Outros` quando nada casa. | R4 |

### US2.2 — Classificar via LLM guiada por configuração (P2)

> **Como** Admin de categorias, **quero** que a IA classifique usando as
> descrições que defino, **para** melhorar a precisão sem alterar código.

**Acceptance**
1. **Given** `categoria.json` com descrições, **When** classifico em modo `llm`,
   **Then** a categoria escolhida está na lista (fora dela → `Outros`) (R4, R5).
2. **Given** um documento já classificado, **When** reexecuto, **Then** usa cache
   (sem nova chamada) (R6).
3. **Given** HTTP 429, **When** classifico em lote, **Then** aplica throttle+retry
   e conclui (R9).

| Task | Descrição | Regra/Fonte |
| --- | --- | --- |
| T2.2.1 | Carregar categorias de `config/categoria.json`. | R5 |
| T2.2.2 | Prompt LLM com nome+descrição; validar retorno. | R4 · `classificar_llm` |
| T2.2.3 | Cache por `link` + throttle/retry. | R6, R9 |

---

## EP3 — Síntese e relatório

Tema: resumir e consolidar. **Entidades:** Documento, Resumo, Cache.

### US3.1 — Resumir cada documento (P2)

> **Como** Analista, **quero** um resumo objetivo de cada documento, **para**
> entender o teor sem abrir o PDF.

**Acceptance**
1. **Given** o texto do PDF, **When** resumo, **Then** o resumo deriva do texto e
   não inventa dados (R1).
2. **Given** falha em um documento, **When** processo o lote, **Then** os demais
   seguem e o item marca falha (R9).

| Task | Descrição | Regra/Fonte |
| --- | --- | --- |
| T3.1.1 | Extrair texto (`pdftotext`), fallback ao trecho. | `extrair_texto` |
| T3.1.2 | Resumir (Mistral) 2–3 frases, sem alucinar. | R1 · `resumir` |
| T3.1.3 | Cache por `link`. | R6 |

### US3.2 — Gerar relatório PDF agrupado (P3)

> **Como** Analista, **quero** um relatório em PDF agrupado por categoria,
> **para** compartilhar/imprimir.

**Acceptance**
1. **Given** documentos resumidos, **When** gero o relatório, **Then** há tabela
   de categorias + uma seção por categoria com as portarias.
2. **Given** o Markdown, **When** converto, **Then** sai um PDF `%PDF` válido.

| Task | Descrição | Regra/Fonte |
| --- | --- | --- |
| T3.2.1 | Markdown agrupado (tabela + seções + docs). | `gerar_markdown` |
| T3.2.2 | Converter Markdown→PDF. | `md_para_pdf` |

---

## EP4 — Interface web

Tema: usar tudo pelo navegador. **Entidades:** todas.

### US4.1 — Buscar pelo navegador (P1)

> **Como** Analista, **quero** digitar o SIAPE numa tela, **para** rodar a
> pipeline sem linha de comando.

**Acceptance**
1. **Given** a tela, **When** informo um SIAPE inválido, **Then** é bloqueado com
   mensagem (R10).
2. **Given** um SIAPE válido, **When** busco, **Then** vejo total, chips por
   categoria e lista com resumos.

| Task | Descrição | Regra/Fonte |
| --- | --- | --- |
| T4.1.1 | Tela com estados busca/loading/erro/resultado. | `app.html` |
| T4.1.2 | `POST /api/buscar` orquestra a pipeline. | `run_pipeline` |

### US4.2 — Baixar relatório e arquivos (P2)

> **Como** Analista, **quero** baixar o PDF do resumo e o ZIP dos documentos,
> **para** guardar/compartilhar.

**Acceptance**
1. **Given** um resultado, **When** clico em PDF do resumo, **Then** baixa
   `application/pdf`.
2. **Given** um resultado, **When** clico em baixar todos, **Then** baixa um ZIP
   dos PDFs; caminho individual é sanitizado (sem traversal).

| Task | Descrição | Regra/Fonte |
| --- | --- | --- |
| T4.2.1 | `GET /api/pdf` e `GET /api/zip`. | `app.py` |
| T4.2.2 | `GET /api/doc` com nome sanitizado. | `app.py` |

---

## EP5 — Administração de categorias

Tema: gerir os critérios de classificação. **Entidades:** Categoria.

### US5.1 — CRUD de categorias (P3)

> **Como** Admin de categorias, **quero** cadastrar/editar/remover categorias,
> **para** ajustar a classificação sem alterar código.

**Acceptance**
1. **Given** a tela, **When** salvo uma categoria com nome, **Then** persiste em
   `config/categoria.json` (R5).
2. **Given** um nome já existente, **When** salvo, **Then** é rejeitado.
3. **Given** payload sem nome, **When** envio ao servidor, **Then** HTTP 400.

| Task | Descrição | Regra/Fonte |
| --- | --- | --- |
| T5.1.1 | `GET/POST /api/categorias` (ler/gravar). | R5 · `app.py` |
| T5.1.2 | Modal criar/editar; bloquear duplicado. | `categorias_app.html` |
| T5.1.3 | Validar payload no servidor. | `gravar_categorias` |

---

## Requisitos não-funcionais (transversais)

Não são US — são restrições que valem para todas (viram FR + constituição).

| ID | Requisito | Regra |
| --- | --- | --- |
| NFR-Priv | PII (tudo em `data/`) e segredos nunca versionados. | R7 |
| NFR-Idem | Etapas caras são idempotentes via cache por `link`. | R6 |
| NFR-Resi | Retry+backoff+throttle; falha isolada não derruba o lote. | R9 |
| NFR-Cfg | Categorias e credenciais fora do código (`config/`). | R5 |
| NFR-Fid | Nenhuma informação inventada; tudo deriva da fonte. | R1 |

## Success Criteria (mensuráveis)

| ID | Métrica |
| --- | --- |
| SC-001 | 100% dos documentos retornados pelo portal são coletados (sem perda de página). |
| SC-002 | 0 falsos-positivos: todo documento listado contém o SIAPE no texto. |
| SC-003 | Reexecução de um SIAPE já consultado faz 0 novas chamadas de API. |
| SC-004 | 100% dos PDFs baixados são arquivos `%PDF` válidos. |
| SC-005 | Nenhum arquivo com PII de terceiros presente no repositório. |

## Rastreabilidade

**Regra → US**

| Regra | User Stories |
| --- | --- |
| R1 Fidelidade | US3.1, US3.2 |
| R2 Filtro SIAPE | US1.2 |
| R3 Nome determinístico | US1.3 |
| R4 Uma categoria | US2.1, US2.2 |
| R5 Categorias=config | US2.2, US5.1 |
| R6 Idempotência | US2.2, US3.1 |
| R7 Privacidade | NFR-Priv |
| R8 IDs dinâmicos | US1.1 |
| R9 Degradação segura | US2.2, US3.1 |
| R10 SIAPE válido | US1.2, US4.1 |

**Entidade → Épicos**

| Entidade | Épicos |
| --- | --- |
| Servidor | EP1 |
| Documento | EP1, EP2, EP3 |
| Categoria | EP2, EP5 |
| ResultadoBusca | EP1 |
| Resumo | EP3 |
| Cache | EP2, EP3 |
| Repositorio | EP1 |

---

### Uso no GitHub Spec Kit

Este backlog alimenta o fluxo do spec-kit:

- **User Stories** (com P1/P2/P3 e Given/When/Then) → `spec.md` (`/speckit-specify`).
- **Entidades** da ontologia → *Key Entities* do `spec.md`.
- **NFR + Regras (R1–R10)** → *Functional Requirements* e checks da constituição.
- **Success Criteria** → seção *Success Criteria* do `spec.md`.
- **Tasks** (agrupadas por US) → `tasks.md` (`/speckit-tasks`).

### Manutenção
Atualizar quando `docs/ontology.yaml` mudar. Toda nova US deve ter prioridade,
Given/When/Then e ao menos uma task e uma regra associadas.
