//! Strategy (GoF) — política de classificação de um Documento em exatamente
//! uma Categoria (R4). Duas estratégias:
//! - `ClassificadorPalavraChave`: casamento por nome/descrição da categoria,
//!   sem custo de API, instantânea (default de `commands::buscar`).
//! - `ClassificadorLlm`: guiada pelas descrições de `config/categoria.json`
//!   via um `ports::ia::ChatIa` (US5). Uma falha de comunicação com a IA cai
//!   no `ClassificadorPalavraChave` para aquele documento (R11 — não derruba
//!   o lote); uma resposta fora da lista de categorias cai em `OUTROS` (R4).

use serde_json::Value;

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

    /// Spec 010 — classifica vários `docs` numa única chamada de IA
    /// (`ChatIa::chat_lote`), ancorando cada resultado por índice (nunca por
    /// ordem posicional — FR-002). Devolve um vetor do mesmo tamanho de
    /// `docs`: `Some(categoria)` para cada índice confirmado na resposta
    /// (fora da lista de `categorias` → OUTROS, mesma validação do caminho
    /// por-documento); `None` para índice ausente — o chamador
    /// (`services::classificador::classificar_lote`) decide o fallback
    /// por-documento (FR-004). Erro de comunicação com a IA propaga como
    /// `Err` — o chamador faz o fallback do lote inteiro.
    pub(crate) fn classificar_lote_via_ia(
        &self,
        docs: &[&Documento],
        categorias: &[Categoria],
    ) -> Result<Vec<Option<String>>, AppError> {
        let (sistema, usuario) = montar_prompt_lote(docs, categorias);
        let resposta = self.chat.chat_lote(&sistema, &usuario, 0.0)?;
        Ok(extrair_categorias_lote(&resposta, docs.len(), categorias))
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

    validar_categoria(&nome, categorias)
}

/// Valida que `nome` (já aparado) pertence à lista de `categorias` (R4);
/// fora do conjunto — incluindo vazio — cai em `OUTROS`. Compartilhado pelo
/// caminho por-documento (`extrair_categoria`) e pelo de lote
/// (`extrair_categorias_lote`), spec 010.
fn validar_categoria(nome: &str, categorias: &[Categoria]) -> String {
    let nome = nome.trim();
    if categorias.iter().any(|c| c.nome == nome) {
        nome.to_string()
    } else {
        OUTROS.to_string()
    }
}

