<!--
Sync Impact Report
- Version change: (template) → 1.0.0
- Ratification: initial adoption (constitution criada a partir do codigo existente)
- Principles defined:
  I. Fidelidade a Fonte Oficial
  II. Privacidade e LGPD (NON-NEGOTIABLE)
  III. Reprodutibilidade e Cache
  IV. Configuracao sobre Codigo
  V. Separacao de Camadas e DRY
- Added sections: Restricoes Tecnicas; Fluxo de Trabalho; Governance
- Removed sections: none
- Templates alignment:
  ✅ .specify/templates/plan-template.md (Constitution Check generico — compativel)
  ✅ .specify/templates/spec-template.md (sem conflito)
  ✅ .specify/templates/tasks-template.md (sem conflito)
  ✅ README.md (principios refletem o documentado)
- Deferred TODOs: none
-->

# GeDoc IFES Toolkit Constitution

## Core Principles

### I. Fidelidade a Fonte Oficial
O sistema apenas consulta documentos publicos do portal GeDoc do IFES.
- Os identificadores do formulario JSF/PrimeFaces MUST ser descobertos
  dinamicamente da pagina; ids autogerados NUNCA devem ser fixados no codigo.
- Dados apresentados ao usuario MUST derivar do conteudo real (texto do PDF,
  trecho da busca). Resumos e classificacoes NAO podem inventar informacao
  ausente na fonte.
- Toda extracao MUST degradar com seguranca: falha de um documento nao aborta
  o lote; a ausencia de um campo produz vazio, nao erro fatal.

Rationale: a utilidade da ferramenta depende de refletir fielmente atos
administrativos oficiais; alucinacao ou dado fabricado invalida o resultado.

### II. Privacidade e LGPD (NON-NEGOTIABLE)
Documentos contem dados pessoais de terceiros (nomes, SIAPE).
- PDFs, JSON de resultado, resumos e paginas geradas MUST permanecer em `data/`
  e NUNCA ser versionados.
- Segredos (`config/.env`) MUST ficar fora do controle de versao.
- Todo novo tipo de saida com PII MUST ser adicionado ao `.gitignore` antes de
  ser gerado; um commit NAO pode conter PII de terceiros.
- Aplica-se o principio do menor privilegio: coletar e expor o minimo necessario.

Rationale: conformidade com a LGPD e protecao de terceiros e requisito legal,
nao opcional.

### III. Reprodutibilidade e Cache
Reexecutar a pipeline MUST produzir o mesmo resultado sem custo desnecessario.
- Operacoes caras (chamadas a LLM, downloads) MUST ser idempotentes e usar
  cache por documento (chave = link do documento).
- Uma reexecucao com cache existente NAO deve repetir chamadas a API.
- Nomes de arquivo derivados MUST ser deterministicos (`AAAA_NUMERO_ASSUNTO`).

Rationale: previsibilidade, custo controlado e resiliencia a interrupcoes.

### IV. Configuracao sobre Codigo
Comportamento de dominio MUST ser ajustavel por configuracao, sem editar codigo.
- Categorias sao definidas em `config/categoria.json` (nome + descricao) e
  guiam a classificacao; alterar categorias NAO exige mudanca de codigo.
- Credenciais e ambiente vem de `config/.env`.
- Caminhos MUST ser resolvidos em relacao a raiz do projeto, permitindo execucao
  a partir de qualquer diretorio.

Rationale: adaptar o sistema a novos usos deve ser barato e sem risco de
regressao no codigo.

### V. Separacao de Camadas e DRY
A estrutura `src/ | config/ | data/` MUST ser respeitada.
- Codigo em `src/`, configuracao em `config/`, saidas em `data/`.
- Cada script tem responsabilidade unica (buscar, categorizar, resumir, pdf).
- Integracoes compartilhadas (ex.: cliente Mistral) MUST residir em um unico
  modulo reutilizavel; logica duplicada deve ser extraida.

Rationale: coesao e baixa duplicacao reduzem defeitos e facilitam manutencao.

## Restricoes Tecnicas

- Linguagem: Python 3.8+; dependencias minimas (`requests`, `markdown`).
- Ferramentas externas: `pdftotext` (poppler) para texto; Chrome/Chromium para
  PDF; API Mistral para classificacao/resumo.
- Chamadas a servicos externos MUST tratar rate limit (HTTP 429) com throttle e
  retry com backoff exponencial.
- Type hints SHOULD ser usados em codigo novo; erros de negocio MUST usar
  excecoes especificas e mensagens claras ao usuario (nao stack trace cru).

## Fluxo de Trabalho

Pipeline canonica, cada etapa consumindo a saida da anterior:

1. `buscar_gedoc.py` — busca + paginacao + filtro por SIAPE + download + HTML/JSON
2. `categorizar.py` — classificacao (keyword ou LLM via `categoria.json`)
3. `resumir_mistral.py` — resumo por documento, agrupado por categoria
4. `md_para_pdf.py` — Markdown -> PDF

- Saidas MUST ser validadas apos gerar (contagem, validade de PDF, zero falhas).
- Mudanca em codigo com efeito observavel MUST ser exercida de ponta a ponta
  antes de considerar concluida.

## Governance

Esta constituicao supersede outras praticas do projeto.
- Emendas MUST ser documentadas neste arquivo com atualizacao de versao
  (semver: MAJOR remocao/redefinicao incompativel; MINOR novo principio/secao;
  PATCH ajustes de texto).
- Todo commit/PR MUST verificar conformidade com os principios — em especial o
  Principio II (nenhuma PII de terceiros versionada).
- Complexidade adicional MUST ser justificada; na duvida, prevalece a
  simplicidade (YAGNI).

**Version**: 1.0.0 | **Ratified**: 2026-07-03 | **Last Amended**: 2026-07-03
