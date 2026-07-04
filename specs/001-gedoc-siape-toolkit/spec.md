# Feature Specification: GeDoc IFES Toolkit — Consulta por SIAPE

**Feature Branch**: `001-gedoc-siape-toolkit`

**Created**: 2026-07-04

**Status**: Draft

**Input**: Consulta de documentos públicos do portal GeDoc do IFES por matrícula
SIAPE: busca, filtro por SIAPE, download organizado, classificação configurável,
resumo, relatório PDF e interface web com CRUD de categorias.
(Referências: `docs/ontology.yaml`, `docs/analise-e-projeto.md`, `docs/epics-e-tasks.md`.)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Buscar e coletar todos os documentos de um servidor (Priority: P1)

Um analista informa a matrícula SIAPE e recebe a lista completa de documentos do
servidor no portal, sem precisar navegar página a página.

**Why this priority**: É a porta de entrada do sistema; sem a coleta completa
nenhuma outra funcionalidade tem valor.

**Independent Test**: Informar um SIAPE conhecido e verificar que o total
recuperado é igual ao total informado pelo portal.

**Acceptance Scenarios**:

1. **Given** um SIAPE válido, **When** o analista realiza a busca, **Then** o
   sistema retorna todos os documentos (todas as páginas), sem duplicatas.
2. **Given** o portal instável, **When** ocorre um erro temporário, **Then** o
   sistema tenta novamente e conclui, ou informa o erro claramente.

---

### User Story 2 - Filtrar por SIAPE evitando falso-positivo (Priority: P1)

O analista quer ver apenas documentos que realmente citam o servidor, não
aqueles em que o número aparece por coincidência (ex.: número de processo).

**Why this priority**: Sem o filtro, os resultados incluem documentos de outras
pessoas, comprometendo a confiança na ferramenta.

**Independent Test**: Buscar um SIAPE e confirmar que todo documento listado
contém o número no texto e que os demais aparecem como "descartados".

**Acceptance Scenarios**:

1. **Given** um documento cujo texto cita o SIAPE, **When** o filtro é aplicado,
   **Then** ele aparece na lista de resultados.
2. **Given** um documento em que o número aparece fora do contexto do servidor,
   **When** o filtro é aplicado, **Then** ele é separado como descartado.
3. **Given** um termo que não é um SIAPE válido, **When** a busca é solicitada,
   **Then** o sistema rejeita a entrada com mensagem clara.

---

### User Story 3 - Buscar pelo navegador (Priority: P1)

O analista usa uma interface web para informar o SIAPE e ver os resultados, sem
linha de comando.

**Why this priority**: Torna a ferramenta acessível a usuários não técnicos.

**Independent Test**: Abrir a página, informar um SIAPE e ver total, categorias
e lista com resumos.

**Acceptance Scenarios**:

1. **Given** a tela de busca, **When** informo um SIAPE inválido, **Then** a
   busca é bloqueada com mensagem.
2. **Given** um SIAPE válido, **When** busco, **Then** vejo o total, a contagem
   por categoria e a lista de documentos com seus resumos.

---

### User Story 4 - Baixar documentos organizados (Priority: P2)

O analista baixa os PDFs com nomes padronizados e legíveis.

**Why this priority**: Facilita arquivamento e localização posterior; complementa
a coleta.

**Independent Test**: Após uma busca, baixar e conferir que cada arquivo abre e
tem nome no padrão ano_número_assunto.

**Acceptance Scenarios**:

1. **Given** documentos recuperados, **When** faço o download, **Then** cada PDF
   é um arquivo válido e nomeado como `AAAA_NUMERO_ASSUNTO`.
2. **Given** dois documentos com o mesmo nome derivado, **When** baixo, **Then**
   nenhum arquivo é sobrescrito.

---

### User Story 5 - Classificar documentos por categoria (Priority: P2)

O analista vê os documentos agrupados por categoria (ex.: Progressão, Comissão,
Férias, Outros).

**Why this priority**: Organiza grandes volumes e acelera a leitura.

**Independent Test**: Após a busca, verificar que cada documento recebeu
exatamente uma categoria e que a contagem por categoria soma o total.

**Acceptance Scenarios**:

1. **Given** um documento, **When** é classificado, **Then** recebe exatamente
   uma categoria.
2. **Given** um documento que não se encaixa em nenhuma categoria definida,
   **When** é classificado, **Then** recebe a categoria "Outros".

---

### User Story 6 - Resumir cada documento (Priority: P2)

O analista lê um resumo curto e fiel de cada documento, sem abrir o PDF.

**Why this priority**: Economiza tempo de leitura; principal ganho de
produtividade.

**Independent Test**: Conferir que cada resumo reflete o conteúdo do documento e
não contém informação inexistente na fonte.

**Acceptance Scenarios**:

1. **Given** o conteúdo de um documento, **When** é resumido, **Then** o resumo
   deriva do texto e não inventa dados.
2. **Given** um documento que não pôde ser lido, **When** o lote é processado,
   **Then** os demais documentos seguem sendo resumidos normalmente.

---

### User Story 7 - Gerar relatório e baixar arquivos (Priority: P3)

O analista gera um relatório consolidado (por categoria) em PDF e baixa todos os
documentos em um pacote.