/// Monta (sistema, usuário) para classificar **vários** documentos numa só
/// chamada (spec 010): mesmas categorias/descrições de `montar_prompt`; o
/// usuário numera cada documento por índice (0-based, na ordem de `docs`) —
/// a resposta deve ancorar cada item por esse índice (`extrair_categorias_lote`),
/// nunca por ordem posicional (FR-002). Trecho truncado a ~300 chars por
/// documento (bem menor que o limite do prompt por-documento, para caber
/// vários no mesmo orçamento de tokens). Função pura, testável sem rede.
fn montar_prompt_lote(docs: &[&Documento], categorias: &[Categoria]) -> (String, String) {
    let definicoes = categorias
        .iter()
        .map(|c| format!("- {}: {}", c.nome, c.descricao.as_deref().unwrap_or("")))
        .collect::<Vec<_>>()
        .join("\n");

    let sistema = "Você classifica documentos administrativos do IFES. Cada documento \
listado deve receber exatamente UMA categoria da lista fornecida, com base nas \
descrições. Classifique CADA documento numerado abaixo. Responda apenas em \
JSON: {\"itens\":[{\"i\": <indice>, \"categoria\": \"<nome exato da categoria>\"}]}."
        .to_string();

    let documentos = docs
        .iter()
        .enumerate()
        .map(|(i, doc)| {
            let trecho: String = doc
                .trecho
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(300)
                .collect();
            format!("{i}: Título: {}\nTrecho: {trecho}", doc.titulo)
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let usuario = format!("Categorias disponíveis:\n{definicoes}\n\nDocumentos:\n{documentos}");

    (sistema, usuario)
}

/// Extrai as categorias do JSON de lote `{"itens":[{"i":<indice>,"categoria":"<nome>"}]}`
/// (spec 010). Devolve um vetor de tamanho `n` (mesmo tamanho do lote
/// enviado): `Some(categoria_validada)` para cada índice presente na resposta
/// — validado contra `categorias` como em `extrair_categoria` (fora da lista
/// → OUTROS) —, `None` para índice ausente/fora do intervalo (o chamador
/// aplica o fallback por-documento, FR-004). JSON inválido, ou sem o campo
/// `itens`, devolve todos `None` — nunca panica (R11).
fn extrair_categorias_lote(
    resposta: &str,
    n: usize,
    categorias: &[Categoria],
) -> Vec<Option<String>> {
    let mut resultado = vec![None; n];

    let itens = serde_json::from_str::<serde_json::Value>(resposta)
        .ok()
        .and_then(|v| v.get("itens").cloned())
        .and_then(|v| v.as_array().cloned());

    let Some(itens) = itens else {
        return resultado;
    };

    for item in itens {
        let Some(indice) = item.get("i").and_then(Value::as_u64).map(|v| v as usize) else {
            continue;
        };
        if indice >= n {
            continue;
        }
        let nome = item
            .get("categoria")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        resultado[indice] = Some(validar_categoria(nome, categorias));
    }

    resultado
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- prompt da IA usa as categorias do category.json (spec 006) --------- //

    #[test]
    fn prompt_da_ia_inclui_nome_e_descricao_de_cada_categoria() {
        let categorias = vec![
            Categoria::nova(
                "Progressão",
                Some("Progressão funcional por mérito".to_string()),
            ),
            Categoria::nova(
                "Férias",
                Some("Concessão e interrupção de férias".to_string()),
            ),
            Categoria::nova(OUTROS, Some("Demais documentos".to_string())),
        ];
        let doc = Documento::novo("link", "PORTARIA Nº 1 - 2024 - Progressão funcional");

        let (sistema, usuario) = montar_prompt(&doc, &categorias);

        // Sistema orienta a escolher UMA categoria da lista fornecida.
        assert!(sistema.contains("UMA"));
        // Usuário lista as categorias do config (nome + descrição) — FR-002/SC-002.
        assert!(usuario.contains("Categorias disponíveis:"));
        for c in &categorias {
            assert!(
                usuario.contains(&c.nome),
                "o prompt deve citar a categoria '{}'",
                c.nome
            );
            let descricao = c.descricao.as_deref().unwrap();
            assert!(
                usuario.contains(descricao),
                "o prompt deve citar a descrição de '{}'",
                c.nome
            );
        }
    }

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

    // --- classificação em lote (spec 010) ------------------------------------ //

    #[test]
    fn montar_prompt_lote_inclui_categorias_e_documentos_numerados() {
        let categorias = vec![
            Categoria::nova(
                "Progressão",
                Some("Progressão funcional por mérito".to_string()),
            ),
            Categoria::nova(OUTROS, Some("Demais documentos".to_string())),
        ];
        let doc1 = Documento::novo("l1", "PORTARIA Nº 1 - 2024 - Progressão funcional");
        let doc2 = Documento::novo("l2", "DESPACHO Nº 2 - 2024 - Assunto qualquer");
        let docs = vec![&doc1, &doc2];

        let (sistema, usuario) = montar_prompt_lote(&docs, &categorias);

        assert!(sistema.contains("CADA documento"));
        assert!(sistema.contains("itens"));
        assert!(usuario.contains("Categorias disponíveis:"));
        for c in &categorias {
            assert!(usuario.contains(&c.nome));
            assert!(usuario.contains(c.descricao.as_deref().unwrap()));
        }
        // cada documento aparece numerado pelo seu índice (0-based) — ancoragem
        // por índice, não por ordem posicional (FR-002).
        assert!(usuario.contains("0: Título: PORTARIA Nº 1 - 2024 - Progressão funcional"));
        assert!(usuario.contains("1: Título: DESPACHO Nº 2 - 2024 - Assunto qualquer"));
    }

    #[test]
    fn extrair_categorias_lote_mapeia_cada_item_pelo_indice() {
        let categorias = categorias_teste();
        let resposta =
            r#"{"itens":[{"i":1,"categoria":"Progressão"},{"i":0,"categoria":"Outros"}]}"#;

        let resultado = extrair_categorias_lote(resposta, 2, &categorias);

        assert_eq!(
            resultado,
            vec![Some("Outros".to_string()), Some("Progressão".to_string())]
        );
    }

    #[test]
    fn extrair_categorias_lote_categoria_fora_da_lista_cai_em_outros() {
        let categorias = categorias_teste();
        let resposta = r#"{"itens":[{"i":0,"categoria":"Categoria Inexistente"}]}"#;

        let resultado = extrair_categorias_lote(resposta, 1, &categorias);

        assert_eq!(resultado, vec![Some(OUTROS.to_string())]);
    }

    #[test]
    fn extrair_categorias_lote_indice_ausente_fica_none() {
        let categorias = categorias_teste();
        let resposta = r#"{"itens":[{"i":0,"categoria":"Progressão"}]}"#;

        let resultado = extrair_categorias_lote(resposta, 3, &categorias);

        assert_eq!(
            resultado,
            vec![Some("Progressão".to_string()), None, None],
            "índices sem item na resposta ficam None (fallback por-documento)"
        );
    }

    #[test]
    fn extrair_categorias_lote_json_invalido_devolve_todos_none_sem_panico() {
        let categorias = categorias_teste();

        let resultado = extrair_categorias_lote("isto não é JSON", 3, &categorias);

        assert_eq!(resultado, vec![None, None, None]);
    }

    #[test]
    fn extrair_categorias_lote_sem_campo_itens_devolve_todos_none() {
        let categorias = categorias_teste();

        let resultado = extrair_categorias_lote(r#"{"outra_coisa":[]}"#, 2, &categorias);

        assert_eq!(resultado, vec![None, None]);
    }

    struct ChatLoteFake {
        resultado: Result<String, AppError>,
    }

    impl ChatIa for ChatLoteFake {
        fn chat(&self, _sistema: &str, _usuario: &str) -> Result<String, AppError> {
            unreachable!("classificar_lote_via_ia usa chat_lote(), não chat()")
        }

        fn chat_lote(
            &self,
            _sistema: &str,
            _usuario: &str,
            _temperatura: f64,
        ) -> Result<String, AppError> {
            match &self.resultado {
                Ok(s) => Ok(s.clone()),
                Err(_) => Err(AppError::FalhaIA {
                    motivo: "falha simulada".to_string(),
                }),
            }
        }
    }

    #[test]
    fn classificar_lote_via_ia_ancora_resultado_por_indice() {
        let chat = ChatLoteFake {
            resultado: Ok(r#"{"itens":[{"i":0,"categoria":"Progressão"}]}"#.to_string()),
        };
        let doc = Documento::novo("l1", "Qualquer título");
        let docs = vec![&doc];

        let resultado = ClassificadorLlm::novo(&chat)
            .classificar_lote_via_ia(&docs, &categorias_teste())
            .expect("deve extrair o resultado do lote");

        assert_eq!(resultado, vec![Some("Progressão".to_string())]);
    }

    #[test]
    fn classificar_lote_via_ia_propaga_erro_de_comunicacao_da_ia() {
        let chat = ChatLoteFake {
            resultado: Err(AppError::FalhaIA {
                motivo: "indisponível".to_string(),
            }),
        };
        let doc = Documento::novo("l1", "Qualquer título");
        let docs = vec![&doc];

        let erro = ClassificadorLlm::novo(&chat)
            .classificar_lote_via_ia(&docs, &categorias_teste())
            .unwrap_err();

        assert!(matches!(erro, AppError::FalhaIA { .. }));
    }
}
