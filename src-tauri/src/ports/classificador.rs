//! Strategy (GoF) — política de classificação de um Documento em exatamente
//! uma Categoria (R4). Este MVP inclui só a estratégia `keyword`; a
//! estratégia `llm` (ver `docs/ontology.yaml` `ModoClassificacao`) é **TODO**
//! da US5.

use crate::domain::categoria::{Categoria, OUTROS};
use crate::domain::documento::Documento;

pub trait Classificador {
    /// Classifica `documento` em exatamente uma das `categorias`; se nenhuma
    /// se aplicar, MUST retornar `"Outros"` (R4).
    fn classificar(&self, documento: &Documento, categorias: &[Categoria]) -> String;
}

/// Estratégia simples e sem custo de API: primeira categoria cujo nome
/// aparece (case-insensitive) no título ou no trecho do documento.
pub struct ClassificadorPalavraChave;

impl Classificador for ClassificadorPalavraChave {
    fn classificar(&self, documento: &Documento, categorias: &[Categoria]) -> String {
        let texto = format!(
            "{} {}",
            documento.titulo,
            documento.trecho.as_deref().unwrap_or("")
        )
        .to_lowercase();

        categorias
            .iter()
            .find(|c| texto.contains(&c.nome.to_lowercase()))
            .map(|c| c.nome.clone())
            .unwrap_or_else(|| OUTROS.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifica_pela_primeira_categoria_cujo_nome_aparece_no_texto() {
        let doc = Documento::novo("link", "PORTARIA Nº 1 - 2024 - Progressão funcional");
        let categorias = vec![
            Categoria::nova("Progressão", None),
            Categoria::nova("Comissão", None),
        ];
        let resultado = ClassificadorPalavraChave.classificar(&doc, &categorias);
        assert_eq!(resultado, "Progressão");
    }

    #[test]
    fn cai_em_outros_quando_nenhuma_categoria_combina() {
        let doc = Documento::novo("link", "Comunicado interno qualquer");
        let categorias = vec![Categoria::nova("Progressão", None)];
        let resultado = ClassificadorPalavraChave.classificar(&doc, &categorias);
        assert_eq!(resultado, OUTROS);
    }
}
