//! Strategy (GoF) — política de classificação de um Documento em exatamente
//! uma Categoria (R4). Duas estratégias:
//! - `ClassificadorPalavraChave`: casamento por nome/descrição da categoria,
//!   sem custo de API, instantânea (default de `commands::buscar`).
//! - `ClassificadorLlm`: guiada pelas descrições de `config/categoria.json`
//!   via um `ports::ia::ChatIa` (US5). Uma falha de comunicação com a IA cai
//!   no `ClassificadorPalavraChave` para aquele documento (R11 — não derruba
//!   o lote); uma resposta fora da lista de categorias cai em `OUTROS` (R4).

use crate::domain::categoria::{Categoria, OUTROS};
use crate::domain::documento::Documento;
use crate::error::AppError;
use crate::ports::ia::ChatIa;

pub trait Classificador {
    /// Classifica `documento` em exatamente uma das `categorias`; se nenhuma
    /// se aplicar, MUST retornar `"Outros"` (R4).
    fn classificar(&self, documento: &Documento, categorias: &[Categoria]) -> String;
}

/// Estratégia simples e sem custo de API: primeira categoria cujo nome (ou
/// alguma palavra significativa da descrição) aparece no título/trecho do
/// documento — comparação sem acento e sem diferenciar maiúsculas/minúsculas
/// (`normalizar`), para não depender de o texto do portal vir sempre
/// acentuado.
pub struct ClassificadorPalavraChave;

impl Classificador for ClassificadorPalavraChave {
    fn classificar(&self, documento: &Documento, categorias: &[Categoria]) -> String {
        let texto = normalizar(&format!(
            "{} {}",
            documento.titulo,
            documento.trecho.as_deref().unwrap_or("")
        ));

        categorias
            .iter()
            .find(|c| combina(&texto, c))
            .map(|c| c.nome.clone())
            .unwrap_or_else(|| OUTROS.to_string())
    }
}

/// Verdadeiro se o nome da categoria aparece no texto, ou se alguma palavra
/// "significativa" (>= 4 letras, para não casar preposições/artigos comuns)
/// da descrição aparece — descrições vêm de `config/categoria.json`
/// (Princípio IV: critério configurável, não hard-coded).
fn combina(texto_normalizado: &str, categoria: &Categoria) -> bool {
    if texto_normalizado.contains(&normalizar(&categoria.nome)) {
        return true;
    }
    match &categoria.descricao {
        Some(descricao) => normalizar(descricao)
            .split(|c: char| !c.is_alphanumeric())
            .filter(|palavra| palavra.chars().count() >= 4)
            .any(|palavra| texto_normalizado.contains(palavra)),
        None => false,
    }
}

/// Minúsculas e sem acento (só os diacríticos do português), para casamento
/// robusto de palavras-chave — equivalente ao `_norm` do Python de
/// referência, sem depender de uma crate externa de normalização Unicode.
fn normalizar(texto: &str) -> String {
    texto.to_lowercase().chars().map(remover_acento).collect()
}

fn remover_acento(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        outro => outro,
    }
}

/// Estratégia via IA (Mistral, por trás de `ChatIa`): monta um prompt guiado
/// pelas descrições das categorias e pede exatamente uma em JSON (R4).
pub struct ClassificadorLlm<'a, C: ChatIa + ?Sized> {
    chat: &'a C,
}

impl<'a, C: ChatIa + ?Sized> ClassificadorLlm<'a, C> {
    pub fn novo(chat: &'a C) -> Self {
        Self { chat }
    }

    /// Classifica via IA **sem** aplicar o fallback de palavra-chave em caso
    /// de falha de comunicação — devolve o erro para o chamador decidir.
    /// Usado pela orquestração (`services::classificador::classificar_lote`)
    /// para só cachear (R6) resultados que vieram de fato da IA, permitindo
    /// nova tentativa depois de uma falha transitória.
    pub(crate) fn classificar_via_ia(
        &self,
        documento: &Documento,
        categorias: &[Categoria],
    ) -> Result<String, AppError> {
        let (sistema, usuario) = montar_prompt(documento, categorias);
        let resposta = self.chat.chat(&sistema, &usuario)?;
        Ok(extrair_categoria(&resposta, categorias))
    }
}

