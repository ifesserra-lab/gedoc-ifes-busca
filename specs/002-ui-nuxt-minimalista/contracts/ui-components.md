# Contrato de UI — telas, componentes e estados

Fase 1. Inventário de apresentação (não há novo contrato de dados; consome o IPC
de `specs/001`). Cada tela lista componentes (Nuxt UI) e os 5 estados.

## Design system (tokens) — `app/src/assets/tokens.css`
- Cores: `--surface`, `--surface-2`, `--text`, `--muted`, `--border`,
  `--accent` (1 acento), `--danger`, `--success`. Variantes light/dark.
- Tipografia: 1 família (Inter self-hosted); escala (12/14/16/20/28).
- Espaçamento: múltiplos de 4 (`--sp-1..8`). Raio: `--radius`. Sombra discreta.
- Acento do Nuxt UI configurado em `app/src/ui.config.ts` mapeando para `--accent`.

## Tela: Busca (`views/BuscaView.vue`)
- **Componentes**: `UInput` (SIAPE, validação R10 inline), `UButton` (Buscar,
  desabilita em loading), `UBadge` (chips por categoria + contagem),
  `UCard`/`UTable` (documentos), `DocItem` (título, data, resumo, `UButton` PDF),
  ações globais (`UButton` PDF do resumo, ZIP).
- **Estados**:
  - idle: campo + dica.
  - loading: `USkeleton` na lista + botão desabilitado.
  - vazio: `EmptyState` ("nenhum documento para este SIAPE").
  - erro: `ErrorState`/`UAlert` (mensagem útil + "tentar novamente").
  - sucesso: chips + lista.

## Tela: Categorias (`views/CategoriasView.vue`)
- **Componentes**: `UTable` (nome, descrição, ações), `UButton` (adicionar),
  `UModal` + `UForm` + `UInput`/`UTextarea` (criar/editar), confirmação de
  remoção, `UBadge` de status.
- **Estados**: idle (tabela), vazio (sem categorias), erro (falha ao salvar),
  sucesso (toast de salvo). Validação: nome obrigatório + único.

## Componentes base (finos) — `components/base/`
- `LoadingState`, `EmptyState`, `ErrorState` (sobre `USkeleton`/`UAlert`),
  reutilizados por todas as telas (consistência — SC-001).
- Regra: sem lógica de negócio; recebem props e emitem eventos.

## Acessibilidade (todas as telas)
- Foco visível; navegação por teclado; `aria-label` em ícones/ações; contraste
  AA (validar tokens); alvos ≥ 40px.

## Não-metas
- Sem novo comando IPC. Se a UI precisar de dado novo, especificar aqui e pedir
  ao `tauri-mvc-expert`.
