//! Parser da resposta parcial do portal (integração, via API pública):
//! extrai total, título, data, trecho e SIAPEs citados; erros do portal
//! (mensagem de erro, sessão expirada) viram `AppError::FalhaPortal`.

use gedocs_core::error::AppError;
use gedocs_core::services::gedoc_parse::parse_resposta;

const FIXTURE_OK: &str = include_str!("fixtures/resposta_ok.xml");
const FIXTURE_ERRO: &str = include_str!("fixtures/resposta_erro.xml");
const FIXTURE_REDIRECT: &str = include_str!("fixtures/resposta_redirect.xml");
const FIXTURE_DUPLICADA: &str = include_str!("fixtures/resposta_duplicada.xml");

#[test]
fn extrai_total_e_todos_os_documentos_sem_duplicatas() {
    let r = parse_resposta(FIXTURE_OK).expect("deve parsear");
    assert_eq!(r.total, Some(1234));
    assert_eq!(r.documentos.len(), 3);
}

#[test]
fn cada_documento_traz_link_titulo_data_trecho_e_siapes() {
    let r = parse_resposta(FIXTURE_OK).unwrap();
    let d1 = &r.documentos[0];
    assert!(d1.link.starts_with("https://gedoc.ifes.edu.br/documento/"));
    assert_eq!(d1.titulo, "PORTARIA Nº 10 - 2024 - Progressão de servidor");
    assert_eq!(d1.data.as_deref(), Some("10/01/2024"));
    assert_eq!(d1.siapes, vec!["1998547".to_string()]);
}

#[test]
fn erro_do_servidor_e_sessao_expirada_viram_falha_portal() {
    assert!(matches!(
        parse_resposta(FIXTURE_ERRO).unwrap_err(),
        AppError::FalhaPortal { .. }
    ));
    assert!(matches!(
        parse_resposta(FIXTURE_REDIRECT).unwrap_err(),
        AppError::FalhaPortal { .. }
    ));
}

#[test]
fn deduplica_documentos_com_o_mesmo_link() {
    let r = parse_resposta(FIXTURE_DUPLICADA).expect("deve parsear");
    assert_eq!(r.documentos.len(), 1);
}
