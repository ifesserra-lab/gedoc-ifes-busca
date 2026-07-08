//! Controllers Tauri `baixar_documento`/`abrir_documento` (US4).
//!
//! Fronteira fina: resolvem o diretório de dados do app (`app_data_dir`) e
//! delegam aos use-cases puros `gedocs_core::usecases::documento`. PDFs contêm
//! PII de terceiros (Princípio II/LGPD, R7): sempre sob `app_data_dir`.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use gedocs_core::domain::siape;
use gedocs_core::dto::{AbrirDocumentoInput, BaixarDocumentoInput};
use gedocs_core::error::AppError;
use gedocs_core::ports::http::ReqwestHttp;
use gedocs_core::services::downloader::SUBPASTA_DOCUMENTOS;
use gedocs_core::usecases::documento::{executar_download, resolver_caminho_abertura};

/// `<app_data_dir>/documentos` — raiz de todos os downloads (fora do VCS).
/// Único ponto que conhece o `AppHandle`; `pub(crate)` pois `commands::buscar`
/// (US6) e `commands::exportar` (US7) também o usam.
pub(crate) fn dir_documentos(app: &AppHandle) -> Result<PathBuf, AppError> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::FalhaArquivo {
            motivo: format!("Falha ao localizar o diretório de dados do app: {e}"),
        })?;
    Ok(base.join(SUBPASTA_DOCUMENTOS))
}

#[tauri::command]
pub async fn baixar_documento(
    app: AppHandle,
    input: BaixarDocumentoInput,
) -> Result<String, AppError> {
    siape::validar(&input.siape)?;
    let dir_base = dir_documentos(&app)?;

    tokio::task::spawn_blocking(move || {
        let http = ReqwestHttp::novo()?;
        executar_download(&http, &dir_base, &input)
    })
    .await
    .map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Falha interna ao baixar o documento: {e}"),
    })?
}

#[tauri::command]
pub fn abrir_documento(app: AppHandle, input: AbrirDocumentoInput) -> Result<(), AppError> {
    let dir_base = dir_documentos(&app)?;
    let caminho = resolver_caminho_abertura(&dir_base, &input)?;

    app.opener()
        .open_path(caminho.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|e| AppError::FalhaArquivo {
            motivo: format!("Falha ao abrir o documento: {e}"),
        })
}
