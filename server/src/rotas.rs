//! Handlers HTTP — finos, delegam aos use-cases puros de `gedocs_lib`
//! (Princípio V/VIII). Ver contrato em `specs/003-versao-web/contracts/http-api.md`.

use axum::{
    body::Body,
    extract::{Extension, Path},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use gedocs_core::domain::siape::eh_siape;
use gedocs_core::dto::{
    AbrirDocumentoInput, BaixarDocumentoInput, BuscarPorSiapeInput, ResultadoView,
};
use gedocs_core::error::AppError;
use gedocs_core::ports::http::ReqwestHttp;
use gedocs_core::services::classificador::ModoClassificacao;
use gedocs_core::services::empacotador::montar_zip;
use gedocs_core::usecases::buscar::executar;
use gedocs_core::usecases::documento::{executar_download, resolver_caminho_abertura};
use gedocs_core::usecases::exportar::executar_gerar_relatorio;

use crate::erro::{resposta, ApiError};
use crate::sessao::SessionCtx;

const SUB_DOCS: &str = "documentos";
const SUB_REL: &str = "relatorios";
const SUB_CACHE: &str = "cache";

// ------------------------------------------------------------------ helpers //

fn erro_interno(msg: impl std::fmt::Display) -> AppError {
    AppError::FalhaArquivo {
        motivo: msg.to_string(),
    }
}

fn arquivo_resp(bytes: Vec<u8>, content_type: &str, disposition: &str) -> Response {
    let mut resp = Response::new(Body::from(bytes));
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, content_type.parse().unwrap());
    if let Ok(v) = disposition.parse() {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    resp
}

// ------------------------------------------------------------------- health //

/// `/api/health` — liveness + marcador de build (para confirmar, após um
/// deploy, qual versão está de fato no ar). `versao` vem do `Cargo.toml`
/// (compile time); `commit` do SHA que o Render injeta em runtime
/// (`RENDER_GIT_COMMIT`) — ausente/vazio (ex.: rodando local) vira
/// `"desconhecido"`. Corpo montado por `corpo_health` (função pura, testável).
pub async fn health() -> Json<serde_json::Value> {
    Json(corpo_health(std::env::var("RENDER_GIT_COMMIT").ok()))
}

fn corpo_health(commit: Option<String>) -> serde_json::Value {
    let commit = commit
        .map(|s| s.trim().chars().take(12).collect::<String>())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "desconhecido".to_string());
    json!({
        "ok": true,
        "versao": env!("CARGO_PKG_VERSION"),
        "commit": commit,
    })
}

// -------------------------------------------------------------------- US1/US4 //

pub async fn buscar(
    Extension(sess): Extension<SessionCtx>,
    Json(input): Json<BuscarPorSiapeInput>,
) -> Result<Json<ResultadoView>, ApiError> {
    let modo = ModoClassificacao::from_entrada(input.modo.as_deref());
    let dir_docs = sess.dir.join(SUB_DOCS);
    let cache_cat = Some(sess.dir.join(SUB_CACHE).join("classificacao.json"));
    let cache_res = Some(sess.dir.join(SUB_CACHE).join("resumo.json"));
    // Categorias sempre do `config/categoria.json` (fonte única; web sem CRUD,
    // spec 005): `None` faz o núcleo usar `caminho_padrao()` = config, sem cópia
    // seed em `data/`. Assim adicionar uma categoria no json reflete na
    // classificação (após deploy), sem estado stale.
    let categorias_path: Option<std::path::PathBuf> = None;

    let resultado = executar(
        &input.siape,
        input.repositorio.as_deref(),
        modo,
        cache_cat,
        dir_docs,
        cache_res,
        categorias_path,
        input.por_nome(),
    )
    .await?;
    Ok(Json(resultado))
}

// --------------------------------------------------------------------- US3 //

pub async fn baixar_documento(
    Extension(sess): Extension<SessionCtx>,
    Json(input): Json<BaixarDocumentoInput>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let dir_base = sess.dir.join(SUB_DOCS);
    let nome = tokio::task::spawn_blocking(move || {
        let http = ReqwestHttp::novo()?;
        executar_download(&http, &dir_base, &input)
    })
    .await
    .map_err(erro_interno)??;
    Ok(Json(json!({ "arquivo": nome })))
}

