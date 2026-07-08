# Brief de Feature — Versão Web do GeDoc IFES Toolkit

> **Propósito deste documento**: servir de entrada para o spec-kit.
> É o **quê/por quê** (comportamento e requisitos), não o **como**
> (arquitetura/implementação — essa está em [plano-web.md](plano-web.md)).
> Ao final há um parágrafo pronto para colar em `/speckit-specify`.
>
> Numeração sugerida da feature: `003-versao-web`.

---

## 1. Contexto e objetivo

O GeDoc IFES Toolkit hoje é um app **desktop** (Tauri + Vue). Este brief
descreve levar as mesmas funcionalidades para a **web**: frontend
acessível pelo navegador, hospedado na Vercel, conversando com uma API
que reusa o núcleo Rust existente.

Meta da v1: qualquer usuário interno abre uma URL, busca documentos do
portal GeDoc por SIAPE, consulta/baixa PDFs, gera relatório e ZIP — sem
instalar nada.

## 2. Público e escopo (decisões travadas)

- **Uso interno, sem login (v1).** Restrito por rede/URL. Sem
  autenticação nesta versão.
- **Sessão efêmera + TTL.** Cada visitante tem uma sessão (cookie); os
  PDFs baixados vivem só durante a sessão e são apagados por TTL
  (default 1h). Nada persiste entre sessões.
- **Categorias globais.** Configuração única compartilhada no servidor.
- **Chave de IA só no servidor.** Nunca exposta ao navegador.

## 3. User Stories (priorizadas)

### US-W1 — Buscar documentos por SIAPE no navegador (P1)
Como servidor do IFES, quero abrir o app no navegador e buscar
documentos de um SIAPE, para consultar sem instalar o app desktop.
- **Given** a URL do app aberta, **When** informo um SIAPE válido e
  busco, **Then** vejo os documentos agrupados por categoria, com total.
- **Given** um SIAPE inválido, **When** busco, **Then** recebo mensagem
  de erro amigável e nenhuma busca é feita.
- **Given** o portal indisponível, **When** busco, **Then** recebo erro
  amigável e o app continua utilizável.

### US-W2 — Visualizar e baixar o PDF de um documento (P1)
Como usuário, quero abrir/baixar o PDF de um resultado, para ler o
documento oficial.
- **Given** um resultado com link, **When** clico em baixar, **Then** o
  PDF é obtido e fica disponível para abrir no navegador (nova aba).
- **Given** um PDF já baixado na sessão, **When** clico em abrir,
  **Then** ele abre no navegador sem baixar de novo.

### US-W3 — Classificação e resumo por IA (P2)
Como usuário, quero (opcionalmente) classificar e resumir por IA, para
entender rápido o conteúdo.
- **Given** o modo IA ligado e chave configurada no servidor, **When**
  busco, **Then** cada documento vem classificado e resumido.
- **Given** sem chave/IA indisponível, **When** busco no modo IA,
  **Then** a busca degrada para classificação por palavra-chave e nunca
  falha por causa da IA.

### US-W4 — Gerar relatório consolidado (P2)
Como usuário, quero gerar um relatório da busca atual, para arquivar/
imprimir.
- **Given** uma busca na tela, **When** gero o relatório, **Then** um
  HTML consolidado abre no navegador (imprimível como PDF).

### US-W5 — Baixar ZIP dos PDFs da sessão (P2)
Como usuário, quero baixar um ZIP com os PDFs que baixei, para levar
tudo de uma vez.
- **Given** PDFs baixados na sessão, **When** peço o ZIP, **Then** o
  navegador baixa um `.zip` com eles.
- **Given** nenhum PDF baixado (ou sessão expirada), **When** peço o
  ZIP, **Then** recebo erro amigável orientando a baixar primeiro.

### US-W6 — Gerenciar categorias (P3)
Como usuário, quero criar/editar/remover categorias, para ajustar a
classificação.
- **Given** a tela de categorias, **When** salvo a lista, **Then** ela
  vale na próxima busca; nome vazio ou duplicado é rejeitado.

### US-W7 — Sessão efêmera com limpeza automática (P2, transversal, LGPD)
Como responsável pelo sistema, quero que os PDFs (PII de terceiros)
sejam isolados por sessão e apagados por TTL, para cumprir a LGPD sem
login.
- **Given** duas sessões distintas, **When** ambas baixam PDFs, **Then**
  uma não acessa os arquivos da outra.