**Why this priority**: Entrega o resultado final para compartilhamento; depende
das etapas anteriores.

**Independent Test**: Gerar o relatório e o pacote e confirmar que ambos abrem e
contêm o conteúdo esperado.

**Acceptance Scenarios**:

1. **Given** documentos classificados e resumidos, **When** gero o relatório,
   **Then** ele contém uma tabela de categorias e uma seção por categoria.
2. **Given** um resultado de busca, **When** solicito o pacote, **Then** recebo
   um arquivo compactado com todos os documentos.

---

### User Story 8 - Administrar categorias (Priority: P3)

O administrador cadastra, edita e remove categorias (nome + descrição) que guiam
a classificação.

**Why this priority**: Permite adaptar a classificação sem depender de
desenvolvimento; melhoria contínua.

**Independent Test**: Adicionar uma categoria, executar uma busca e ver a nova
categoria sendo usada.

**Acceptance Scenarios**:

1. **Given** a tela de categorias, **When** salvo uma categoria com nome, **Then**
   ela passa a existir e a ser usada na classificação.
2. **Given** um nome de categoria já existente, **When** tento salvar, **Then** é
   rejeitado.
3. **Given** uma categoria salva, **When** a removo, **Then** ela deixa de
   existir.

---

### Edge Cases

- SIAPE sem nenhum documento no portal → resultado vazio, sem erro.
- SIAPE citado apenas como número de processo em todos os documentos → todos
  aparecem como "descartados".
- Documento com PDF ilegível (sem texto extraível) → resumo usa o trecho da
  busca como alternativa.
- Serviço de classificação/resumo indisponível ou com limite excedido → o
  sistema tenta novamente; persistindo a falha, informa sem perder o já feito.
- Nome de arquivo com caracteres inválidos → é saneado.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema MUST recuperar todos os documentos do portal para o SIAPE
  informado, percorrendo toda a paginação, sem duplicatas.
- **FR-002**: O sistema MUST aceitar apenas SIAPE com 5 a 8 dígitos.
- **FR-003**: O sistema MUST incluir no resultado somente documentos cujo texto
  contém o SIAPE buscado, separando os demais como "descartados".
- **FR-004**: O sistema MUST baixar cada documento válido como arquivo nomeado no
  padrão `AAAA_NUMERO_ASSUNTO`, sem sobrescrever nomes iguais.
- **FR-005**: O sistema MUST atribuir a cada documento exatamente uma categoria;
  quando nenhuma se aplica, MUST usar "Outros".
- **FR-006**: O conjunto de categorias MUST ser definido por configuração (nome +
  descrição), editável pelo usuário sem alterar o comportamento por código.
- **FR-007**: O sistema MUST gerar um resumo por documento que reflita fielmente
  o conteúdo, sem incluir informação inexistente na fonte.
- **FR-008**: O sistema MUST gerar um relatório consolidado agrupado por
  categoria e permitir baixá-lo, além de um pacote com todos os documentos.
- **FR-009**: O sistema MUST oferecer uma interface web para busca, visualização,
  download e administração (CRUD) de categorias.
- **FR-010**: O sistema MUST reaproveitar resultados já obtidos ao repetir uma
  operação, evitando reprocessamento desnecessário do mesmo documento.
- **FR-011**: O sistema MUST tolerar falhas: um documento com erro não interrompe
  o processamento dos demais; erros temporários são reprocessados.
- **FR-012**: O sistema MUST NUNCA expor de forma não intencional dados pessoais
  de terceiros nem credenciais.

### Key Entities *(include if feature involves data)*

- **Servidor**: pessoa identificada pela matrícula SIAPE; pode ter nome.
- **Documento**: ato administrativo (portaria, despacho); título, número, ano,
  data, trecho, SIAPEs citados, categoria, resumo, arquivo.
- **Categoria**: rótulo com nome e descrição (a descrição orienta a
  classificação).
- **ResultadoBusca**: conjunto retornado para um SIAPE (documentos e descartados,
  totais).
- **Resumo**: relatório consolidado por categoria (documento agregado).
- **Repositório**: coleção-fonte do portal (Boletim, GeDoc, Site).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% dos documentos que o portal retorna para um SIAPE são
  coletados (nenhuma página perdida).
- **SC-002**: 0 falsos-positivos — todo documento listado contém o SIAPE no
  texto.
- **SC-003**: Repetir a consulta de um SIAPE já processado não gera novo
  reprocessamento dos documentos já tratados.
- **SC-004**: 100% dos arquivos baixados são documentos válidos e abrem
  corretamente.
- **SC-005**: Nenhum dado pessoal de terceiros é exposto de forma não
  intencional.
- **SC-006**: O analista obtém a lista com resumos de um SIAPE já consultado em
  poucos segundos.

## Assumptions

- O acesso ao portal GeDoc é público e somente leitura.
- As categorias iniciais são Progressão, Comissão, Férias e Outros, ajustáveis.
- A classificação e o resumo automáticos podem usar um serviço externo de IA; na
  ausência dele, a classificação recai em regras simples.
- Não há autenticação de usuários nem controle de acesso nesta fase.
- A retenção dos documentos baixados é local e sob responsabilidade do usuário
  (conformidade LGPD).