impl<'a, C: ChatIa + ?Sized> Classificador for ClassificadorLlm<'a, C> {
    fn classificar(&self, documento: &Documento, categorias: &[Categoria]) -> String {
        self.classificar_via_ia(documento, categorias)
            .unwrap_or_else(|_| ClassificadorPalavraChave.classificar(documento, categorias))
    }
}

/// Monta (sistema, usuário) guiado pelas descrições das categorias — a IA
/// deve responder só com `{"categoria": "<nome exato>"}`. Função pura,
/// testável sem rede. Trecho truncado em 1500 chars (mesmo limite do Python
/// de referência, para não estourar o orçamento de tokens do prompt).
fn montar_prompt(documento: &Documento, categorias: &[Categoria]) -> (String, String) {
    let definicoes = categorias
        .iter()
        .map(|c| format!("- {}: {}", c.nome, c.descricao.as_deref().unwrap_or("")))
        .collect::<Vec<_>>()
        .join("\n");

    let sistema = "Você classifica documentos administrativos do IFES em exatamente UMA \
categoria da lista fornecida. Baseie-se nas descrições. Responda apenas em \
JSON: {\"categoria\": \"<nome exato da categoria>\"}."
        .to_string();

    let trecho: String = documento
        .trecho
        .as_deref()
        .unwrap_or("")
        .chars()
        .take(1500)
        .collect();
    let usuario = format!(
        "Categorias disponíveis:\n{definicoes}\n\nDocumento:\nTítulo: {}\nTrecho: {trecho}",
        documento.titulo
    );

    (sistema, usuario)
}

