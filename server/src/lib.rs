//! API HTTP (axum) da versão web do GeDoc IFES Toolkit.
//!
//! Fronteira HTTP fina sobre o núcleo Rust do app (`gedocs_lib`): reusa os
//! use-cases puros e adapta o comportamento ao navegador (sessão efêmera sem
//! login + TTL, isolamento de PII, CORS, rate limit). Ver
//! `specs/003-versao-web/plan.md`. `app()` é exposto para os testes de
//! integração.

pub mod erro;
pub mod rotas;
pub mod sessao;
pub mod ttl;

use std::{
    collections::HashMap,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, Request, State},
    http::{header, HeaderValue, Method},
    middleware::{from_fn_with_state, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;

use crate::sessao::now_secs;

/// Estado compartilhado da API.
#[derive(Clone)]
pub struct AppState {
    /// Raiz efêmera dos dados (sessões + categoria.json global).
    pub data_dir: PathBuf,
    /// Semente de categorias (`config/categoria.json`).
    pub seed_categorias: PathBuf,
    /// TTL de inatividade da sessão.
    pub session_ttl: Duration,
    /// `Secure` no cookie (produção/HTTPS).
    pub secure_cookie: bool,
    /// Contador simples de rate limit por IP (janela fixa de 60s).
    pub rate: Arc<Mutex<HashMap<String, (u64, u32)>>>,
    /// Máximo de requisições por IP por minuto.
    pub rate_limit: u32,
}

impl AppState {
    pub fn sessions_root(&self) -> PathBuf {
        self.data_dir.join("sessions")
    }
    pub fn categorias_path(&self) -> PathBuf {
        self.data_dir.join("categoria.json")
    }
}

pub fn env_ou(chave: &str, padrao: &str) -> String {
    std::env::var(chave).unwrap_or_else(|_| padrao.to_string())
}

/// Rate limit por IP (janela fixa de 60s). 429 amigável ao exceder (FR-016).
async fn rate_limit_mw(
    State(st): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let ip = addr.ip().to_string();
    let janela = now_secs() / 60;
    let excedeu = {
        let mut m = st.rate.lock().unwrap();
        let e = m.entry(ip).or_insert((janela, 0));
        if e.0 != janela {
            *e = (janela, 0);
        }
        e.1 += 1;
        e.1 > st.rate_limit
    };
    if excedeu {
        return rotas::resposta_rate_limit();
    }
    next.run(req).await
}

/// Monta o router (usado por `run()` e pelos testes de integração).
pub fn app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true)
        .allow_origin(
            env_ou("GEDOCS_CORS_ORIGIN", "http://localhost:5173")
                .parse::<HeaderValue>()
                .expect("GEDOCS_CORS_ORIGIN inválido"),
        );

    // Rotas que exigem sessão (tudo menos /api/health).
    let protegidas = Router::new()
        .route("/api/buscar", post(rotas::buscar))
        .route("/api/documento/baixar", post(rotas::baixar_documento))
        .route(
            "/api/documento/:siape/:arquivo",
            get(rotas::abrir_documento),
        )
        .route(
            "/api/categorias",
            get(rotas::listar_categorias).put(rotas::salvar_categorias),
        )
        .route("/api/relatorio", post(rotas::gerar_relatorio))
        .route("/api/relatorio/:siape", get(rotas::servir_relatorio))
        .route("/api/zip/:siape", get(rotas::baixar_zip))
        .route_layer(from_fn_with_state(state.clone(), sessao::middleware));

    Router::new()
        .route("/api/health", get(rotas::health))
        .merge(protegidas)
        .layer(from_fn_with_state(state.clone(), rate_limit_mw))
        .layer(cors)
        .layer(DefaultBodyLimit::max(8 * 1024 * 1024))
        .with_state(state)
}

/// Lê a config do ambiente e monta o `AppState`.
pub fn state_do_ambiente() -> AppState {
    let data_dir = PathBuf::from(env_ou("GEDOCS_DATA_DIR", "./data-web"));
    let seed_categorias = std::env::var("GEDOCS_CATEGORIAS_SEED")
        .map(PathBuf::from)
        .unwrap_or_else(|_| gedocs_lib::services::categorias::caminho_padrao());
    let session_ttl =
        Duration::from_secs(env_ou("GEDOCS_SESSION_TTL", "3600").parse().unwrap_or(3600));
    let secure_cookie = env_ou("GEDOCS_SECURE_COOKIE", "false") == "true";
    let rate_limit = env_ou("GEDOCS_RATE_LIMIT", "120").parse().unwrap_or(120);
    let _ = std::fs::create_dir_all(data_dir.join("sessions"));

    AppState {
        data_dir,
        seed_categorias,
        session_ttl,
        secure_cookie,
        rate: Arc::new(Mutex::new(HashMap::new())),
        rate_limit,
    }
}

/// Sobe o servidor (chamado por `main`).
pub async fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gedocs_server=info,tower_http=warn".into()),
        )
        .init();

    let bind = env_ou("GEDOCS_BIND", "0.0.0.0:8787");
    let state = state_do_ambiente();
    ttl::spawn_cleanup(state.clone());

    let app = app(state);
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .expect("falha ao bindar");
    tracing::info!("gedocs-server ouvindo em http://{bind}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("falha ao servir");
}
