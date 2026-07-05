//! Port/Adapter (GoF) — fronteira de I/O do serviço de IA (Mistral) usado
//! pela classificação (US5) e, futuramente, pelo resumo (US6). Espelha
//! `src/mistral_client.py`: mesma URL/modelo padrão, mesmo retry em 429/5xx
//! com backoff exponencial (R9) e a mesma leitura de chave — variável de
//! ambiente primeiro, depois um `.env` simples (`config/.env`/`.env`; nunca
//! hard-coded, nunca logada — Princípio II/LGPD).
//!
//! Fronteira de rede: sem teste de rede real (Princípio VII). A construção
//! do corpo da requisição, o parse da resposta e a leitura/validação da
//! chave são extraídos em funções puras testáveis abaixo.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::error::AppError;

const API_URL: &str = "https://api.mistral.ai/v1/chat/completions";
const MODELO_PADRAO: &str = "mistral-small-latest";
const TIMEOUT: Duration = Duration::from_secs(60);
/// Nº total de tentativas por requisição (espelha `RETRIES = 4` do Python).
const RETRIES: u32 = 4;
/// Intervalo mínimo entre chamadas (espelha `THROTTLE = 1.2` do Python).
const THROTTLE_PADRAO: Duration = Duration::from_millis(1200);
/// Classificação: resposta curta e determinística — exatamente uma categoria
/// em JSON (R4). Fixo neste port porque, por ora, ele só serve classificação;
/// se US6 (resumo) precisar de outro formato/limite, o trait pode crescer um
/// parâmetro sem afetar quem já o usa.
const MAX_TOKENS: u32 = 40;
const TEMPERATURE: f64 = 0.0;

/// Port para um serviço de chat de IA. Modo JSON fixo (ver `montar_corpo`) —
/// pensado para classificação; mantido simples de propósito.
pub trait ChatIa {
    fn chat(&self, sistema: &str, usuario: &str) -> Result<String, AppError>;
}

/// Adapter concreto sobre a API de chat completions da Mistral
/// (`reqwest::blocking`, mesma justificativa de `ports::http::ReqwestHttp`:
/// roda dentro de `tokio::task::spawn_blocking`). Aplica throttle (R9) antes
/// de cada chamada e retry/backoff em erros transitórios (429/5xx).
pub struct MistralClient {
    client: reqwest::blocking::Client,
    api_key: String,
    modelo: String,
    throttle: Duration,
    ultima_chamada: Mutex<Option<Instant>>,
}

impl MistralClient {
    pub fn novo(api_key: impl Into<String>) -> Result<Self, AppError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(TIMEOUT)
            .use_rustls_tls()
            .build()
            .map_err(|e| AppError::FalhaIA {
                motivo: format!("Falha ao iniciar o cliente de IA: {e}"),
            })?;
        Ok(Self {
            client,
            api_key: api_key.into(),
            modelo: MODELO_PADRAO.to_string(),
            throttle: THROTTLE_PADRAO,
            ultima_chamada: Mutex::new(None),
        })
    }

    /// Aguarda o necessário para respeitar o throttle desde a última
    /// chamada (R9 — evita 429 de rate limit). Nunca panica por um `Mutex`
    /// envenenado por um pânico anterior em outra thread.
    fn respeitar_throttle(&self) {
        let mut guarda = self
            .ultima_chamada
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(anterior) = *guarda {
            let decorrido = anterior.elapsed();
            if decorrido < self.throttle {
                std::thread::sleep(self.throttle - decorrido);
            }
        }
        *guarda = Some(Instant::now());
    }
}

impl ChatIa for MistralClient {
    fn chat(&self, sistema: &str, usuario: &str) -> Result<String, AppError> {
        self.respeitar_throttle();

        let corpo = montar_corpo(&self.modelo, sistema, usuario);
        let corpo_bytes = serde_json::to_vec(&corpo).map_err(|e| AppError::FalhaIA {
            motivo: format!("Falha ao montar a requisição: {e}"),
        })?;

        let mut tentativa = 1;
        loop {
            let pode_retentar = tentativa < RETRIES;
            let requisicao = self
                .client
                .post(API_URL)
                .bearer_auth(&self.api_key)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(corpo_bytes.clone());

            match requisicao.send() {
                Ok(resp) if resp.status().as_u16() == 200 => {
                    let texto = resp.text().map_err(erro_rede)?;
                    return parsear_resposta_chat(&texto);
                }
                Ok(resp) if deve_retentar_status(resp.status().as_u16()) && pode_retentar => {
                    std::thread::sleep(backoff(tentativa));
                }
                Ok(resp) => return Err(erro_http(resp)),
                Err(e) if erro_transitorio(&e) && pode_retentar => {
                    std::thread::sleep(backoff(tentativa));
                }
                Err(e) => return Err(erro_rede(e)),
            }
            tentativa += 1;
        }
    }
}