- **Given** uma sessão inativa além do TTL, **When** o job de limpeza
  roda, **Then** seus arquivos são removidos.

## 4. Requisitos funcionais

- **FR-W1**: buscar documentos por SIAPE via API (scrape server-side),
  filtrar por SIAPE e agrupar por categoria.
- **FR-W2**: baixar o PDF de um documento para o armazenamento da sessão
  e disponibilizá-lo para abertura no navegador.
- **FR-W3**: classificar (palavra-chave sempre; IA opcional) e resumir
  (IA opcional) documentos; IA degrada com segurança.
- **FR-W4**: gerar relatório HTML consolidado da busca atual.
- **FR-W5**: empacotar em ZIP os PDFs baixados na sessão.
- **FR-W6**: CRUD de categorias (global), com validação de nome
  obrigatório e único (case-insensitive).
- **FR-W7**: manter os mesmos formatos de dados e mensagens de erro do
  app desktop (contrato `AppError` `{tipo, mensagem}`).
- **FR-W8**: isolar dados por sessão e limpar por TTL.

## 5. Requisitos não-funcionais

- **Privacidade/LGPD (II, NON-NEGOTIABLE)**: PDFs com PII isolados por
  sessão, apagados por TTL; nada de PII em logs; chave de IA só no
  servidor.
- **Fidelidade à fonte (I)**: dados vêm do portal oficial, sem alterar
  conteúdo.
- **TDD (VII, NON-NEGOTIABLE)**: endpoints e adaptador web com testes
  antes da implementação.
- **Camadas/DRY (V) e reuso**: reaproveitar o núcleo Rust puro; não
  duplicar regra de negócio na API.
- **CORS**: API aceita apenas a origem do frontend (domínio Vercel).
- **Compatibilidade**: o mesmo frontend serve desktop (Tauri) e web,
  escolhendo o transporte em runtime.

## 6. Fora de escopo (v1)

- Autenticação/login e categorias por usuário.
- Persistência de longo prazo (banco, object storage) e ZIP entre
  sessões.
- App mobile nativo.
- Multi-tenant / SaaS público.

## 7. Alinhamento com a constituição

| Princípio | Como este brief atende |
|---|---|
| II LGPD (NN) | sessão efêmera + TTL, isolamento, IA server-only |
| VII TDD (NN) | testes de endpoint/adaptador antes do código |
| V Camadas/DRY | reuso do núcleo Rust, API fina |
| VIII Código pequeno | handlers finos delegando aos use-cases |
| X Issue-first (NN) | abrir issue da feature antes de codar |
| XII UI/UX | telas web reaproveitam o design system existente |

## 8. Perguntas em aberto (para o /speckit-clarify)

- TTL exato da sessão e política de limpeza (job vs. on-access)?
- Comportamento se dois usuários salvarem categorias ao mesmo tempo
  (config global, sem lock)?
- Limites de rate/tamanho de requisição na API?

## 9. Texto pronto para o /speckit-specify

> Copiar o parágrafo abaixo como argumento do comando.

```
Versão web do GeDoc IFES Toolkit para uso interno sem login. O usuário
abre uma URL no navegador e realiza as mesmas ações do app desktop:
buscar documentos do portal GeDoc por SIAPE (agrupados por categoria),
classificar/resumir opcionalmente por IA (chave só no servidor, com
degradação segura para palavra-chave), abrir e baixar PDFs, gerar um
relatório HTML consolidado e baixar um ZIP dos PDFs. O frontend Vue é
servido estático (Vercel) e conversa com uma API que reusa o núcleo
Rust do app. Como não há login, cada visitante tem uma sessão efêmera
(cookie) cujo armazenamento de PDFs (PII de terceiros) é isolado e
apagado por TTL (LGPD). Categorias são uma configuração global no
servidor, com nome obrigatório e único. Erros seguem o mesmo contrato
do desktop (tipo + mensagem amigável). Fora de escopo na v1:
autenticação, banco/persistência de longo prazo, categorias por
usuário e mobile.
```

## 10. Próximos comandos spec-kit

1. `/speckit-specify` — colar o texto do §9 → gera `specs/003-versao-web/spec.md`.
2. `/speckit-clarify` — resolver o §8.
3. `/speckit-plan` — gerar `plan.md` (referenciar [plano-web.md](plano-web.md)).
4. `/speckit-tasks` — gerar `tasks.md`.
5. `/speckit-implement` — executar.
