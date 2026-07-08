//! R10 — validação da matrícula SIAPE: `^[0-9]{5,8}$`.
//! Fonte de referência (legado): `src/app.py::_SIAPE_RE`.

use std::sync::LazyLock;

use regex::Regex;

use crate::error::AppError;

static SIAPE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9]{5,8}$").unwrap());

/// Verdadeiro se `termo` casa exatamente `^[0-9]{5,8}$`.
pub fn eh_siape(termo: &str) -> bool {
    SIAPE_RE.is_match(termo)
}

/// Valida o termo de busca como SIAPE (R10). Usado tanto pelo comando IPC
/// quanto por qualquer serviço que precise garantir a entrada antes de tocar
/// a rede.
pub fn validar(termo: &str) -> Result<(), AppError> {
    if eh_siape(termo) {
        Ok(())
    } else {
        Err(AppError::SiapeInvalido {
            termo: termo.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aceita_de_5_a_8_digitos() {
        for termo in ["12345", "123456", "1234567", "12345678"] {
            assert!(eh_siape(termo), "deveria aceitar {termo}");
            assert!(validar(termo).is_ok());
        }
    }

    #[test]
    fn rejeita_menos_de_5_digitos() {
        assert!(!eh_siape("1234"));
        assert!(validar("1234").is_err());
    }

    #[test]
    fn rejeita_mais_de_8_digitos() {
        assert!(!eh_siape("123456789"));
    }

    #[test]
    fn rejeita_nao_numericos_e_vazio() {
        assert!(!eh_siape("19985ab"));
        assert!(!eh_siape(""));
        assert!(!eh_siape(" 123456"));
        assert!(!eh_siape("123456 "));
    }

    #[test]
    fn erro_carrega_o_termo_original() {
        match validar("abc") {
            Err(AppError::SiapeInvalido { termo }) => assert_eq!(termo, "abc"),
            other => panic!("esperava SiapeInvalido, veio {other:?}"),
        }
    }
}
