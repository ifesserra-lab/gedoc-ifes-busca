//! Documento — ato administrativo recuperado de uma busca no GeDoc.
//! Campos e regras conforme `data-model.md` / `docs/ontology.yaml`.

use serde::{Deserialize, Serialize};

use super::texto::{extrair_ano, extrair_numero};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TipoDocumento {
    Portaria,
    Despacho,
    Outro,
}

impl TipoDocumento {
    /// Deriva o tipo do prefixo do título (R1 — deriva do conteúdo real).
    pub fn from_titulo(titulo: &str) -> Self {
        let inicio = titulo.trim_start().to_uppercase();
        if inicio.starts_with("PORTARIA") {
            TipoDocumento::Portaria
        } else if inicio.starts_with("DESPACHO") {
            TipoDocumento::Despacho
        } else {
            TipoDocumento::Outro
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Documento {
    /// Chave do documento: URL canônica `/documento/<hash32>?inline`.
    pub link: String,
    pub titulo: String,
    pub tipo: TipoDocumento,
    /// Número do ato, após "Nº" no título.
    pub numero: Option<String>,
    /// Ano do ato (do título; senão, da data).
    pub ano: Option<u16>,
    /// Data bruta no formato `DD/MM/AAAA`, como veio do portal.
    pub data: Option<String>,
    /// Trecho ("snippet") destacado retornado pela busca.
    pub trecho: Option<String>,
    /// Todos os SIAPE citados no trecho (rotulados como "SIAPE N").
    pub siapes: Vec<String>,
    /// Nome determinístico do PDF baixado (R3); preenchido no download.
    pub arquivo: Option<String>,
    /// R2: verdadeiro se o SIAPE buscado aparece no trecho do documento.
    pub contem_siape: bool,
    /// Categoria atribuída (R4); None até a etapa de classificação (US5).
    pub categoria: Option<String>,
    /// Resumo fiel à fonte (R1); None até a etapa de resumo (US6).
    pub resumo: Option<String>,
}

impl Documento {
    /// Constrói um Documento mínimo a partir de link+título, derivando tipo,
    /// número e ano do próprio título (sem necessidade de dados de rede).
    pub fn novo(link: impl Into<String>, titulo: impl Into<String>) -> Self {
        let titulo = titulo.into();
        let tipo = TipoDocumento::from_titulo(&titulo);
        let numero = extrair_numero(&titulo);
        let ano = extrair_ano(&titulo, None);
        Documento {
            link: link.into(),
            titulo,
            tipo,
            numero,
            ano,
            data: None,
            trecho: None,
            siapes: Vec::new(),
            arquivo: None,
            contem_siape: true,
            categoria: None,
            resumo: None,
        }
    }

    /// SIAPEs citados no trecho, sem duplicatas, preservando a ordem.
    pub fn outros_siapes(&self) -> Vec<String> {
        let mut vistos = std::collections::HashSet::new();
        self.siapes
            .iter()
            .filter(|s| vistos.insert((*s).clone()))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deriva_tipo_numero_e_ano_do_titulo() {
        let d = Documento::novo("link1", "PORTARIA Nº 10 - 2024 - Progressão");
        assert_eq!(d.tipo, TipoDocumento::Portaria);
        assert_eq!(d.numero.as_deref(), Some("10"));
        assert_eq!(d.ano, Some(2024));
        assert!(d.contem_siape, "documento novo começa incluído por padrão");
    }

    #[test]
    fn tipo_outro_quando_prefixo_nao_reconhecido() {
        let d = Documento::novo("link2", "Ofício circular qualquer");
        assert_eq!(d.tipo, TipoDocumento::Outro);
    }

    #[test]
    fn outros_siapes_remove_duplicatas_preservando_ordem() {
        let mut d = Documento::novo("link3", "DESPACHO Nº 1 - 2024 - Assunto");
        d.siapes = vec!["111".into(), "222".into(), "111".into()];
        assert_eq!(d.outros_siapes(), vec!["111".to_string(), "222".to_string()]);
    }
}
