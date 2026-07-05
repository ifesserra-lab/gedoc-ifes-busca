//! `CacheArquivo` — cache genérico por link, persistido em um arquivo JSON
//! `{ link: valor }` (R6: idempotência — não reclassifica/resume um
//! documento já visto). **Sem PII**: a chave é `doc.link` (já uma URL opaca,
//! `/documento/<hash32>?inline`) e o valor é um rótulo curto (ex.: nome da
//! categoria); título, trecho e SIAPE nunca são gravados aqui. Vive em
//! `app_data_dir`, fora do VCS (Princípio II/LGPD).
//!
//! Genérico o suficiente para ser reutilizado tal como está por US6 (resumo)
//! — cada uso aponta para um arquivo diferente (`cache/classificacao.json`,
//! `cache/resumo.json`), sem precisar de um novo tipo.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::error::AppError;

#[derive(Debug, Default)]
pub struct CacheArquivo {
    caminho: PathBuf,
    dados: HashMap<String, String>,
}

impl CacheArquivo {
    /// Carrega o cache de `caminho`. Se o arquivo não existir ou estiver
    /// corrompido (JSON inválido), começa vazio: um cache ilegível nunca deve
    /// impedir a classificação/resumo (degradação segura).
    pub fn carregar(caminho: impl Into<PathBuf>) -> Self {
        let caminho = caminho.into();
        let dados = fs::read_to_string(&caminho)
            .ok()
            .and_then(|conteudo| serde_json::from_str(&conteudo).ok())
            .unwrap_or_default();
        Self { caminho, dados }
    }

    /// Valor cacheado para `link`, se algum documento com esse link já foi
    /// processado antes (R6).
    pub fn obter(&self, link: &str) -> Option<&str> {
        self.dados.get(link).map(String::as_str)
    }

    /// Registra/sobrescreve o valor de `link` em memória; `salvar` persiste.
    pub fn inserir(&mut self, link: impl Into<String>, valor: impl Into<String>) {
        self.dados.insert(link.into(), valor.into());
    }

    /// Grava o cache em disco, criando o diretório pai se necessário.
    pub fn salvar(&self) -> Result<(), AppError> {
        if let Some(pai) = self.caminho.parent() {
            fs::create_dir_all(pai).map_err(|e| AppError::FalhaArquivo {
                motivo: format!("Falha ao criar diretório do cache: {e}"),
            })?;
        }
        let json =
            serde_json::to_string_pretty(&self.dados).map_err(|e| AppError::FalhaArquivo {
                motivo: format!("Falha ao serializar o cache: {e}"),
            })?;
        fs::write(&self.caminho, json).map_err(|e| AppError::FalhaArquivo {
            motivo: format!("Falha ao gravar o cache em disco: {e}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn obter_retorna_none_quando_o_arquivo_nao_existe() {
        let dir = tempdir().expect("tempdir");
        let cache = CacheArquivo::carregar(dir.path().join("cache.json"));
        assert_eq!(
            cache.obter("https://gedoc.ifes.edu.br/documento/aaaa?inline"),
            None
        );
    }

    #[test]
    fn inserir_e_obter_no_mesmo_processo() {
        let dir = tempdir().expect("tempdir");
        let mut cache = CacheArquivo::carregar(dir.path().join("cache.json"));

        cache.inserir("link1", "Progressão");

        assert_eq!(cache.obter("link1"), Some("Progressão"));
        assert_eq!(
            cache.obter("link2"),
            None,
            "link nunca inserido continua miss"
        );
    }

    #[test]
    fn inserir_sobrescreve_valor_existente_idempotente() {
        let dir = tempdir().expect("tempdir");
        let mut cache = CacheArquivo::carregar(dir.path().join("cache.json"));

        cache.inserir("link1", "Outros");
        cache.inserir("link1", "Progressão");

        assert_eq!(cache.obter("link1"), Some("Progressão"));
    }

    #[test]
    fn salvar_e_recarregar_preserva_os_dados_roundtrip() {
        let dir = tempdir().expect("tempdir");
        // subpasta inexistente: `salvar` deve criar o diretório pai.
        let caminho = dir.path().join("cache").join("classificacao.json");
        let mut cache = CacheArquivo::carregar(&caminho);
        cache.inserir("link1", "Progressão");
        cache.inserir("link2", "Outros");

        cache.salvar().expect("deve salvar em disco");

        let recarregado = CacheArquivo::carregar(&caminho);
        assert_eq!(recarregado.obter("link1"), Some("Progressão"));
        assert_eq!(recarregado.obter("link2"), Some("Outros"));
    }

    #[test]
    fn arquivo_corrompido_comeca_vazio_sem_falhar() {
        let dir = tempdir().expect("tempdir");
        let caminho = dir.path().join("cache.json");
        fs::write(&caminho, "isto não é JSON válido").expect("grava arquivo corrompido");

        let cache = CacheArquivo::carregar(&caminho);

        assert_eq!(cache.obter("qualquer"), None);
    }
}
