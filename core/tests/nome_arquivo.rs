//! R3 (integração): `Documento.arquivo` MUST seguir `AAAA_NUMERO_ASSUNTO.pdf`;
//! colisões recebem sufixo, sem sobrescrever.

use std::collections::HashSet;

use gedocs_core::domain::documento::Documento;
use gedocs_core::domain::nome_arquivo::{nome_arquivo, nome_unico};

#[test]
fn deriva_nome_a_partir_do_titulo_com_ano_e_numero() {
    let d = Documento::novo(
        "link",
        "PORTARIA Nº 123 - 2024 - Progressão funcional de servidor",
    );
    assert_eq!(
        nome_arquivo(&d),
        "2024_123_Progressão funcional de servidor.pdf"
    );
}

#[test]
fn usa_ano_da_data_quando_titulo_nao_tem_ano() {
    let mut d = Documento::novo("link", "DESPACHO Nº 45 - Anuência de férias");
    d.data = Some("05/03/2023".to_string());
    assert!(nome_arquivo(&d).starts_with("2023_45_"));
}

#[test]
fn colisao_de_nomes_recebe_sufixo_numerico_sem_sobrescrever() {
    let mut usados = HashSet::new();
    let doc = Documento::novo("link", "PORTARIA Nº 1 - 2024 - Mesmo Assunto");
    let nome = nome_arquivo(&doc);

    let a = nome_unico(&nome, &mut usados);
    let b = nome_unico(&nome, &mut usados);
    let c = nome_unico(&nome, &mut usados);

    assert_eq!(a, nome);
    assert_ne!(b, a);
    assert_ne!(c, a);
    assert_ne!(c, b);
    assert_eq!(usados.len(), 3, "nenhum nome deve se sobrescrever");
}
