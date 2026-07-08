//! R10 (integração, via API pública do crate): `Servidor.siape` e o termo de
//! busca MUST casar `^[0-9]{5,8}$`.

use gedocs_core::domain::siape::{eh_siape, validar};
use gedocs_core::error::AppError;

#[test]
fn aceita_siape_de_5_a_8_digitos() {
    for termo in ["12345", "123456", "1234567", "12345678"] {
        assert!(eh_siape(termo), "deveria aceitar {termo}");
        assert!(validar(termo).is_ok());
    }
}

#[test]
fn rejeita_menos_de_5_digitos() {
    let erro = validar("1234").unwrap_err();
    assert_eq!(
        erro,
        AppError::SiapeInvalido {
            termo: "1234".to_string()
        }
    );
}

#[test]
fn rejeita_mais_de_8_digitos() {
    assert!(validar("123456789").is_err());
}

#[test]
fn rejeita_nao_numericos_espacos_e_vazio() {
    for termo in ["19985ab", "", " 123456", "123456 ", "12 345"] {
        assert!(validar(termo).is_err(), "deveria rejeitar '{termo}'");
    }
}
