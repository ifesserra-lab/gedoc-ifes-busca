# Feature Specification: Versão Web (uso interno, sem login, sessão efêmera + TTL)

**Feature Branch**: `003-versao-web`

**Created**: 2026-07-08

**Status**: Draft

**Input**: User description: "Versão web do GeDoc IFES Toolkit para uso interno. NÃO haverá login/autenticação nesta versão (acesso restrito por rede/URL). Como não há login, cada visitante recebe uma SESSÃO EFÊMERA e todo o armazenamento de PDFs baixados (PII de terceiros) é isolado por sessão e apagado automaticamente por TTL. O usuário realiza as mesmas ações do app desktop no navegador (buscar por SIAPE, classificar/resumir por IA, abrir/baixar PDFs, gerar relatório, baixar ZIP). Categorias são configuração global. Fora de escopo v1: login, banco/persistência de longo prazo, categorias por usuário, ZIP entre sessões, mobile."

Documentos de apoio: [brief-web.md](../../docs/brief-web.md), [plano-web.md](../../docs/plano-web.md).

## Clarifications

### Session 2026-07-08

- Q: Valor do TTL da sessão efêmera? → A: 1 hora de inatividade.
- Q: Como tratar duas gravações simultâneas de categorias (config global, sem login)? → A: Última gravação vence (sem trava) na v1.
- Q: Proteção contra abuso da API sem login? → A: Rate limit por IP + limite de tamanho de requisição.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Buscar documentos por SIAPE no navegador (Priority: P1)

Como servidor do IFES, quero abrir o app pelo navegador e buscar
documentos de um SIAPE, para consultar os documentos oficiais sem
instalar o aplicativo desktop.

**Why this priority**: é o valor central do produto; sem a busca, nada
mais tem utilidade. Entrega, sozinha, um MVP navegável.

**Independent Test**: acessar a URL, informar um SIAPE válido e verificar
que os documentos aparecem agrupados por categoria com o total.

**Acceptance Scenarios**:

1. **Given** o app aberto no navegador, **When** informo um SIAPE válido
   e busco, **Then** vejo os documentos agrupados por categoria e o total.
2. **Given** um SIAPE inválido, **When** busco, **Then** recebo uma
   mensagem de erro amigável e nenhuma coleta é feita.
3. **Given** o portal oficial indisponível, **When** busco, **Then**
   recebo erro amigável e o app permanece utilizável.

---

### User Story 2 - Acesso sem login com sessão efêmera e privacidade (Priority: P1)

Como responsável pelo sistema, quero que o app seja usável sem
autenticação, mas que os arquivos com dados pessoais (PII de terceiros)
fiquem isolados por sessão e sejam apagados automaticamente, para cumprir
a LGPD mesmo sem login.

**Why this priority**: privacidade/LGPD é requisito legal
NÃO-NEGOCIÁVEL (Constituição, Princípio II). Sem essa garantia, o app não
pode ir ao ar.

**Independent Test**: em duas sessões distintas, baixar arquivos e
verificar que uma sessão não acessa os arquivos da outra; deixar uma
sessão inativa além do TTL e verificar que seus arquivos foram removidos.

**Acceptance Scenarios**:

1. **Given** o app aberto, **When** acesso pela primeira vez, **Then**
   recebo automaticamente uma sessão, sem qualquer tela de login.
2. **Given** duas sessões diferentes que baixaram arquivos, **When** uma
   tenta acessar arquivos da outra, **Then** o acesso é negado.
3. **Given** uma sessão inativa por mais tempo que o TTL configurado,
   **When** a limpeza automática ocorre, **Then** todos os arquivos
   daquela sessão são removidos.
4. **Given** uma sessão qualquer, **When** ela termina/expira, **Then**
   nenhum PDF daquela sessão sobrevive.

---

### User Story 3 - Abrir e baixar o PDF de um documento (Priority: P1)

Como usuário, quero abrir e baixar o PDF de um resultado, para ler o
documento oficial no navegador.

**Why this priority**: consultar o documento em si é parte essencial do
fluxo de busca; sem isso a busca fica incompleta.

**Independent Test**: após uma busca, baixar um documento e confirmar que
ele pode ser aberto no navegador; reabrir sem baixar de novo na mesma
sessão.

**Acceptance Scenarios**:

1. **Given** um resultado de busca com link, **When** peço para baixar,
   **Then** o PDF fica disponível para abrir no navegador.
2. **Given** um PDF já baixado na sessão atual, **When** peço para abrir,
   **Then** ele abre sem baixar novamente.

---

### User Story 4 - Classificação e resumo por IA (opcional) (Priority: P2)

