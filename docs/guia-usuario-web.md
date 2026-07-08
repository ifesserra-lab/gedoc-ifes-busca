# Guia do Usuário — GeDoc IFES Toolkit (Web)

Versão web para consultar documentos públicos do portal GeDoc do IFES por
matrícula **SIAPE**, direto no navegador — sem instalar nada.

- **Acesso:** <https://gedocs.vercel.app>
- **Uso interno**, sem login (acesso restrito por rede/URL).

> Prefere o app desktop? Veja os instaladores no [README](../README.md). O
> desktop tem os mesmos recursos de busca **mais** o gerenciamento de
> categorias (que na web é somente leitura — ver [Categorias](#categorias)).

---

## Visão geral

| O que você faz | Como |
|---|---|
| Buscar documentos de um SIAPE | Digite o SIAPE e clique em **Buscar** |
| Resumir/classificar por IA | Ligue o **modo IA** antes de buscar (opcional) |
| Abrir/baixar um PDF | Botões no documento listado |
| Gerar relatório consolidado | **Baixar relatório** (executa a IA e gera) |
| Baixar todos os PDFs em um ZIP | **Baixar ZIP** |

Os documentos aparecem **agrupados por categoria** e, dentro de cada
categoria, **ordenados por ano** (mais recente primeiro).

---

## Passo a passo

### 1. Buscar por SIAPE

1. Abra <https://gedocs.vercel.app>.
2. Digite a matrícula **SIAPE** (5 a 8 dígitos).
3. Clique em **Buscar**.

O sistema coleta os documentos no portal oficial, mantém apenas os que citam
o SIAPE no texto e agrupa por categoria. SIAPE inválido ou portal
indisponível mostram uma mensagem amigável — a tela continua utilizável.

### 2. Modo IA (opcional)

Ligue o **modo IA** antes de buscar para que cada documento seja
**classificado e resumido** por IA. Sem o modo IA, a busca é gratuita e
instantânea (classificação por palavra-chave, sem resumo).

> A chave de IA fica **somente no servidor** — nunca no navegador. Se a IA
> estiver indisponível, a busca **não falha**: cai para classificação por
> palavra-chave.

### 3. Abrir e baixar PDFs

- **Baixar**: obtém o PDF do documento (guardado na sua sessão).
- **Abrir**: abre o PDF em uma nova aba do navegador.
- **Baixar todos os PDFs**: baixa, um a um, os PDFs listados (pula os já
  baixados; mostra progresso).

### 4. Gerar relatório

Clique em **Baixar relatório**. O relatório consolida os **resumos da IA**,
então:

- se a busca atual **não** foi no modo IA, o sistema **executa a IA agora**
  (resume os documentos) e depois gera o relatório — isso pode levar um
  tempo, proporcional ao número de documentos;
- o relatório (HTML) abre em uma nova aba. Para PDF, use **Imprimir → Salvar
  como PDF** do navegador.

> Abrir o link do relatório direto (sem gerar pela tela) mostra a mensagem
> "Relatório não encontrado…". Isso é esperado: o relatório é **gerado por
> sessão**; gere-o pela tela primeiro.

### 5. Baixar ZIP

Clique em **Baixar ZIP** para baixar um `.zip` com os PDFs que você baixou
**nesta sessão**. Sem nenhum PDF baixado (ou sessão expirada), aparece uma
mensagem orientando a baixar primeiro.

---

## Categorias

As categorias (ex.: *Progressão, Comissão, Férias, Outros*) são definidas em
um arquivo de configuração do servidor (`category.json`) e guiam a
classificação. Na web elas são **somente leitura**:

- os documentos aparecem agrupados nas categorias definidas;
- um documento que não se encaixa em nenhuma vai para **Outros**;
- **não há** tela de criar/editar/remover categorias na web (isso existe
  apenas no app desktop).

Adicionar/editar uma categoria no `category.json` passa a valer na próxima
busca (após o deploy) — sem mudança de código.

---

## Privacidade e sessão (LGPD)

Como não há login, cada visitante recebe uma **sessão efêmera** (um
identificador em cookie). Os PDFs baixados (que contêm dados pessoais de
terceiros) ficam **isolados por sessão** e são **apagados automaticamente**
após um período de inatividade (TTL, ~1 hora). Uma sessão nunca acessa os
arquivos de outra, e nada persiste entre sessões.

---

## Perguntas frequentes

**A primeira busca demorou muito.** No plano gratuito de hospedagem, o
servidor hiberna após ~15 min ociosos; a primeira requisição depois disso
tem um "cold start" de ~30–60 s. As seguintes são rápidas.

**Gerei o relatório mas ao reabrir o link deu "não encontrado".** O
relatório é por sessão e some após o TTL; gere-o de novo pela tela.

**O modo IA está lento.** Ele resume documento a documento; para SIAPEs com
muitos documentos, leva mais tempo. A busca sem IA é instantânea.

**Preciso gerenciar categorias.** Use o app desktop (a web é somente
leitura). Ou edite o `category.json` do servidor (requer deploy).
