//! R2 — filtro por SIAPE: um Documento só entra em `documentos` se o SIAPE
//! buscado aparecer no seu trecho (rotulado "SIAPE N" ou não, pois o snippet
//! pode cortar a palavra "SIAPE" fora da janela exibida); caso contrário vai
//! para `descartados`. Fonte de referência (legado):
//! `src/buscar_gedoc.py::filtrar_por_siape`.

use crate::domain::documento::Documento;
use crate::domain::siape::eh_siape;

/// Marca `contem_siape` em cada documento. Quando `termo` não é um SIAPE
/// válido (R10), nada é filtrado — mantém o comportamento do legado, que só
/// aplica o filtro quando faz sentido (busca por SIAPE).
pub fn filtrar_por_siape(docs: &mut [Documento], termo: &str) {
    let procura_siape = eh_siape(termo);
    for d in docs.iter_mut() {
        d.contem_siape = if !procura_siape {
            true
        } else {
            d.siapes.iter().any(|s| s == termo)
                || d.trecho.as_deref().unwrap_or("").contains(termo)
        };
    }
}

/// Separa os documentos já marcados em (válidos, descartados), preservando a
/// ordem original em cada lista.
pub fn separar(docs: Vec<Documento>) -> (Vec<Documento>, Vec<Documento>) {
    docs.into_iter().partition(|d| d.contem_siape)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_com_trecho(trecho: &str, siapes: Vec<&str>) -> Documento {
        let mut d = Documento::novo(
            "https://gedoc.ifes.edu.br/documento/aaaa?inline",
            "PORTARIA Nº 1 - 2024 - Teste",
        );
        d.trecho = Some(trecho.to_string());
        d.siapes = siapes.into_iter().map(String::from).collect();
        d
    }

    #[test]
    fn marca_contem_siape_quando_termo_rotulado_aparece_no_trecho() {
        let mut docs = vec![doc_com_trecho(
            "Designa o servidor SIAPE 1998547 para exercer a função",
            vec!["1998547"],
        )];
        filtrar_por_siape(&mut docs, "1998547");
        assert!(docs[0].contem_siape);
    }

    #[test]
    fn marca_contem_siape_quando_numero_aparece_sem_rotulo_siape() {
        // O snippet pode cortar a palavra "SIAPE" fora da janela exibida.
        let mut docs = vec![doc_com_trecho(
            "O servidor matrícula 1998547 foi designado para a comissão",
            vec![],
        )];
        filtrar_por_siape(&mut docs, "1998547");
        assert!(docs[0].contem_siape);
    }

    #[test]
    fn descarta_quando_termo_nao_aparece_no_trecho() {
        // O portal casou a busca em outro lugar do documento (não no
        // trecho exibido) -- não há como confirmar que é sobre esse SIAPE.
        let mut docs = vec![doc_com_trecho(
            "Processo administrativo 30022/2024 referente a outro assunto",
            vec![],
        )];
        filtrar_por_siape(&mut docs, "1998547");
        assert!(!docs[0].contem_siape);
    }

    #[test]
    fn termo_invalido_como_siape_nao_filtra_nada() {
        let mut docs = vec![doc_com_trecho("qualquer coisa", vec![])];
        filtrar_por_siape(&mut docs, "abcabc");
        assert!(docs[0].contem_siape);
    }

    #[test]
    fn separar_particiona_validos_e_descartados_preservando_ordem() {
        let mut docs = vec![
            doc_com_trecho("SIAPE 1998547 designado", vec!["1998547"]),
            doc_com_trecho("nada a ver com o termo", vec![]),
        ];
        filtrar_por_siape(&mut docs, "1998547");
        let (validos, descartados) = separar(docs);
        assert_eq!(validos.len(), 1);
        assert_eq!(descartados.len(), 1);
        assert!(validos[0].trecho.as_deref().unwrap().contains("designado"));
    }
}
