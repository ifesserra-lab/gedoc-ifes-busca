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
/// Classificação (US5, R4): resposta curta e determinística — exatamente uma
/// categoria em JSON (ver `montar_corpo`, `modo_json = true`).
const MAX_TOKENS: u32 = 40;
const TEMPERATURE: f64 = 0.0;
/// Resumo (US6): texto livre e mais longo — sem `response_format` fixo
/// (espelha `resumir(..., max_tokens=300, temperature=0.2)` de
/// `src/resumir_mistral.py`).
const MAX_TOKENS_RESUMO: u32 = 300;
const TEMPERATURE_RESUMO: f64 = 0.2;
/// Classificação/resumo em lote (specs 010/011): várias respostas por
/// chamada exigem um orçamento de tokens bem maior que o de 1 item.
const MAX_TOKENS_LOTE: u32 = 2000;

/// Port para um serviço de chat de IA. `chat` é usado pela classificação
/// (US5): resposta curta, determinística, sempre JSON. `resumir` (US6) é uma
/// variante de texto livre; tem um default que delega para `chat` — mantém
/// compatibilidade com os dublês de teste já existentes (que só implementam
/// `chat`) sem exigir que sejam alterados. `MistralClient` sobrescreve
/// `resumir` com os parâmetros corretos (mais tokens, temperatura maior, sem
/// forçar JSON).
///
/// `chat_lote` (specs 010/011) envia vários itens numa única chamada —
/// resposta sempre em JSON (`{"itens":[...]}`), `temperatura` escolhida pelo
/// chamador (0.0 para classificação, 0.2 para resumo, mesmas temperaturas do
/// caminho por-item). Tem um default que delega para `chat` — mesma
/// justificativa de `resumir`: dublês de teste existentes continuam
/// compilando sem alteração; quem precisar do comportamento de lote de fato
/// sobrescreve (só `MistralClient` o faz em produção).
pub trait ChatIa {
    fn chat(&self, sistema: &str, usuario: &str) -> Result<String, AppError>;

    fn resumir(&self, sistema: &str, usuario: &str) -> Result<String, AppError> {
        self.chat(sistema, usuario)
    }

    fn chat_lote(
        &self,
        sistema: &str,
        usuario: &str,
        _temperatura: f64,
    ) -> Result<String, AppError> {
        self.chat(sistema, usuario)
    }
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
        let corpo = montar_corpo(
            &self.modelo,
            sistema,
            usuario,
            MAX_TOKENS,
            TEMPERATURE,
            true,
        );
        self.enviar(corpo)
    }

    /// US6 — resumo: texto livre (sem `response_format` fixo), mais tokens e
    /// temperatura levemente maior que a classificação. Mesmo throttle/retry
    /// (R9) do `chat`, via `enviar`.
    fn resumir(&self, sistema: &str, usuario: &str) -> Result<String, AppError> {
        let corpo = montar_corpo(
            &self.modelo,
            sistema,
            usuario,
            MAX_TOKENS_RESUMO,
            TEMPERATURE_RESUMO,
            false,
        );
        self.enviar(corpo)
    }

    /// Specs 010/011 — lote: mais tokens (`MAX_TOKENS_LOTE`, vários itens por
    /// resposta), `temperatura` escolhida pelo chamador (0.0 classificação,
    /// 0.2 resumo) e resposta sempre em JSON (`modo_json = true`) — tanto a
    /// classificação quanto o resumo em lote pedem `{"itens":[...]}`. Mesmo
    /// throttle/retry (R9) do `chat`/`resumir`, via `enviar`.
    fn chat_lote(
        &self,
        sistema: &str,
        usuario: &str,
        temperatura: f64,
    ) -> Result<String, AppError> {
        let corpo = montar_corpo(
            &self.modelo,
            sistema,
            usuario,
            MAX_TOKENS_LOTE,
            temperatura,
            true,
        );
        self.enviar(corpo)
    }
}

