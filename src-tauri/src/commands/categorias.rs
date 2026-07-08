//! Controllers Tauri `listar_categorias`/`salvar_categorias` (US8 — CRUD).
//!
//! Persiste em `<app_config_dir>/categoria.json` (fora do VCS), semeado do
//! `config/categoria.json` versionado na 1ª execução. Fronteira fina: resolve
//! o caminho via `AppHandle` e delega a `gedocs_core::services::categorias`
//! (que recebe caminhos já resolvidos e é 100% testável — Princípio VII).

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use gedocs_core::domain::categoria::Categoria;
use gedocs_core::error::AppError;
use gedocs_core::services::categorias as categorias_service;

/// Nome do arquivo de categorias dentro do diretório de configuração do app.
const ARQUIVO_CATEGORIAS: &str = "categoria.json";

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SalvarResposta {
    pub ok: bool,
    pub total: usize,
}

/// `<app_config_dir>/categoria.json` — onde o CRUD (US8) lê/grava; também
/// usado por `commands::buscar` (US5) para ler o mesmo arquivo. Único ponto
/// deste módulo que conhece o `AppHandle`.
pub fn caminho_categorias_app(app: &AppHandle) -> Result<PathBuf, AppError> {
    let base = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::FalhaArquivo {
            motivo: format!("Falha ao localizar o diretório de configuração do app: {e}"),
        })?;
    Ok(base.join(ARQUIVO_CATEGORIAS))
}

#[tauri::command]
pub fn listar_categorias(app: AppHandle) -> Result<Vec<Categoria>, AppError> {
    let caminho_app = caminho_categorias_app(&app)?;
    categorias_service::resolver_com_semente(&caminho_app, &categorias_service::caminho_padrao())
}

#[tauri::command]
pub fn salvar_categorias(
    app: AppHandle,
    categorias: Vec<Categoria>,
) -> Result<SalvarResposta, AppError> {
    let caminho_app = caminho_categorias_app(&app)?;
    let total = categorias_service::salvar_categorias(&caminho_app, &categorias)?;
    Ok(SalvarResposta { ok: true, total })
}
