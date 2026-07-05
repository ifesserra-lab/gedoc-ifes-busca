//! Comando `buscar_por_siape` — ver `contracts/ipc-commands.md`.
//!
//! Cobre US1 (coleta completa), US2 (filtro por SIAPE) e US3 (consumo pela
//! View). Este MVP valida a entrada (R10) e já expõe o formato de saída
//! (`ResultadoView`) definido no contrato; a busca real no portal GeDoc
//! (sessão, IDs JSF dinâmicos — R8, paginação — FR-001) é **TODO**: sem uma
//! implementação de `GedocRepository`, o núcleo testável fica em
//! `domain`/`services` (validação + parser), sem tocar rede.

use serde::{Deserialize, Serialize};

use crate::domain::siape;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct BuscarPorSiapeInput {
    pub siape: String,
    pub repositorio: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DocView {
    pub titulo: String,
    pub data: Option<String>,
    pub link: String,
    pub arquivo: Option<String>,
    pub resumo: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CategoriaGrupo {
    pub categoria: String,
    pub qtd: usize,
    pub itens: Vec<DocView>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ResultadoView {
    pub termo: String,
    pub total: u32,
    pub categorias: Vec<CategoriaGrupo>,
    pub tem_pdf: bool,
}

/// Núcleo testável do comando (sem depender do runtime Tauri): valida o
/// SIAPE (R10) antes de qualquer outra coisa — falha rápida, sem tocar rede.
///
/// TODO(rede): quando `services::gedoc_repository` existir, trocar o `Err`
/// final por: buscar (US1) -> `services::filtro::filtrar_por_siape` (R2) ->
/// montar `ResultadoView` agrupado por categoria (US5/US6, ainda não
/// implementadas neste MVP).
pub async fn executar(siape: &str, _repositorio: Option<&str>) -> Result<ResultadoView, AppError> {
    siape::validar(siape)?;

    Err(AppError::FalhaPortal {
        motivo: "Busca no portal GeDoc ainda não implementada (TODO: GedocRepository/rede)."
            .to_string(),
    })
}

#[tauri::command]
pub async fn buscar_por_siape(input: BuscarPorSiapeInput) -> Result<ResultadoView, AppError> {
    executar(&input.siape, input.repositorio.as_deref()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejeita_siape_invalido_sem_tocar_rede() {
        let erro = executar("abc", None).await.unwrap_err();
        assert!(matches!(erro, AppError::SiapeInvalido { .. }));
    }

    #[tokio::test]
    async fn rejeita_siape_curto_demais() {
        let erro = executar("123", None).await.unwrap_err();
        assert!(matches!(erro, AppError::SiapeInvalido { .. }));
    }

    #[tokio::test]
    async fn siape_valido_ainda_sem_rede_retorna_falha_portal_explicita() {
        let erro = executar("1998547", None).await.unwrap_err();
        assert!(matches!(erro, AppError::FalhaPortal { .. }));
    }
}