/// Extrai `categoria` do JSON de resposta e valida que está na lista de
/// nomes conhecidos (R4); qualquer desvio — JSON inválido, campo ausente,
/// nome fora da lista — cai em `OUTROS`. Nunca panica (R11).
fn extrair_categoria(resposta: &str, categorias: &[Categoria]) -> String {
    let nome = serde_json::from_str::<serde_json::Value>(resposta)
        .ok()
        .and_then(|v| {
            v.get("categoria")
                .and_then(|c| c.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default();

    if categorias.iter().any(|c| c.nome == nome.trim()) {
        nome.trim().to_string()
    } else {
        OUTROS.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ClassificadorPalavraChave ------------------------------------------ //

    #[test]
    fn classifica_pela_primeira_categoria_cujo_nome_aparece_no_texto() {
        let doc = Documento::novo("link", "PORTARIA Nº 1 - 2024 - Progressão funcional");
        let categorias = vec![
            Categoria::nova("Progressão", None),
            Categoria::nova("Comissão", None),
        ];
        let resultado = ClassificadorPalavraChave.classificar(&doc, &categorias);
        assert_eq!(resultado, "Progressão");
    }

    #[test]
    fn cai_em_outros_quando_nenhuma_categoria_combina() {
        let doc = Documento::novo("link", "Comunicado interno qualquer");
        let categorias = vec![Categoria::nova("Progressão", None)];
        let resultado = ClassificadorPalavraChave.classificar(&doc, &categorias);
        assert_eq!(resultado, OUTROS);
    }

    #[test]
    fn ignora_acentuacao_e_caixa_ao_casar_o_nome_da_categoria() {
        let doc = Documento::novo("link", "PORTARIA Nº 2 - 2024 - PROGRESSAO por capacitacao");
        let categorias = vec![Categoria::nova("Progressão", None)];
        let resultado = ClassificadorPalavraChave.classificar(&doc, &categorias);
        assert_eq!(resultado, "Progressão");
    }

    #[test]
    fn casa_por_palavra_significativa_da_descricao_sem_citar_o_nome() {
        let doc = Documento::novo(
            "link",
            "PORTARIA Nº 3 - 2024 - Designação de banca examinadora",
        );
        let categorias = vec![Categoria::nova(
            "Comissão",
            Some("Designação de comissões, comitês, bancas ou equipes.".to_string()),
        )];
        let resultado = ClassificadorPalavraChave.classificar(&doc, &categorias);
        assert_eq!(resultado, "Comissão");
    }

    #[test]
    fn nao_casa_por_palavra_curta_da_descricao() {
        // "de" e "ou" não devem, sozinhas, disparar um falso positivo.
        let doc = Documento::novo("link", "Ofício de encaminhamento qualquer");
        let categorias = vec![Categoria::nova(
            "Comissão",
            Some("Designação de comissões ou comitês.".to_string()),
        )];
        let resultado = ClassificadorPalavraChave.classificar(&doc, &categorias);
        assert_eq!(resultado, OUTROS);
    }

    // --- ClassificadorLlm ---------------------------------------------------- //

    struct ChatFake {
        resultado: Result<String, AppError>,
    }

    impl ChatIa for ChatFake {
        fn chat(&self, _sistema: &str, _usuario: &str) -> Result<String, AppError> {
            match &self.resultado {
                Ok(s) => Ok(s.clone()),
                Err(_) => Err(AppError::FalhaIA {
                    motivo: "falha simulada".to_string(),
                }),
            }
        }
    }

    fn categorias_teste() -> Vec<Categoria> {
        vec![
            Categoria::nova("Progressão", None),
            Categoria::nova("Outros", None),
        ]
    }

    #[test]
    fn llm_classifica_certo_quando_resposta_e_json_valido_na_lista() {
        let chat = ChatFake {
            resultado: Ok(r#"{"categoria":"Progressão"}"#.to_string()),
        };
        let doc = Documento::novo("link", "Qualquer título");
        let resultado = ClassificadorLlm::novo(&chat).classificar(&doc, &categorias_teste());
        assert_eq!(resultado, "Progressão");
    }

    #[test]
    fn llm_cai_em_outros_quando_categoria_esta_fora_da_lista() {
        let chat = ChatFake {
            resultado: Ok(r#"{"categoria":"Categoria Inexistente"}"#.to_string()),
        };
        let doc = Documento::novo("link", "Qualquer título");
        let resultado = ClassificadorLlm::novo(&chat).classificar(&doc, &categorias_teste());
        assert_eq!(resultado, OUTROS);
    }

    #[test]
    fn llm_cai_em_outros_quando_json_invalido_sem_entrar_em_panico() {
        let chat = ChatFake {
            resultado: Ok("isto não é json".to_string()),
        };
        let doc = Documento::novo("link", "Qualquer título");
        let resultado = ClassificadorLlm::novo(&chat).classificar(&doc, &categorias_teste());
        assert_eq!(resultado, OUTROS);
    }

    #[test]
    fn llm_cai_em_palavra_chave_quando_a_chamada_de_ia_falha_r11() {
        let chat = ChatFake {
            resultado: Err(AppError::FalhaIA {
                motivo: "indisponível".to_string(),
            }),
        };
        let doc = Documento::novo("link", "PORTARIA Nº 1 - 2024 - Progressão funcional");
        let resultado = ClassificadorLlm::novo(&chat).classificar(&doc, &categorias_teste());
        assert_eq!(
            resultado, "Progressão",
            "sem resposta da IA, cai no classificador por palavra-chave (R11)"
        );
    }

    #[test]
    fn classificar_via_ia_propaga_o_erro_para_o_chamador_diferenciar_de_json_invalido() {
        let chat = ChatFake {
            resultado: Err(AppError::FalhaIA {
                motivo: "indisponível".to_string(),
            }),
        };
        let doc = Documento::novo("link", "Qualquer título");
        let erro = ClassificadorLlm::novo(&chat)
            .classificar_via_ia(&doc, &categorias_teste())
            .unwrap_err();
        assert!(matches!(erro, AppError::FalhaIA { .. }));
    }
}