Como usuário, quero opcionalmente classificar e resumir os documentos por
IA, para entender rapidamente o conteúdo de cada um.

**Why this priority**: agrega valor, mas o app é plenamente útil no modo
por palavra-chave (grátis e instantâneo); a IA é um incremento.

**Independent Test**: com a IA habilitada, buscar e verificar
classificação/resumo; desabilitando a IA (ou sem ela disponível),
verificar que a busca continua funcionando por palavra-chave.

**Acceptance Scenarios**:

1. **Given** o modo IA habilitado e disponível, **When** busco, **Then**
   os documentos vêm classificados e resumidos.
2. **Given** a IA indisponível, **When** busco no modo IA, **Then** a
   classificação cai para palavra-chave e a busca nunca falha por causa
   da IA.

---

### User Story 5 - Gerar relatório consolidado (Priority: P2)

Como usuário, quero gerar um relatório consolidado da busca atual, para
arquivar ou imprimir.

**Why this priority**: útil para registro, mas secundário à consulta.

**Independent Test**: com uma busca na tela, gerar o relatório e
confirmar que um documento consolidado e legível é produzido.

**Acceptance Scenarios**:

1. **Given** uma busca exibida, **When** gero o relatório, **Then** um
   relatório consolidado abre no navegador (imprimível).
2. **Given** os resumos por IA presentes na busca, **When** gero o
   relatório, **Then** ele reflete os mesmos resumos exibidos.

---

### User Story 6 - Baixar ZIP dos PDFs da sessão (Priority: P2)

Como usuário, quero baixar um ZIP com os PDFs que baixei na sessão, para
levar todos de uma vez.

**Why this priority**: conveniência sobre documentos já baixados;
secundário.

**Independent Test**: baixar alguns PDFs na sessão, pedir o ZIP e
confirmar o download de um pacote com eles.

**Acceptance Scenarios**:

1. **Given** PDFs baixados na sessão atual, **When** peço o ZIP, **Then**
   baixo um pacote com esses PDFs.
2. **Given** nenhum PDF baixado na sessão (ou sessão expirada), **When**
   peço o ZIP, **Then** recebo erro amigável orientando a baixar primeiro.

---

### User Story 7 - Gerenciar categorias (Priority: P3)

Como usuário, quero criar, editar e remover categorias, para ajustar como
os documentos são classificados.

**Why this priority**: personalização; o app funciona com as categorias
padrão sem esta tela.

**Independent Test**: salvar uma lista de categorias e verificar que a
próxima busca as utiliza; rejeitar nome vazio ou duplicado.

**Acceptance Scenarios**:

1. **Given** a tela de categorias, **When** salvo a lista, **Then** ela
   passa a valer na próxima busca.
2. **Given** um nome vazio ou duplicado (ignorando maiúsc./minúsc.),
   **When** tento salvar, **Then** a operação é rejeitada com mensagem
   clara e nada é gravado.
3. **Given** que não há login, **When** qualquer usuário edita as
   categorias, **Then** a alteração vale para todos (configuração global).

---

### Edge Cases

- **Sessão expira no meio do fluxo**: ao pedir o ZIP após a expiração, o
  armazenamento está vazio → erro amigável orientando a rebaixar.
- **Dois usuários editam categorias ao mesmo tempo**: sendo configuração
  global sem login, a última gravação vence (sem trava). Comportamento
  aceito na v1 e documentado.
- **Busca muito grande/lenta**: o app deve indicar progresso e não travar
  a interface; timeouts produzem erro amigável.
- **Documento sem PDF disponível**: baixar retorna erro amigável, sem
  quebrar os demais resultados.
- **Acesso direto a um arquivo por URL adivinhada de outra sessão**: deve
  ser negado (isolamento por sessão).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: O sistema MUST permitir buscar documentos do portal oficial
  por SIAPE, filtrar por SIAPE e agrupar por categoria, exibindo o total.
- **FR-002**: O sistema MUST validar o SIAPE e, quando inválido ou quando
  a fonte estiver indisponível, retornar mensagem de erro amigável sem
  interromper o uso do app.
- **FR-003**: O sistema MUST permitir baixar e abrir, no navegador, o PDF
  de um documento retornado pela busca.
- **FR-004**: O sistema MUST oferecer, opcionalmente, classificação e
  resumo por IA, degradando com segurança para classificação por
  palavra-chave quando a IA estiver indisponível — a busca NUNCA falha por
  causa da IA.
- **FR-005**: O sistema MUST NOT expor a chave/credencial de IA ao
  navegador; ela permanece apenas no servidor.
