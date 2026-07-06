//! Comandos `listar_categorias`/`salvar_categorias` (US8 — CRUD de
//! categorias), ver `contracts/ipc-commands.md`.
//!
//! **Decisão de persistência**: para não gravar em runtime dentro do
//! repositório (nem exigir permissão de escrita nele), o CRUD persiste em
//! `AppHandle.path().app_config_dir()/categoria.json` — fora do VCS, como o
//! `<app_data_dir>/documentos` de `commands::documento` (US4). O
//! `config/categoria.json` versionado passa a ser só a **semente**: na
//! primeira execução, se o arquivo do app_config ainda não existir, ele é
//! copiado de lá (`services::categorias::resolver_com_semente`); depois
//! disso, o arquivo do app_config é a única fonte de verdade — edições feitas
//! nesta tela nunca são perdidas nem sobrescritas pela semente.
//!
//! `commands::buscar::executar` (US5) foi ajustado para ler categorias do
//! MESMO caminho (`caminho_categorias_app`, exportado deste módulo) em vez de
//! `services::categorias::caminho_padrao()` direto — uma categoria criada ou
//! editada aqui aparece na busca seguinte. Nenhum `AppHandle` chega às
//! funções de `services::categorias`: elas recebem caminhos já resolvidos e
//! continuam 100% testáveis com `tempfile`, sem depender do runtime Tauri
//! (Princípio VII) — o mesmo padrão de `commands::documento::dir_documentos`.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::domain::categoria::Categoria;
use crate::error::AppError;
use crate::services::categorias as categorias_service;

/// Nome do arquivo de categorias dentro do diretório de configuração do app.
const ARQUIVO_CATEGORIAS: &str = "categoria.json";

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SalvarResposta {
    pub ok: bool,
    pub total: usize,
}

/// `<app_config_dir>/categoria.json` — onde o CRUD (US8) lê e grava; também
/// usado por `commands::buscar` para a classificação (US5) ler o mesmo
/// arquivo. Único ponto deste módulo que conhece o `AppHandle` — não testado
/// unitariamente, pois depende do runtime Tauri real (mesmo padrão de
/// `commands::documento::dir_documentos`).
pub fn caminho_categorias_app(app: &AppHandle) -> Result<PathBuf, AppError> {
    let base = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::FalhaArquivo {
            motivo: format!("Falha ao localizar o diretório de configuração do app: {e}"),
        })?;
    Ok(base.join(ARQUIVO_CATEGORIAS))
}

/// Lista as categorias cadastradas (US8). Semeia o arquivo do app_config a
/// partir do `config/categoria.json` empacotado na primeira execução (ver
/// doc do módulo); nunca falha por arquivo ausente — uma lista vazia leva a
/// tela ao estado "vazio" (com ação de criar a primeira categoria).
#[tauri::command]
pub fn listar_categorias(app: AppHandle) -> Result<Vec<Categoria>, AppError> {
    let caminho_app = caminho_categorias_app(&app)?;
    categorias_service::resolver_com_semente(&caminho_app, &categorias_service::caminho_padrao())
}

/// Substitui a lista completa de categorias (US8). Valida R5 (nome
/// obrigatório e único, case-insensitive) antes de gravar qualquer coisa —
/// ver `services::categorias::salvar_categorias`.
#[tauri::command]
pub fn salvar_categorias(
    app: AppHandle,
    categorias: Vec<Categoria>,
) -> Result<SalvarResposta, AppError> {
    let caminho_app = caminho_categorias_app(&app)?;
    let total = categorias_service::salvar_categorias(&caminho_app, &categorias)?;
    Ok(SalvarResposta { ok: true, total })
}
