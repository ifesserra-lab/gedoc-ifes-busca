# Implementation Plan: IA classifica pelas categorias do category.json (no prompt)

**Branch**: `006-ia-prompt-categorias` | **Date**: 2026-07-08 | **Spec**: [spec.md](./spec.md)

## Summary

**A funcionalidade já existe no núcleo.** A classificação por IA
(`gedocs_core::ports::classificador::ClassificadorLlm`) já:

- injeta as categorias do `category.json` (nome + descrição) no prompt —
  `montar_prompt` ([classificador.rs:118](../../core/src/ports/classificador.rs)):
  `"Categorias disponíveis:\n- <nome>: <descrição>\n..."` + sistema pede
  "exatamente UMA categoria da lista fornecida";
- restringe a resposta ao conjunto — `extrair_categoria` mapeia qualquer
  nome fora da lista (ou JSON inválido) para `OUTROS` (R4);
- degrada por documento para palavra-chave se a IA falhar (R11);
- lê as categorias sempre de `category.json` (via
  `categorias::resolver_com_semente`), tanto no desktop quanto na web.

Logo, FR-001..006 já são atendidos. **Lacuna**: não há teste explícito
travando que o **prompt inclui os nomes+descrições** (FR-002/SC-002). Esta
feature adiciona esse teste-guarda (evita regressão futura). Sem mudança de
comportamento em runtime.

## Technical Context

**Language**: Rust (núcleo `gedocs-core`). **Testing**: `cargo test`.
**Project Type**: núcleo compartilhado (desktop + web). Sem novas deps.

## Constitution Check

PASS. I (fidelidade): IA escolhe entre categorias reais, fora→Outros, não
inventa. IV (config): categorias vêm do `category.json`. VII (TDD): adiciona
teste-guarda do prompt. VIII: mudança mínima (só teste). Sem violações.

## Project Structure (afetado)

```text
core/src/ports/classificador.rs   # ADICIONAR teste: prompt inclui nome+descrição
```

**Structure Decision**: nenhuma mudança de runtime — apenas um teste em
`classificador.rs` (mod tests) chamando `montar_prompt` e verificando que o
prompt do usuário contém cada `nome` + `descrição`. Se numa revisão futura o
comportamento for constatado insuficiente, o teste falha e orienta o ajuste.
