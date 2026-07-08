//! Categoria — rótulo de classificação definido pelo usuário (R5).
//! Leitura de `config/categoria.json` implementada em `services::categorias`
//! (US5); CRUD completo (criar/editar/remover, persistindo de volta no
//! arquivo) é TODO de US8. Aqui só o modelo.

use serde::{Deserialize, Serialize};

/// Categoria padrão quando nenhuma outra se aplica (R4).
pub const OUTROS: &str = "Outros";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Categoria {
    pub nome: String,
    pub descricao: Option<String>,
}

impl Categoria {
    pub fn nova(nome: impl Into<String>, descricao: Option<String>) -> Self {
        Self {
            nome: nome.into(),
            descricao,
        }
    }
}
