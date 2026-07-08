//! Testes de integração da API web (sem rede — Princípio VII). Exercitam os
//! caminhos que não dependem do portal/IA: health, validação, sessão/isolamento,
//! TTL, categorias, relatório e zip vazio.

use std::{
    collections::HashMap,
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{header, Request, StatusCode},
    Router,
};
use gedocs_server::{app, AppState};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn estado(tmp: &Path, rate_limit: u32) -> AppState {
    AppState {
        data_dir: tmp.join("data"),
        session_ttl: Duration::from_secs(3600),
        secure_cookie: false,
        rate: Arc::new(Mutex::new(HashMap::new())),
        rate_limit,
    }
}

async fn call(
    router: &Router,
    method: &str,
    uri: &str,
    body: Option<&str>,
    cookie: Option<&str>,
) -> (StatusCode, Option<String>, Vec<u8>) {
    let mut b = Request::builder().method(method).uri(uri);
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    if let Some(c) = cookie {
        b = b.header(header::COOKIE, c);
    }
    let corpo = body
        .map(|s| Body::from(s.to_string()))
        .unwrap_or(Body::empty());
    let mut req = b.body(corpo).unwrap();
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1111))));

    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let setc = resp
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let bytes = resp
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    (status, setc, bytes)
}

/// "gedocs_sid=..." pronto para o header Cookie.
fn cookie_de(setc: &Option<String>) -> String {
    setc.as_ref()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

fn sid_de(setc: &Option<String>) -> String {
    cookie_de(setc)
        .strip_prefix("gedocs_sid=")
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn health_responde_ok() {
    let tmp = tempfile::tempdir().unwrap();
    let r = app(estado(tmp.path(), 1000));
    let (status, _c, body) = call(&r, "GET", "/api/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&body).contains("\"ok\":true"));
}

#[tokio::test]
async fn buscar_siape_invalido_400_sem_rede() {
    let tmp = tempfile::tempdir().unwrap();
    let r = app(estado(tmp.path(), 1000));
    let (status, _c, body) =
        call(&r, "POST", "/api/buscar", Some(r#"{"siape":"abc"}"#), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(String::from_utf8_lossy(&body).contains("SiapeInvalido"));
}

#[tokio::test]
async fn sem_endpoint_de_categorias_na_web() {
    // spec 005: a web não expõe CRUD de categorias. Os endpoints não existem
    // (404), tanto leitura quanto escrita.
    let tmp = tempfile::tempdir().unwrap();
    let r = app(estado(tmp.path(), 1000));

    let (get_status, _c, _b) = call(&r, "GET", "/api/categorias", None, None).await;
    assert_eq!(get_status, StatusCode::NOT_FOUND);

    let (put_status, _c, _b) = call(
        &r,
        "PUT",
        "/api/categorias",
        Some(r#"[{"nome":"X"}]"#),
        None,
    )
    .await;
    assert_eq!(put_status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn relatorio_gera_e_serve_na_mesma_sessao() {
    let tmp = tempfile::tempdir().unwrap();
    let r = app(estado(tmp.path(), 1000));

    let resultado = r#"{"termo":"1998547","total":0,"categorias":[],"tem_pdf":false}"#;
    let (status, setc, body) = call(&r, "POST", "/api/relatorio", Some(resultado), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&body).contains("1998547_relatorio.html"));

    // GET na MESMA sessão serve o HTML.
    let cookie = cookie_de(&setc);
    let (status, _c, body) = call(&r, "GET", "/api/relatorio/1998547", None, Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&body)
        .to_lowercase()
        .contains("html"));
}

#[tokio::test]
async fn zip_sem_pdfs_falha_amigavel() {
    let tmp = tempfile::tempdir().unwrap();
    let r = app(estado(tmp.path(), 1000));
    let (status, _c, _body) = call(&r, "GET", "/api/zip/1998547", None, None).await;
    assert!(
        status.is_client_error() || status.is_server_error(),
        "sem PDFs deve falhar, veio {status}"
    );
}

#[tokio::test]
async fn documento_isolado_por_sessao_sc002() {
    let tmp = tempfile::tempdir().unwrap();
    let st = estado(tmp.path(), 1000);
    let r = app(st.clone());

    // Sessão A: uma rota protegida emite o cookie; cria manualmente um PDF.
    let (_s, setc_a, _b) = call(&r, "GET", "/api/relatorio/1998547", None, None).await;
    let sid_a = sid_de(&setc_a);
    let doc = st
        .sessions_root()
        .join(&sid_a)
        .join("documentos")
        .join("1998547");
    std::fs::create_dir_all(&doc).unwrap();
    std::fs::write(doc.join("x.pdf"), b"%PDF-1.4 teste").unwrap();

    // Sessão A abre -> 200.
    let cookie_a = format!("gedocs_sid={sid_a}");
    let (status_a, _c, _b) = call(
        &r,
        "GET",
        "/api/documento/1998547/x.pdf",
        None,
        Some(&cookie_a),
    )
    .await;
    assert_eq!(status_a, StatusCode::OK);

    // Sessão B (outro sid) NÃO acessa o arquivo de A -> 404.
    let (status_b, _c, _b) = call(
        &r,
        "GET",
        "/api/documento/1998547/x.pdf",
        None,
        Some("gedocs_sid=outrasessaobbbb"),
    )
    .await;
    assert_eq!(status_b, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn ttl_remove_sessao_inativa_sc003() {
    let tmp = tempfile::tempdir().unwrap();
    let mut st = estado(tmp.path(), 1000);
    st.session_ttl = Duration::from_secs(1);

    // Cria uma sessão "velha" (last no passado).
    let dir = st.sessions_root().join("velha");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(".last"), "1").unwrap();

    let removidas = gedocs_server::ttl::limpar_uma_vez(&st);
    assert_eq!(removidas, 1);
    assert!(!dir.exists(), "sessão inativa deve ser removida");
}

#[tokio::test]
async fn rate_limit_429_ao_exceder() {
    let tmp = tempfile::tempdir().unwrap();
    let r = app(estado(tmp.path(), 2)); // limite de 2/min

    let (s1, _c, _b) = call(&r, "GET", "/api/health", None, None).await;
    let (s2, _c, _b) = call(&r, "GET", "/api/health", None, None).await;
    let (s3, _c, _b) = call(&r, "GET", "/api/health", None, None).await;
    assert_eq!(s1, StatusCode::OK);
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(s3, StatusCode::TOO_MANY_REQUESTS);
}
