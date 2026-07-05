//! R4 (integração, via API pública do crate): a estratégia por
//! palavra-chave (`ClassificadorPalavraChave`, default de `buscar_por_siape`)
//! classifica um Documento em exatamente uma Categoria; sem casamento, cai
//! em "Outros".

use gedocs_lib::domain::categoria::{Categoria, OUTROS};
use gedocs_lib::domain::documento::Documento;
use gedocs_lib::ports::classificador::{Classificador, ClassificadorPalavraChave};

fn categorias_padrao() -> Vec<Categoria> {
    vec![
        Categoria::nova(
            "Progressão",
            Some("Progressão funcional por mérito.".to_string()),
        ),
        Categoria::nova(
            "Comissão",
            Some("Designação de comissões, comitês ou bancas.".to_string()),
        ),
        Categoria::nova(OUTROS, None),
    ]
}

#[test]
fn classifica_pelo_nome_da_categoria_no_titulo() {
    let doc = Documento::novo("l1", "PORTARIA Nº 1 - 2024 - Progressão funcional");
    let categoria = ClassificadorPalavraChave.classificar(&doc, &categorias_padrao());
    assert_eq!(categoria, "Progressão");
}

#[test]
fn classifica_por_palavra_significativa_da_descricao_sem_citar_o_nome() {
    let doc = Documento::novo(
        "l2",
        "PORTARIA Nº 2 - 2024 - Designação de banca examinadora",
    );
    let categoria = ClassificadorPalavraChave.classificar(&doc, &categorias_padrao());
    assert_eq!(categoria, "Comissão");
}

#[test]
fn ignora_acentuacao_ao_casar_o_nome_da_categoria() {
    let doc = Documento::novo("l3", "PORTARIA Nº 3 - 2024 - PROGRESSAO por capacitacao");
    let categoria = ClassificadorPalavraChave.classificar(&doc, &categorias_padrao());
    assert_eq!(categoria, "Progressão");
}

#[test]
fn cai_em_outros_quando_nenhuma_categoria_casa() {
    let doc = Documento::novo("l4", "Comunicado interno sem relação com as categorias");
    let categoria = ClassificadorPalavraChave.classificar(&doc, &categorias_padrao());
    assert_eq!(categoria, OUTROS);
}

#[test]
fn cada_documento_recebe_exatamente_uma_categoria_e_a_soma_bate_com_o_total() {
    let categorias = categorias_padrao();
    let docs = [
        Documento::novo("l1", "PORTARIA Nº 1 - 2024 - Progressão funcional"),
        Documento::novo(
            "l2",
            "PORTARIA Nº 2 - 2024 - Designação de banca examinadora",
        ),
        Documento::novo("l3", "Comunicado interno qualquer"),
    ];
    let classificador = ClassificadorPalavraChave;

    let resultado: Vec<String> = docs
        .iter()
        .map(|d| classificador.classificar(d, &categorias))
        .collect();

    assert_eq!(
        resultado.len(),
        docs.len(),
        "cada documento recebe exatamente uma categoria (R4)"
    );
    assert_eq!(resultado, vec!["Progressão", "Comissão", OUTROS]);
}
