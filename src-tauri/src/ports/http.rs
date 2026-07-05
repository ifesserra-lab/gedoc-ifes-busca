//! Port/Adapter (GoF) — fronteira de I/O de rede usada por
//! `services::gedoc_repository`. O trait `HttpPort` é síncrono de propósito:
//! a chamada de rede real roda dentro de `tokio::task::spawn_blocking` (ver
//! `commands::buscar::executar`), o que permite testar toda a orquestração
//! do repositório com um dublê síncrono, sem runtime async e sem rede
//! (Princípio VII).

use std::time::Duration;

use crate::error::AppError;

const USER_AGENT: &str = "Mozilla/5.0 (gedoc-busca-tauri)";
const TIMEOUT: Duration = Duration::from_secs(30);

pub trait HttpPort {
    /// GET simples; usado para abrir a sessão e descobrir os IDs do portal.
    fn get(&self, url: &str) -> Result<String, AppError>;

    /// POST `application/x-www-form-urlencoded` com os headers de requisição
    /// parcial (AJAX) do JSF/PrimeFaces.
    fn post_form(&self, url: &str, campos: &[(String, String)]) -> Result<String, AppError>;
}

/// Adapter concreto sobre `reqwest::blocking`. Fronteira de I/O — não tem
/// teste unitário (Princípio VII: a rede real não é exercitada em teste);
/// a orquestração que o usa (`GedocRepositoryHttp`) é testada com um dublê.
pub struct ReqwestHttp {
    client: reqwest::blocking::Client,
}

impl ReqwestHttp {
    /// Constrói o cliente HTTP: cookies de sessão habilitados (o portal usa
    /// `jsessionid`), TLS via `rustls` (evita depender do OpenSSL do
    /// sistema) e timeout de 30s por requisição.
    pub fn novo() -> Result<Self, AppError> {
        let client = reqwest::blocking::Client::builder()
            .cookie_store(true)
            .user_agent(USER_AGENT)
            .timeout(TIMEOUT)
            .use_rustls_tls()
            .build()
            .map_err(|e| AppError::FalhaPortal {
                motivo: format!("Falha ao iniciar cliente HTTP: {e}"),
            })?;
        Ok(Self { client })
    }
}

impl HttpPort for ReqwestHttp {
    fn get(&self, url: &str) -> Result<String, AppError> {
        let resp = self.client.get(url).send();
        ler_corpo(resp, &format!("GET {url}"))
    }

    fn post_form(&self, url: &str, campos: &[(String, String)]) -> Result<String, AppError> {
        let resp = self
            .client
            .post(url)
            .form(campos)
            .header("Faces-Request", "partial/ajax")
            .header("X-Requested-With", "XMLHttpRequest")
            .header(
                "Content-Type",
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .send();
        ler_corpo(resp, &format!("POST {url}"))
    }
}

/// Converte o resultado de uma requisição `reqwest` em texto, transformando
/// qualquer falha de rede/HTTP em `AppError::FalhaPortal` (nunca panica).
fn ler_corpo(
    resp: Result<reqwest::blocking::Response, reqwest::Error>,
    contexto: &str,
) -> Result<String, AppError> {
    let erro = |e: reqwest::Error| AppError::FalhaPortal {
        motivo: format!("{contexto}: {e}"),
    };
    resp.map_err(erro)?
        .error_for_status()
        .map_err(erro)?
        .text()
        .map_err(erro)
}
