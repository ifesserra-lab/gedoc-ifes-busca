//! Carrega `config/categoria.json` (R5 — categorias vêm de configuração, não
//! de código: Princípio IV). Formato:
//! `{ "categorias": [ { "nome": ..., "descricao": ... }, ... ] }`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::domain::categoria::Categoria;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
struct CategoriasArquivo {
    categorias: Vec<Categoria>,
}

/// Lê e valida `caminho`: erro claro (nunca panic) se o arquivo estiver
/// ausente, malformado ou sem nenhuma categoria — uma lista vazia por engano
/// faria todo documento cair em "Outros" silenciosamente.
pub fn carregar_categorias(caminho: &Path) -> Result<Vec<Categoria>, AppError> {
    let conteudo = fs::read_to_string(caminho).map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Não foi possível ler '{}': {e}", caminho.display()),
    })?;
    let arquivo: CategoriasArquivo =
        serde_json::from_str(&conteudo).map_err(|e| AppError::FalhaArquivo {
            motivo: format!(
                "'{}' não é um JSON válido de categorias: {e}",
                caminho.display()
            ),
        })?;
    if arquivo.categorias.is_empty() {
        return Err(AppError::FalhaArquivo {
            motivo: format!("'{}' não contém categorias.", caminho.display()),
        });
    }
    Ok(arquivo.categorias)
}

/// Caminho padrão de `config/categoria.json`, tentando alguns candidatos
/// relativos: o app pode rodar com cwd na raiz do repositório ou em
/// `src-tauri/` (mesma ambiguidade resolvida por `ports::ia::resolver_api_key`
/// para o `.env`). Não toca a rede; só verifica quais candidatos existem —
/// glue de runtime, não testado unitariamente (como `dir_documentos` em
/// `commands::documento`), pois depende do cwd real do processo.
pub fn caminho_padrao() -> PathBuf {
    for candidato in ["config/categoria.json", "../config/categoria.json"] {
        let caminho = PathBuf::from(candidato);
        if caminho.is_file() {
            return caminho;
        }
    }
    PathBuf::from("config/categoria.json")
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    fn escrever(dir: &Path, nome: &str, conteudo: &str) -> PathBuf {
        let caminho = dir.join(nome);
        fs::write(&caminho, conteudo).expect("grava fixture");
        caminho
    }

    #[test]
    fn carrega_categorias_do_json_no_formato_esperado() {
        let dir = tempdir().expect("tempdir");
        let caminho = escrever(
            dir.path(),
            "categoria.json",
            r#"{
                "categorias": [
                    {"nome": "Progressão", "descricao": "Progressão funcional."},
                    {"nome": "Outros", "descricao": null}
                ]
            }"#,
        );

        let categorias = carregar_categorias(&caminho).expect("deve carregar");

        assert_eq!(categorias.len(), 2);
        assert_eq!(categorias[0].nome, "Progressão");
        assert_eq!(
            categorias[0].descricao.as_deref(),
            Some("Progressão funcional.")
        );
        assert_eq!(categorias[1].nome, "Outros");
        assert_eq!(categorias[1].descricao, None);
    }

    #[test]
    fn erro_claro_quando_arquivo_nao_existe() {
        let dir = tempdir().expect("tempdir");
        let erro = carregar_categorias(&dir.path().join("nao_existe.json")).unwrap_err();
        assert!(matches!(erro, AppError::FalhaArquivo { .. }));
    }

    #[test]
    fn erro_claro_quando_json_malformado() {
        let dir = tempdir().expect("tempdir");
        let caminho = escrever(dir.path(), "categoria.json", "isto não é json");
        let erro = carregar_categorias(&caminho).unwrap_err();
        assert!(matches!(erro, AppError::FalhaArquivo { .. }));
    }

    #[test]
    fn erro_claro_quando_lista_de_categorias_vazia() {
        let dir = tempdir().expect("tempdir");
        let caminho = escrever(dir.path(), "categoria.json", r#"{"categorias": []}"#);
        let erro = carregar_categorias(&caminho).unwrap_err();
        assert!(matches!(erro, AppError::FalhaArquivo { .. }));
    }
}