fn erro_rede(e: reqwest::Error) -> AppError {
    AppError::FalhaIA {
        motivo: format!("Falha ao comunicar com o serviço de IA: {e}"),
    }
}

fn erro_http(resp: reqwest::blocking::Response) -> AppError {
    let status = resp.status();
    let corpo = resp.text().unwrap_or_default();
    AppError::FalhaIA {
        motivo: format!("HTTP {status} da API Mistral: {corpo}"),
    }
}

fn erro_transitorio(e: &reqwest::Error) -> bool {
    e.is_timeout() || e.is_connect()
}

/// Status HTTP que justificam retentativa (mesma lista de `ports::http`).
fn deve_retentar_status(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504)
}

/// Espera antes da tentativa nº `tentativa` (1-based): `2^tentativa` segundos
/// — espelha `espera = 2 ** tentativa` do cliente Python de referência.
fn backoff(tentativa: u32) -> Duration {
    Duration::from_secs(2u64.pow(tentativa))
}

/// Monta o corpo JSON do chat completion — modo JSON fixo, `temperature=0`,
/// `max_tokens=40` (classificação: resposta curta e determinística, R4).
/// Função pura, testável sem rede.
fn montar_corpo(modelo: &str, sistema: &str, usuario: &str) -> Value {
    serde_json::json!({
        "model": modelo,
        "temperature": TEMPERATURE,
        "max_tokens": MAX_TOKENS,
        "response_format": { "type": "json_object" },
        "messages": [
            { "role": "system", "content": sistema },
            { "role": "user", "content": usuario },
        ],
    })
}

/// Extrai `choices[0].message.content` (já sem espaços nas bordas) do corpo
/// de resposta bruto. Nunca panica: JSON inválido ou campo ausente vira
/// `AppError::FalhaIA` (R11 — quem chama decide o que fazer com a falha).
fn parsear_resposta_chat(corpo: &str) -> Result<String, AppError> {
    let json: Value = serde_json::from_str(corpo).map_err(|e| AppError::FalhaIA {
        motivo: format!("Resposta da IA não é um JSON válido: {e}"),
    })?;
    json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.trim().to_string())
        .ok_or_else(|| AppError::FalhaIA {
            motivo: "Resposta da IA sem conteúdo (choices[0].message.content ausente)".to_string(),
        })
}

// --- resolução da chave: env var ou `config/.env` -------------------------- //

/// Retorna a chave da Mistral: variáveis de ambiente têm prioridade
/// (`MISTRAL_API_KEY` ou `MISTRAL_KEY`); senão tenta ler de um `.env` simples
/// em `config/.env`/`.env` (candidatos relativos — o app pode rodar com cwd
/// na raiz do repositório ou em `src-tauri/`, conforme como foi iniciado).
/// Nunca loga o valor da chave.
pub fn resolver_api_key() -> Option<String> {
    chave_do_ambiente().or_else(|| {
        candidatos_env()
            .into_iter()
            .find_map(|caminho| fs::read_to_string(caminho).ok())
            .and_then(|conteudo| extrair_key(&parsear_env(&conteudo)))
    })
}

fn chave_do_ambiente() -> Option<String> {
    std::env::var("MISTRAL_API_KEY")
        .ok()
        .or_else(|| std::env::var("MISTRAL_KEY").ok())
        .filter(|s| !s.trim().is_empty())
}

/// Caminhos candidatos para o `.env`, na ordem em que são tentados — espelha
/// `_envs_candidatos` do cliente Python de referência.
fn candidatos_env() -> Vec<PathBuf> {
    vec![
        PathBuf::from("config/.env"),
        PathBuf::from(".env"),
        PathBuf::from("../config/.env"),
        PathBuf::from("../.env"),
    ]
}

/// Parseia um `.env` simples (`CHAVE=valor`; comentários com `#`; aspas
/// simples/duplas nas bordas do valor são removidas). Função pura — mesma
/// sintaxe de `mistral_client.carregar_env`.
fn parsear_env(conteudo: &str) -> HashMap<String, String> {
    conteudo
        .lines()
        .map(str::trim)
        .filter(|linha| !linha.is_empty() && !linha.starts_with('#'))
        .filter_map(|linha| linha.split_once('='))
        .map(|(chave, valor)| (chave.trim().to_string(), limpar_aspas(valor.trim())))
        .collect()
}

fn limpar_aspas(valor: &str) -> String {
    let sem_aspas = valor
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| valor.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')));
    sem_aspas.unwrap_or(valor).to_string()
}