pub async fn abrir_documento(
    Extension(sess): Extension<SessionCtx>,
    Path((siape, arquivo)): Path<(String, String)>,
) -> Response {
    let dir_base = sess.dir.join(SUB_DOCS);
    let input = AbrirDocumentoInput { siape, arquivo };
    // `resolver_caminho_abertura` sanitiza siape/arquivo (R7) e confere existência.
    let caminho = match resolver_caminho_abertura(&dir_base, &input) {
        Ok(c) => c,
        Err(e) => return resposta(&e, StatusCode::NOT_FOUND),
    };
    match std::fs::read(&caminho) {
        Ok(bytes) => arquivo_resp(bytes, "application/pdf", "inline"),
        Err(_) => resposta(
            &AppError::FalhaArquivo {
                motivo: "Arquivo não encontrado.".into(),
            },
            StatusCode::NOT_FOUND,
        ),
    }
}

// --------------------------------------------------------------------- US5 //

pub async fn gerar_relatorio(
    Extension(sess): Extension<SessionCtx>,
    Json(resultado): Json<ResultadoView>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let dir_saida = sess.dir.join(SUB_REL);
    let nome =
        tokio::task::spawn_blocking(move || executar_gerar_relatorio(&resultado, &dir_saida))
            .await
            .map_err(erro_interno)??;
    Ok(Json(json!({ "arquivo": nome })))
}

pub async fn servir_relatorio(
    Extension(sess): Extension<SessionCtx>,
    Path(siape): Path<String>,
) -> Response {
    if !eh_siape(&siape) {
        return resposta(
            &AppError::SiapeInvalido { termo: siape },
            StatusCode::BAD_REQUEST,
        );
    }
    let caminho = sess
        .dir
        .join(SUB_REL)
        .join(format!("{siape}_relatorio.html"));
    match std::fs::read(&caminho) {
        Ok(bytes) => arquivo_resp(bytes, "text/html; charset=utf-8", "inline"),
        // Tipo próprio `RelatorioAusente` (não `FalhaArquivo`): não é falha de
        // acesso a arquivo, é orientação. O front mostra a mensagem limpa,
        // sem o prefixo "Falha ao acessar arquivo:".
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "tipo": "RelatorioAusente",
                "mensagem": {"motivo": "Relatório não encontrado. Faça a busca no modo IA e \
                    clique em 'Baixar relatório' na tela antes de abrir este link — o relatório \
                    é gerado por sessão e consolida os resumos da IA."}
            })),
        )
            .into_response(),
    }
}

// --------------------------------------------------------------------- US6 //

pub async fn baixar_zip(
    Extension(sess): Extension<SessionCtx>,
    Path(siape): Path<String>,
) -> Response {
    if !eh_siape(&siape) {
        return resposta(
            &AppError::SiapeInvalido { termo: siape },
            StatusCode::BAD_REQUEST,
        );
    }
    let dir_siape = sess.dir.join(SUB_DOCS).join(&siape);
    let nome_zip = format!("{siape}_documentos.zip");
    let saida = sess.dir.join(SUB_REL).join(&nome_zip);

    let saida_task = saida.clone();
    let res = tokio::task::spawn_blocking(move || montar_zip(&dir_siape, &saida_task)).await;

    match res {
        Ok(Ok(_)) => match std::fs::read(&saida) {
            Ok(bytes) => arquivo_resp(
                bytes,
                "application/zip",
                &format!("attachment; filename=\"{nome_zip}\""),
            ),
            Err(e) => resposta(&erro_interno(e), StatusCode::INTERNAL_SERVER_ERROR),
        },
        Ok(Err(app_err)) => {
            let st = crate::erro::status_de(&app_err);
            resposta(&app_err, st)
        }
        Err(join) => resposta(&erro_interno(join), StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// Sem CRUD de categorias na web (spec 005): a gestão só existe no desktop
// (comandos Tauri). A busca continua classificando pelas categorias globais
// (o núcleo lê `categoria.json` internamente, sem endpoint).

// ---------------------------------------------------------------- rate limit //

/// 429 amigável — tipo próprio `LimiteTaxa` (não culpa o portal); o front
/// mapeia em `mensagemDeErro`.
pub fn resposta_rate_limit() -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({
            "tipo": "LimiteTaxa",
            "mensagem": {"motivo": "Muitas requisições. Tente novamente em instantes."}
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_expoe_ok_versao_e_commit_curto() {
        let corpo = corpo_health(Some("1c7f3ecabcdef0123456789deadbeef".to_string()));
        assert_eq!(corpo["ok"], true);
        assert_eq!(corpo["versao"], env!("CARGO_PKG_VERSION"));
        // SHA truncado a 12 chars (suficiente pra identificar o build).
        assert_eq!(corpo["commit"], "1c7f3ecabcde");
    }

    #[test]
    fn health_sem_commit_no_ambiente_vira_desconhecido() {
        assert_eq!(corpo_health(None)["commit"], "desconhecido");
        assert_eq!(
            corpo_health(Some("   ".to_string()))["commit"],
            "desconhecido"
        );
    }
}
