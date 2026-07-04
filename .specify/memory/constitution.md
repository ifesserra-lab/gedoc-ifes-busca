<!--
Sync Impact Report
- Version change: 1.0.0 → 1.1.0  (MINOR: novos principios e secao de stack)
- Ratification: 2026-07-03 (mantida) | Last Amended: 2026-07-04
- Principles:
  I. Fidelidade a Fonte Oficial       (mantido)
  II. Privacidade e LGPD              (mantido, NON-NEGOTIABLE)
  III. Reprodutibilidade e Cache      (mantido)
  IV. Configuracao sobre Codigo       (mantido)
  V. Separacao de Camadas e DRY       (mantido)
  VI. Orientacao a Objetos            (novo)
  VII. Test-Driven Development        (novo, NON-NEGOTIABLE)
  VIII. Codigo Pequeno e Simples      (novo)
  IX. Padroes de Projeto              (novo)
  X. Issue-First no GitHub            (novo, NON-NEGOTIABLE)
- Added sections: nenhuma nova (Restricoes Tecnicas e Fluxo de Trabalho atualizados)
- Removed sections: none
- Templates alignment:
  ✅ .specify/templates/plan-template.md (Constitution Check — cobre TDD/OO/patterns)
  ✅ .specify/templates/spec-template.md (sem conflito)
  ✅ .specify/templates/tasks-template.md (tasks de teste primeiro; issue-first)
  ⚠ README.md (atualizar stack alvo Tauri 2.0 + Vue quando a migracao iniciar)
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
- Caminhos MUST ser resolvidos em relacao a raiz do projeto.

Rationale: adaptar o sistema a novos usos deve ser barato e sem risco de
regressao no codigo.

### V. Separacao de Camadas e DRY
A estrutura `src/ | config/ | data/` MUST ser respeitada.
- Codigo em `src/`, configuracao em `config/`, saidas em `data/`.
- Cada modulo tem responsabilidade unica.
- Integracoes compartilhadas (ex.: cliente Mistral) MUST residir em um unico
  modulo reutilizavel; logica duplicada deve ser extraida.

Rationale: coesao e baixa duplicacao reduzem defeitos e facilitam manutencao.

### VI. Orientacao a Objetos
O dominio MUST ser modelado em objetos coesos, com responsabilidade unica.
- As entidades da ontologia (`docs/ontology.yaml`) MUST corresponder a classes
  com encapsulamento; estado interno nao e exposto diretamente.
- Preferir composicao a heranca; dependencias explicitas (injecao) em vez de
  globais/singletons ocultos.
- Regras de dominio MUST viver nos objetos de dominio, nao espalhadas em
  controllers/UI.

Rationale: OO bem aplicado localiza a mudanca e torna o dominio testavel.

### VII. Test-Driven Development (NON-NEGOTIABLE)
Todo comportamento novo MUST nascer de um teste que falha primeiro.
- Ciclo Red-Green-Refactor obrigatorio: teste falha → codigo minimo → refatora.
- As regras de dominio (R1-R10 da ontologia) MUST ter testes automatizados.
- Testes MUST rodar sem depender de rede (portal/LLM sao dublados/mocked).
- Um bug corrigido MUST vir acompanhado de um teste que o reproduz.

Rationale: TDD garante regressao controlada e projeto guiado por contrato.

### VIII. Codigo Pequeno e Simples
Manter unidades pequenas e legiveis.
- Funcoes/metodos curtos e com um proposito; arquivos pequenos.
- Aplicar YAGNI: nao implementar o que nao e exigido por uma US/regra.
- Complexidade adicional MUST ser justificada; na duvida, simplifica.

Rationale: codigo pequeno e mais facil de testar, revisar e evoluir.

### IX. Padroes de Projeto
Usar padroes de projeto para resolver problemas reais, sem over-engineering.
- Aplicar padroes GoF/arquiteturais quando houver problema recorrente; nomear e
  justificar o padrao escolhido.
- Arquitetura de UI segue MVC: Model/Controller em Rust, View em Vue, estado em
  Pinia (ver principio de stack).
- Padrao NAO deve ser introduzido sem necessidade demonstrada.

Rationale: padroes comunicam intencao e reduzem acoplamento quando bem usados.

### X. Issue-First no GitHub (NON-NEGOTIABLE)
Nenhuma execucao/implementacao comeca sem uma issue aberta.
- Antes de codificar qualquer tarefa, MUST existir uma issue no GitHub
  descrevendo objetivo, criterio de aceite e escopo.
- Commits e PRs MUST referenciar a issue (ex.: `#123`).
- Tasks derivadas do spec-kit MUST virar issues antes da implementacao
  (`/speckit-taskstoissues` pode ser usado).

Rationale: rastreabilidade, revisao e historico do "porque" de cada mudanca.

## Restricoes Tecnicas

Stack alvo do produto (aplicacao desktop):

- **Desktop:** Tauri 2.0 (backend em Rust).
- **Frontend:** Vue 3 (Composition API, `<script setup>`, TypeScript) + Pinia.
- **Arquitetura:** MVC — Model/Controller em Rust (`src-tauri/`), View em Vue,
  ViewModel/estado em Pinia; IPC tipado entre camadas.
- **Testes:** unitarios + integracao; disciplina de TDD; especialista em teste,
  Tauri e padroes de projeto conduz as decisoes.
- **Seguranca:** menor privilegio nas `capabilities`/permissions do Tauri;
  validar toda entrada de IPC.

Implementacao atual (pipeline Python em `src/`: busca, categorizacao, resumo,
web) e considerada legado a ser migrado para a stack alvo, preservando os
principios de dominio (I-V). Chamadas externas MUST tratar rate limit (429) com
throttle e retry com backoff.

## Fluxo de Trabalho

1. **Abrir issue no GitHub** (obrigatorio, principio X) antes de qualquer tarefa.
2. Especificar/planejar via spec-kit (`spec.md` → `plan.md` → `tasks.md`).
3. **TDD:** escrever teste que falha → implementar minimo → refatorar.
4. Commits/PRs referenciam a issue; PR so integra com testes verdes.
5. Validar saidas de ponta a ponta antes de concluir.

Pipeline de dominio (etapas encadeadas, idempotentes via cache):
`buscar → categorizar → resumir → pdf`.

## Governance

Esta constituicao supersede outras praticas do projeto.
- Emendas MUST ser documentadas aqui com atualizacao de versao (semver:
  MAJOR remocao/redefinicao incompativel; MINOR novo principio/secao; PATCH
  ajustes de texto).
- Todo commit/PR MUST verificar conformidade com os principios — em especial
  II (PII), VII (TDD) e X (issue-first).
- Complexidade adicional MUST ser justificada; na duvida, prevalece a
  simplicidade (YAGNI).

**Version**: 1.1.0 | **Ratified**: 2026-07-03 | **Last Amended**: 2026-07-04