impl MistralClient {
    /// Aplica o throttle (R9), serializa o corpo e executa o retry/backoff
    /// em erros transitórios (429/5xx) — compartilhado por `chat` e
    /// `resumir`; só o corpo da requisição muda entre os dois usos.
    fn enviar(&self, corpo: Value) -> Result<String, AppError> {
        self.respeitar_throttle();

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

/// Monta o corpo JSON do chat completion. `modo_json` força
/// `response_format: json_object` (classificação — R4: resposta curta,
/// determinística, parseável); o resumo (US6) passa `false` — texto livre.
/// Função pura, testável sem rede.
fn montar_corpo(
    modelo: &str,
    sistema: &str,
    usuario: &str,
    max_tokens: u32,
    temperature: f64,
    modo_json: bool,
) -> Value {
    let mut corpo = serde_json::json!({
        "model": modelo,
        "temperature": temperature,
        "max_tokens": max_tokens,
        "messages": [
            { "role": "system", "content": sistema },
            { "role": "user", "content": usuario },
        ],
    });
    if modo_json {
        corpo["response_format"] = serde_json::json!({ "type": "json_object" });
    }
    corpo
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
        .and_then(|s| normalizar_chave(&s))
}

/// Normaliza uma chave crua vinda de uma variável de ambiente: apara espaços
/// das bordas e remove um par de aspas simples/duplas acidental. Vazia → None.
/// Espelha o tratamento que o caminho do `.env` já faz (`limpar_aspas` +
/// `trim` em `parsear_env`): colar a chave no painel do Render com aspas ou um
/// espaço/quebra-de-linha na ponta era enviado verbatim à Mistral e recusado
/// com 401. Agora os dois caminhos limpam a chave da mesma forma.
fn normalizar_chave(bruta: &str) -> Option<String> {
    let limpa = limpar_aspas(bruta.trim());
    let limpa = limpa.trim();
    (!limpa.is_empty()).then(|| limpa.to_string())
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
        let corpo = montar_corpo(
            "mistral-small-latest",
            "sistema",
            "usuario",
            MAX_TOKENS,
            TEMPERATURE,
            true,
        );
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
    fn montar_corpo_do_resumo_usa_texto_livre_sem_forcar_json() {
        let corpo = montar_corpo(
            "mistral-small-latest",
            "sistema",
            "usuario",
            MAX_TOKENS_RESUMO,
            TEMPERATURE_RESUMO,
            false,
        );
        assert_eq!(corpo["temperature"], 0.2);
        assert_eq!(corpo["max_tokens"], 300);
        assert!(
            corpo.get("response_format").is_none(),
            "resumo não deve forçar response_format"
        );
    }

    #[test]
    fn resumir_tem_default_que_delega_para_chat_compatibilidade_com_dubles_existentes() {
        struct FakeSoChat;
        impl ChatIa for FakeSoChat {
            fn chat(&self, _sistema: &str, _usuario: &str) -> Result<String, AppError> {
                Ok("via chat".to_string())
            }
        }
        assert_eq!(FakeSoChat.resumir("s", "u").unwrap(), "via chat");
    }

    #[test]
    fn chat_lote_tem_default_que_delega_para_chat_compatibilidade_com_dubles_existentes() {
        struct FakeSoChat;
        impl ChatIa for FakeSoChat {
            fn chat(&self, _sistema: &str, _usuario: &str) -> Result<String, AppError> {
                Ok("via chat".to_string())
            }
        }
        assert_eq!(FakeSoChat.chat_lote("s", "u", 0.0).unwrap(), "via chat");
    }

    #[test]
    fn montar_corpo_do_lote_usa_modo_json_e_max_tokens_maior() {
        let corpo = montar_corpo(
            "mistral-small-latest",
            "sistema",
            "usuario",
            MAX_TOKENS_LOTE,
            0.0,
            true,
        );
        assert_eq!(corpo["max_tokens"], 2000);
        assert_eq!(corpo["response_format"]["type"], "json_object");
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
    fn normalizar_chave_apara_espacos_e_remove_aspas_acidentais() {
        // Gotcha do painel do Render: chave colada com aspas/espaço/quebra de
        // linha era enviada verbatim e recusada (401). Agora é limpa.
        assert_eq!(normalizar_chave("  abc123  "), Some("abc123".to_string()));
        assert_eq!(normalizar_chave("\"abc123\""), Some("abc123".to_string()));
        assert_eq!(normalizar_chave("'abc123'"), Some("abc123".to_string()));
        assert_eq!(normalizar_chave("\"abc123\"\n"), Some("abc123".to_string()));
    }

    #[test]
    fn normalizar_chave_vazia_ou_so_espacos_ou_aspas_vira_none() {
        assert_eq!(normalizar_chave(""), None);
        assert_eq!(normalizar_chave("   "), None);
        assert_eq!(normalizar_chave("\"\""), None);
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