- **FR-006**: O sistema MUST gerar um relatório consolidado da busca
  atual, refletindo os mesmos dados/resumos exibidos, abrível/imprimível
  no navegador.
- **FR-007**: O sistema MUST empacotar em ZIP os PDFs baixados na sessão
  atual e, quando não houver PDFs, retornar erro amigável.
- **FR-008**: O sistema MUST oferecer CRUD de categorias com nome
  obrigatório e único (sem diferenciar maiúsc./minúsc.); a configuração é
  GLOBAL (sem isolamento por usuário).
- **FR-009**: O sistema MUST NÃO exigir login/autenticação para uso na v1.
- **FR-010**: O sistema MUST atribuir automaticamente uma SESSÃO EFÊMERA a
  cada visitante, sem qualquer etapa de cadastro ou login.
- **FR-011**: O sistema MUST isolar por sessão todo o armazenamento de
  PDFs baixados (PII de terceiros); uma sessão NUNCA acessa arquivos de
  outra.
- **FR-012**: O sistema MUST apagar automaticamente os arquivos de uma
  sessão por TTL de **1 hora de inatividade** (configurável pelo
  operador) — nada persiste entre sessões.
- **FR-013**: O sistema MUST NÃO registrar PII em logs.
- **FR-016**: O sistema MUST proteger a API contra abuso, aplicando
  limite de requisições por origem (rate limit) e limite de tamanho de
  requisição; ao exceder, retorna erro amigável sem processar.
- **FR-017**: Ao salvar categorias concorrentemente, o sistema MAY
  aplicar "última gravação vence" (sem trava de concorrência na v1); a
  gravação mais recente sobrescreve a anterior.
- **FR-014**: O sistema MUST preservar o mesmo contrato de erros do app
  desktop (tipo do erro + mensagem amigável), para consistência de
  comportamento entre desktop e web.
- **FR-015**: O mesmo frontend MUST atender tanto o app desktop quanto a
  web, sem duplicar a lógica de negócio.

### Key Entities *(include if feature involves data)*

- **Sessão**: contexto efêmero atribuído a um visitante sem login;
  possui um identificador, um instante de última atividade e um TTL;
  delimita o isolamento e a expiração dos arquivos baixados.
- **Documento**: item retornado pela busca (título, data, link, categoria,
  e — quando baixado/resumido — arquivo e resumo).
- **Categoria**: rótulo de classificação global (nome obrigatório e único,
  descrição opcional).
- **Relatório**: consolidação legível/imprimível da busca atual.
- **Pacote (ZIP)**: agrupamento dos PDFs baixados na sessão atual.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Um usuário consegue, a partir da URL, obter os documentos de
  um SIAPE válido em até 3 passos (abrir, digitar, buscar), sem instalar
  nada e sem login.
- **SC-002**: 100% dos PDFs de uma sessão ficam inacessíveis a outras
  sessões (isolamento verificável).
- **SC-003**: 100% dos arquivos de uma sessão são removidos após o TTL de
  inatividade (limpeza verificável).
- **SC-004**: A chave de IA nunca aparece no conteúdo entregue ao
  navegador (0 ocorrências).
- **SC-005**: Falhas de IA/portal não impedem a conclusão da busca em
  100% dos casos (degradação/mensagem amigável).
- **SC-006**: As mesmas ações do app desktop (buscar, baixar, abrir,
  classificar/resumir, relatório, ZIP, categorias) estão disponíveis na
  web.
- **SC-007**: Requisições acima do limite por origem ou do tamanho máximo
  são rejeitadas em 100% dos casos, sem processar a requisição.

## Assumptions

- Acesso é restrito por rede/URL na v1 (uso interno); autenticação é fora
  de escopo e será adicionada depois, se o app abrir ao público.
- Sem login, a identidade não é conhecida; por isso categorias são
  configuração global e não há histórico por usuário.
- O armazenamento de arquivos é efêmero (sem banco nem persistência de
  longo prazo); o ZIP só cobre o que foi baixado na mesma sessão.
- TTL padrão de sessão = 1 hora de inatividade, configurável pelo
  operador.
- Suporte mobile nativo está fora de escopo na v1.
- A fonte de dados é o portal oficial GeDoc; o conteúdo não é alterado
  (fidelidade à fonte, Princípio I).
- A capacidade de IA depende de credencial configurada no servidor; sua
  ausência é um estado válido (modo palavra-chave).

## Out of Scope (v1)

- Autenticação/login e categorias por usuário.
- Banco de dados ou persistência de longo prazo; ZIP entre sessões.
- Aplicativo mobile nativo.
- Multi-tenant / SaaS público.
