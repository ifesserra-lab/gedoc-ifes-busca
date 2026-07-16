//! Controller Tauri de `buscar_por_siape` (US1/US2/US3/US5/US6).
//!
//! Fronteira fina: resolve os diretórios do app via `AppHandle` e delega ao
//! use-case puro `gedocs_core::usecases::buscar::executar`. Os caches de IA
//! ficam em `app_data_dir` (fora do VCS, Princípio II); as categorias vêm do
//! mesmo arquivo que o CRUD (US8) grava, para que uma categoria criada/editada
//! na tela apareça já na próxima busca.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use gedocs_core::domain::siape;
use gedocs_core::dto::{BuscarPorSiapeInput, ResultadoView};
use gedocs_core::error::AppError;
use gedocs_core::services::classificador::ModoClassificacao;
use gedocs_core::services::downloader::SUBPASTA_DOCUMENTOS;
use gedocs_core::usecases::buscar::{
    executar, ARQUIVO_CACHE_CLASSIFICACAO, ARQUIVO_CACHE_RESUMO, SUBPASTA_CACHE,
};

use super::documento;

#[tauri::command]
pub async fn buscar_por_siape(
    app: AppHandle,
    input: BuscarPorSiapeInput,
) -> Result<ResultadoView, AppError> {
    // Modo `nome` (spec 009): termo livre, não valida SIAPE.
    let por_nome = input.por_nome();
    if !por_nome {
        siape::validar(&input.siape)?;
    }

    let modo = ModoClassificacao::from_entrada(input.modo.as_deref());
    // Caches só são relevantes no modo `llm`.
    let app_data_dir = (modo == ModoClassificacao::Llm)
        .then(|| app.path().app_data_dir().ok())
        .flatten();
    let cache_categoria_path = app_data_dir
        .clone()
        .map(|dir| dir.join(SUBPASTA_CACHE).join(ARQUIVO_CACHE_CLASSIFICACAO));
    let cache_resumo_path =
        app_data_dir.map(|dir| dir.join(SUBPASTA_CACHE).join(ARQUIVO_CACHE_RESUMO));
    // Best-effort: se `app_data_dir()` falhar, o resumo cai no trecho (R11).
    let dir_documentos =
        documento::dir_documentos(&app).unwrap_or_else(|_| PathBuf::from(SUBPASTA_DOCUMENTOS));
    // US8: mesmo arquivo que `commands::categorias` lê/grava.
    let categorias_path = crate::commands::categorias::caminho_categorias_app(&app).ok();

    executar(
        &input.siape,
        input.repositorio.as_deref(),
        modo,
        cache_categoria_path,
        dir_documentos,
        cache_resumo_path,
        categorias_path,
        por_nome,
    )
    .await
}
