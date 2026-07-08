//! Sessão efêmera sem login (US2, FR-010/011/012). Cada visitante recebe um
//! cookie opaco `gedocs_sid`; o servidor mapeia para
//! `<data>/sessions/<sid>/` e isola ali todos os PDFs (PII de terceiros).
//! A cada request, atualiza `.last` (para o TTL) — ver `ttl.rs`.

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::{Request, State},
    http::{header, HeaderValue},
    middleware::Next,
    response::Response,
};

use crate::AppState;

pub const COOKIE: &str = "gedocs_sid";

/// Contexto da sessão, injetado nas extensões do request e extraído pelos
/// handlers via `Extension<SessionCtx>`.
#[derive(Clone)]
pub struct SessionCtx {
    pub sid: String,
    /// `<data>/sessions/<sid>/`
    pub dir: PathBuf,
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `sid` válido = só `[A-Za-z0-9-]` (bloqueia `..`/`/`, evita path traversal).
fn sid_valido(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn extrair_sid(req: &Request) -> Option<String> {
    let raw = req.headers().get(header::COOKIE)?.to_str().ok()?;
    for parte in raw.split(';') {
        let parte = parte.trim();
        if let Some(v) = parte.strip_prefix(&format!("{COOKIE}=")) {
            if sid_valido(v) {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn novo_sid() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// Middleware: resolve/gera a sessão, cria o diretório, marca atividade e
/// injeta `SessionCtx`; ao final, (re)emite o cookie `gedocs_sid`.
pub async fn middleware(State(st): State<AppState>, mut req: Request, next: Next) -> Response {
    let sid = extrair_sid(&req).unwrap_or_else(novo_sid);
    let dir = st.sessions_root().join(&sid);
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(".last"), now_secs().to_string());

    req.extensions_mut().insert(SessionCtx {
        sid: sid.clone(),
        dir,
    });

    let mut resp = next.run(req).await;

    let ttl = st.session_ttl.as_secs();
    // Em produção o front (Vercel) e a API (Render/Fly) ficam em domínios
    // diferentes: o cookie só volta no `fetch` cross-site se for
    // `SameSite=None; Secure`. Em dev (same-site localhost) usa `Lax` sem
    // `Secure`. `secure_cookie` é ligado nos deploys (fly.toml/render.yaml).
    let atributos = if st.secure_cookie {
        "SameSite=None; Secure"
    } else {
        "SameSite=Lax"
    };
    let cookie = format!("{COOKIE}={sid}; Path=/; HttpOnly; {atributos}; Max-Age={ttl}");
    if let Ok(v) = HeaderValue::from_str(&cookie) {
        resp.headers_mut().append(header::SET_COOKIE, v);
    }
    resp
}
