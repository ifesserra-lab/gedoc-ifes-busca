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

/// Nº total de tentativas por requisição (1ª + retentativas). Espelha o
/// `Retry(total=3)` do cliente Python de referência: 4 tentativas no total.
const MAX_TENTATIVAS: u32 = 4;
/// Fator de backoff (ms). Espelha `backoff_factor=0.5` do urllib3: a espera
/// antes da tentativa nº k é `BASE * 2^(k-1)` → 0,5s, 1s, 2s.
const BACKOFF_BASE_MS: u64 = 500;

pub trait HttpPort {
    /// GET simples; usado para abrir a sessão e descobrir os IDs do portal.
    fn get(&self, url: &str) -> Result<String, AppError>;

    /// POST `application/x-www-form-urlencoded` com os headers de requisição
    /// parcial (AJAX) do JSF/PrimeFaces.
    fn post_form(&self, url: &str, campos: &[(String, String)]) -> Result<String, AppError>;
}

/// Adapter concreto sobre `reqwest::blocking`. Fronteira de I/O — a rede real
/// não é exercitada em teste (Princípio VII); a orquestração que o usa
/// (`GedocRepositoryHttp`) é testada com um dublê. As decisões puras (quando
/// retentar, quanto esperar, como montar a requisição) são extraídas em
/// funções livres testáveis abaixo.
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

    /// Executa uma requisição com retry+backoff contra rate limit (429) e
    /// erros transitórios do servidor (5xx) e de conexão/timeout — exigência
    /// das Restrições Técnicas da constituição, espelhando o `Retry` do
    /// cliente Python. `montar` reconstrói a requisição a cada tentativa
    /// (um `RequestBuilder` é consumido no `send`).
    fn com_retry(
        &self,
        contexto: &str,
        montar: impl Fn() -> reqwest::blocking::RequestBuilder,
    ) -> Result<String, AppError> {
        let mut tentativa = 1;
        loop {
            let pode_retentar = tentativa < MAX_TENTATIVAS;
            match montar().send() {
                Ok(resp) if deve_retentar_status(resp.status().as_u16()) && pode_retentar => {
                    std::thread::sleep(backoff(tentativa));
                }
                Err(e) if erro_transitorio(&e) && pode_retentar => {
                    std::thread::sleep(backoff(tentativa));
                }
                resultado => return ler_corpo(resultado, contexto),
            }
            tentativa += 1;
        }
    }
}

impl HttpPort for ReqwestHttp {
    fn get(&self, url: &str) -> Result<String, AppError> {
        self.com_retry(&format!("GET {url}"), || self.client.get(url))
    }

    fn post_form(&self, url: &str, campos: &[(String, String)]) -> Result<String, AppError> {
        self.com_retry(&format!("POST {url}"), || {
            montar_post(&self.client, url, campos)
        })
    }
}

/// Monta o `POST` parcial do JSF/PrimeFaces. O `Content-Type` com
/// `charset=UTF-8` é definido **antes** de `.form()` de propósito: `form()`
/// só insere o header se ausente (`or_insert`), então definí-lo depois criaria
/// um segundo header `Content-Type` (requisição malformada). Função livre para
/// ser verificável via `RequestBuilder::build()` sem tocar a rede.
fn montar_post(
    client: &reqwest::blocking::Client,
    url: &str,
    campos: &[(String, String)],
) -> reqwest::blocking::RequestBuilder {
    client
        .post(url)
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded; charset=UTF-8",
        )
        .header("Faces-Request", "partial/ajax")
        .header("X-Requested-With", "XMLHttpRequest")
        .form(campos)
}

/// Status HTTP que justificam retentativa: rate limit e erros transitórios do
/// servidor (mesma lista do `status_forcelist` do Python).
fn deve_retentar_status(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

/// Erro de rede transitório (conexão ou timeout) — vale retentar; erros de
/// TLS/decodificação/etc. não.
fn erro_transitorio(e: &reqwest::Error) -> bool {
    e.is_timeout() || e.is_connect()
}

/// Espera antes da tentativa nº `tentativa` (1-based): `BASE * 2^(tentativa-1)`.
fn backoff(tentativa: u32) -> Duration {
    Duration::from_millis(BACKOFF_BASE_MS * 2u64.pow(tentativa.saturating_sub(1)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deve_retentar_apenas_status_transitorios() {
        for s in [429, 500, 502, 503, 504] {
            assert!(deve_retentar_status(s), "{s} deveria retentar");
        }
        for s in [200, 301, 400, 401, 403, 404] {
            assert!(!deve_retentar_status(s), "{s} não deveria retentar");
        }
    }

    #[test]
    fn backoff_cresce_exponencialmente() {
        assert_eq!(backoff(1), Duration::from_millis(500));
        assert_eq!(backoff(2), Duration::from_millis(1000));
        assert_eq!(backoff(3), Duration::from_millis(2000));
    }

    #[test]
    fn post_envia_um_unico_content_type_com_charset() {
        // Não toca a rede: só constrói a requisição e inspeciona os headers.
        let client = reqwest::blocking::Client::new();
        let campos = vec![("a".to_string(), "1".to_string())];
        let req = montar_post(&client, "https://exemplo/x", &campos)
            .build()
            .expect("deve montar a requisição");

        let content_types: Vec<_> = req
            .headers()
            .get_all(reqwest::header::CONTENT_TYPE)
            .iter()
            .collect();
        assert_eq!(content_types.len(), 1, "sem Content-Type duplicado");
        assert_eq!(
            content_types[0],
            "application/x-www-form-urlencoded; charset=UTF-8"
        );
    }
}
