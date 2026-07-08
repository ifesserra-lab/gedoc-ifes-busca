//! R2 (integração): um Documento só entra em `documentos` se o SIAPE buscado
//! aparecer no seu trecho; caso contrário vai para `descartados`. Combina o
//! parser (`services::gedoc_parse`) com o filtro (`services::filtro`) sobre a
//! mesma fixture usada pelos testes do parser — sem tocar rede.

use gedocs_core::domain::documento::Documento;
use gedocs_core::services::filtro::{filtrar_por_siape, separar};
use gedocs_core::services::gedoc_parse::parse_resposta;

const FIXTURE_OK: &str = include_str!("fixtures/resposta_ok.xml");

fn doc(trecho: &str, siapes: Vec<&str>) -> Documento {
    let mut d = Documento::novo(
        "https://gedoc.ifes.edu.br/documento/aaaa?inline",
        "PORTARIA Nº 1 - 2024 - Teste",
    );
    d.trecho = Some(trecho.to_string());
    d.siapes = siapes.into_iter().map(String::from).collect();
    d
}

#[test]
fn documento_que_cita_o_siape_no_trecho_permanece_incluido() {
    let mut docs = vec![doc(
        "Designa o servidor SIAPE 1998547 para...",
        vec!["1998547"],
    )];
    filtrar_por_siape(&mut docs, "1998547");
    assert!(docs[0].contem_siape);
}

#[test]
fn numero_fora_do_contexto_do_servidor_e_descartado() {
    let mut docs = vec![doc(
        "Processo administrativo 30022/2024 sobre outro assunto",
        vec![],
    )];
    filtrar_por_siape(&mut docs, "1998547");
    assert!(!docs[0].contem_siape);
}

#[test]
fn pipeline_parser_mais_filtro_separa_validos_e_descartados() {
    let resposta = parse_resposta(FIXTURE_OK).expect("fixture deve parsear");
    let mut documentos: Vec<Documento> = resposta
        .documentos
        .into_iter()
        .map(|d| d.para_documento())
        .collect();

    filtrar_por_siape(&mut documentos, "1998547");
    let (validos, descartados) = separar(documentos);

    // Doc 1 (rotulado "SIAPE 1998547") e doc 3 (matrícula 1998547 sem rótulo)
    // permanecem; doc 2 (processo 30022/2024) é descartado.
    assert_eq!(validos.len(), 2);
    assert_eq!(descartados.len(), 1);
    assert!(descartados[0].titulo.contains("Análise de processo"));
}
