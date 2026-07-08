//! Controllers Tauri `gerar_relatorio`/`baixar_zip` (US7).
//!
//! Fronteira fina: resolvem `<app_data_dir>/relatorios` e delegam ao use-case
//! puro `gedocs_core::usecases::exportar::executar_gerar_relatorio` e a
//! `services::empacotador::montar_zip`; depois abrem o HTML / revelam o ZIP via
//! plugin `opener`. Saídas contêm PII (Princípio II/LGPD): sempre sob
//! `app_data_dir`.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use gedocs_core::domain::siape;
use gedocs_core::dto::ResultadoView;
use gedocs_core::error::AppError;
use gedocs_core::services::empacotador;
use gedocs_core::usecases::exportar::executar_gerar_relatorio;

use super::documento;

const SUBPASTA_RELATORIOS: &str = "relatorios";

fn dir_relatorios(app: &AppHandle) -> Result<PathBuf, AppError> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::FalhaArquivo {
            motivo: format!("Falha ao localizar o diretório de dados do app: {e}"),
        })?;
    Ok(base.join(SUBPASTA_RELATORIOS))
}

#[tauri::command]
pub async fn gerar_relatorio(app: AppHandle, resultado: ResultadoView) -> Result<String, AppError> {
    siape::validar(&resultado.termo)?;
    let dir_saida = dir_relatorios(&app)?;
    let dir_saida_task = dir_saida.clone();

    let nome =
        tokio::task::spawn_blocking(move || executar_gerar_relatorio(&resultado, &dir_saida_task))
            .await
            .map_err(|e| AppError::FalhaArquivo {
                motivo: format!("Falha interna ao gerar o relatório: {e}"),
            })??;

    let caminho = dir_saida.join(&nome);
    app.opener()
        .open_path(caminho.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|e| AppError::FalhaArquivo {
            motivo: format!("Falha ao abrir o relatório: {e}"),
        })?;

    Ok(nome)
}

#[tauri::command]
pub async fn baixar_zip(app: AppHandle, siape: String) -> Result<String, AppError> {
    siape::validar(&siape)?;

    let dir_documentos = documento::dir_documentos(&app)?;
    let dir_saida = dir_relatorios(&app)?;
    let dir_saida_task = dir_saida.clone();

    let nome = tokio::task::spawn_blocking(move || -> Result<String, AppError> {
        let dir_siape = dir_documentos.join(&siape);
        let nome_zip = format!("{siape}_documentos.zip");
        let caminho_zip = dir_saida_task.join(&nome_zip);
        empacotador::montar_zip(&dir_siape, &caminho_zip)?;
        Ok(nome_zip)
    })
    .await
    .map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Falha interna ao montar o ZIP: {e}"),
    })??;

    let caminho = dir_saida.join(&nome);
    // Best-effort: se o SO não conseguir revelar o item, o ZIP já foi gravado.
    let _ = app.opener().reveal_item_in_dir(&caminho);

    Ok(nome)
}
