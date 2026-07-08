//! Fronteira de erro HTTP. Reusa `AppError` do núcleo e o serializa no MESMO
//! formato do IPC desktop (`{tipo, mensagem}`), para que
//! `app/src/services/ipc.ts::mensagemDeErro` funcione sem alteração (FR-014).

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use gedocs_lib::error::AppError;

/// Wrapper para usar `?` em handlers que devolvem `Result<_, ApiError>`.
pub struct ApiError(pub AppError);

impl From<AppError> for ApiError {
    fn from(e: AppError) -> Self {
        ApiError(e)
    }
}

/// Mapeia o `tipo` do erro para o status HTTP (ver contracts/http-api.md).
pub fn status_de(erro: &AppError) -> StatusCode {
    match erro {
        AppError::SiapeInvalido { .. }
        | AppError::CategoriaSemNome
        | AppError::NomeDuplicado { .. } => StatusCode::BAD_REQUEST,
        AppError::FalhaPortal { .. } | AppError::FalhaIA { .. } => StatusCode::BAD_GATEWAY,
        AppError::NaoImplementado(_) => StatusCode::NOT_IMPLEMENTED,
        AppError::FalhaArquivo { .. } => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Constrói uma resposta HTTP a partir de um `AppError` (corpo `{tipo,mensagem}`).
/// Usada tanto por `ApiError` quanto pelos handlers que devolvem `Response`.
pub fn resposta(erro: &AppError, status: StatusCode) -> Response {
    let corpo = serde_json::to_value(erro).unwrap_or_else(
        |_| serde_json::json!({"tipo":"FalhaArquivo","mensagem":{"motivo":"erro inesperado"}}),
    );
    (status, Json(corpo)).into_response()
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = status_de(&self.0);
        resposta(&self.0, status)
    }
}