/// Extrai a chave de um mapa já parseado (`MISTRAL_API_KEY` tem prioridade
/// sobre `MISTRAL_KEY`, mesma ordem da variável de ambiente).
fn extrair_key(mapa: &HashMap<String, String>) -> Option<String> {
    mapa.get("MISTRAL_API_KEY")
        .or_else(|| mapa.get("MISTRAL_KEY"))
        .filter(|s| !s.trim().is_empty())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- montar_corpo / parsear_resposta_chat ------------------------------ //

    #[test]
    fn montar_corpo_usa_modo_json_fixo_e_parametros_de_classificacao() {
        let corpo = montar_corpo("mistral-small-latest", "sistema", "usuario");
        assert_eq!(corpo["model"], "mistral-small-latest");
        assert_eq!(corpo["temperature"], 0.0);
        assert_eq!(corpo["max_tokens"], 40);
        assert_eq!(corpo["response_format"]["type"], "json_object");
        assert_eq!(corpo["messages"][0]["role"], "system");
        assert_eq!(corpo["messages"][0]["content"], "sistema");
        assert_eq!(corpo["messages"][1]["role"], "user");
        assert_eq!(corpo["messages"][1]["content"], "usuario");
    }

    #[test]
    fn parsear_resposta_chat_extrai_e_apara_o_conteudo() {
        let corpo = r#"{"choices":[{"message":{"content":"  {\"categoria\":\"Progressão\"}  "}}]}"#;
        let conteudo = parsear_resposta_chat(corpo).expect("deve extrair o conteúdo");
        assert_eq!(conteudo, r#"{"categoria":"Progressão"}"#);
    }

    #[test]
    fn parsear_resposta_chat_falha_com_json_invalido() {
        let erro = parsear_resposta_chat("isto não é JSON").unwrap_err();
        assert!(matches!(erro, AppError::FalhaIA { .. }));
    }

    #[test]
    fn parsear_resposta_chat_falha_quando_content_esta_ausente() {
        let erro = parsear_resposta_chat(r#"{"choices":[{"message":{}}]}"#).unwrap_err();
        assert!(matches!(erro, AppError::FalhaIA { .. }));
    }

    // --- retry/backoff ------------------------------------------------------ //

    #[test]
    fn deve_retentar_apenas_status_transitorios() {
        for status in [429, 500, 502, 503, 504] {
            assert!(deve_retentar_status(status), "{status} deveria retentar");
        }
        for status in [200, 301, 400, 401, 403, 404] {
            assert!(
                !deve_retentar_status(status),
                "{status} não deveria retentar"
            );
        }
    }

    #[test]
    fn backoff_cresce_exponencialmente_em_segundos() {
        assert_eq!(backoff(1), Duration::from_secs(2));
        assert_eq!(backoff(2), Duration::from_secs(4));
        assert_eq!(backoff(3), Duration::from_secs(8));
    }

    // --- .env / resolução de chave ------------------------------------------ //

    #[test]
    fn parsear_env_ignora_comentarios_e_linhas_vazias() {
        let conteudo = "# comentário\n\nMISTRAL_KEY=abc123\n";
        let mapa = parsear_env(conteudo);
        assert_eq!(mapa.get("MISTRAL_KEY"), Some(&"abc123".to_string()));
        assert_eq!(mapa.len(), 1);
    }

    #[test]
    fn parsear_env_remove_aspas_simples_e_duplas_das_bordas() {
        let mapa = parsear_env("A=\"valor1\"\nB='valor2'\nC=valor3\n");
        assert_eq!(mapa.get("A"), Some(&"valor1".to_string()));
        assert_eq!(mapa.get("B"), Some(&"valor2".to_string()));
        assert_eq!(mapa.get("C"), Some(&"valor3".to_string()));
    }

    #[test]
    fn parsear_env_ignora_linha_sem_igual() {
        let mapa = parsear_env("isto nao tem igual\nMISTRAL_KEY=abc\n");
        assert_eq!(mapa.len(), 1);
    }

    #[test]
    fn extrair_key_prioriza_mistral_api_key_sobre_mistral_key() {
        let mut mapa = HashMap::new();
        mapa.insert("MISTRAL_API_KEY".to_string(), "chave-nova".to_string());
        mapa.insert("MISTRAL_KEY".to_string(), "chave-antiga".to_string());
        assert_eq!(extrair_key(&mapa), Some("chave-nova".to_string()));
    }

    #[test]
    fn extrair_key_aceita_mistral_key_quando_api_key_ausente() {
        let mut mapa = HashMap::new();
        mapa.insert("MISTRAL_KEY".to_string(), "chave".to_string());
        assert_eq!(extrair_key(&mapa), Some("chave".to_string()));
    }

    #[test]
    fn extrair_key_retorna_none_quando_ausente_ou_vazia() {
        assert_eq!(extrair_key(&HashMap::new()), None);

        let mut mapa = HashMap::new();
        mapa.insert("MISTRAL_KEY".to_string(), "   ".to_string());
        assert_eq!(extrair_key(&mapa), None, "chave só com espaços não conta");
    }

    #[test]
    fn candidatos_env_tenta_config_env_e_env_da_raiz_antes_de_subir_um_nivel() {
        let candidatos = candidatos_env();
        assert_eq!(
            candidatos,
            vec![
                PathBuf::from("config/.env"),
                PathBuf::from(".env"),
                PathBuf::from("../config/.env"),
                PathBuf::from("../.env"),
            ]
        );
    }
}
