# Contrato da API HTTP — Versão Web

Espelha os comandos IPC do desktop (ver `contracts/ipc-commands.md` da
001). Todos sob `/api`. Erros seguem `AppError` `{tipo, mensagem}`.

## Sessão (transversal)

- Toda resposta pode emitir/renovar o cookie `gedocs_sid` (`HttpOnly`,
  `SameSite=Lax`, `Secure` em produção).
- Requests do frontend usam `credentials: 'include'`.
- O servidor resolve o diretório da sessão a partir do cookie; arquivos
  são sempre isolados por sessão.

## Mapeamento de erro → status

| `tipo` | HTTP |
|---|---|
| `SiapeInvalido`, `CategoriaSemNome`, `NomeDuplicado` | 400 |
| `FalhaPortal`, `FalhaIA` | 502 |
| `NaoImplementado` | 501 |
| `FalhaArquivo` | 500 (ou 404 quando "arquivo não encontrado") |
| (rate limit) | 429 |
| (corpo grande) | 413 |

Corpo de erro: `{ "tipo": "...", "mensagem": ... }`.

## Endpoints

### `GET /api/health`
Liveness. `200 { "ok": true }`. Sem sessão obrigatória.

### `POST /api/buscar`
Busca por SIAPE (US1, FR-001/002/004).
- Req: `{ "siape": string, "repositorio"?: "0"|"1"|"2", "modo"?: "keyword"|"llm" }`
- Res `200`: `ResultadoView`.
- Erros: 400 `SiapeInvalido`; 502 `FalhaPortal`. IA degrada (nunca 5xx por IA).

### `POST /api/documento/baixar`
Baixa o PDF para a sessão (US3, FR-003).
- Req: `{ "siape": string, "link": string, "titulo": string, "data"?: string }`
- Res `200`: `{ "arquivo": string }` (nome, nunca caminho).
- Erros: 400 `SiapeInvalido`; 500/502 `FalhaArquivo`/`FalhaPortal`.

### `GET /api/documento/:siape/:arquivo`
Abre um PDF já baixado na sessão (US3). Substitui `abrir_documento`.
- Res `200`: bytes do PDF, `Content-Type: application/pdf`
  (`Content-Disposition: inline`). Browser abre em nova aba.
- Erros: 404 `FalhaArquivo` (não encontrado / de outra sessão).

### `GET /api/categorias`
Lista categorias globais (US7, FR-008).
- Res `200`: `Categoria[]` (lista vazia é válida).

### `PUT /api/categorias`
Substitui a lista de categorias (US7, FR-008/017).
- Req: `Categoria[]`
- Res `200`: `{ "ok": true, "total": number }`.
- Erros: 400 `CategoriaSemNome` / `NomeDuplicado`.

### `POST /api/relatorio`
Gera o relatório da busca atual (US5, FR-006).
- Req: `ResultadoView`
- Res `200`: `{ "arquivo": string }` (nome do HTML). Frontend abre
  `GET /api/relatorio/:siape` em nova aba.
- Erros: 400 `SiapeInvalido`; 500 `FalhaArquivo`.

### `GET /api/relatorio/:siape`
Serve o HTML do relatório da sessão (US5).
- Res `200`: `text/html`.
- Erros: 404 `FalhaArquivo`.

### `GET /api/zip/:siape`
Empacota e baixa os PDFs da sessão (US6, FR-007).
- Res `200`: bytes do ZIP, `application/zip`,
  `Content-Disposition: attachment; filename="<siape>_documentos.zip"`.
- Erros: 400/500 `FalhaArquivo` (nenhum PDF baixado / sessão expirada).

## Notas de teste (Princípio VII)

- Handlers testados com dublês de porta (sem rede real): `GedocRepository`
  falso, `ChatIa` falso, `HttpPort` falso — reusando os padrões de teste
  já existentes no núcleo.
- Testes de sessão: dois cookies distintos não acessam arquivos um do
  outro (SC-002); dir expira e é removido após TTL (SC-003).
