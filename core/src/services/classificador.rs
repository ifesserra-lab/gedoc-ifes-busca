//! Orquestra a classificação de um lote de documentos (US5): escolhe a
//! estratégia (`ModoClassificacao`) e aplica `Classificador` a cada
//! `Documento`, em lugar (`doc.categoria`).
//!
//! **Decisão de design**: o modo default é `Keyword` — grátis, instantâneo,
//! sem tocar API nem exigir chave; é o que `commands::buscar` usa a menos
//! que o pedido informe `modo: "llm"` explicitamente (custo/latência de IA
//! não deve incidir em toda busca). No modo `Llm`, cada classificação passa
//! primeiro pelo `Cache` por link (R6 — não reclassifica um documento já
//! visto) e só chama a IA em caso de cache miss; o throttle entre chamadas
//! (R9) fica embutido no adapter de IA (`ports::ia::MistralClient`), não
//! aqui. Uma falha ao classificar 1 documento via IA cai no classificador
//! `Keyword` só para aquele documento (R11) — e o resultado da falha **não**
//! é cacheado, para permitir nova tentativa numa busca futura.

use crate::domain::categoria::Categoria;
use crate::domain::documento::Documento;
use crate::ports::classificador::{Classificador, ClassificadorLlm, ClassificadorPalavraChave};
use crate::ports::ia::ChatIa;
use crate::services::cache::CacheArquivo;

/// Spec 010 — nº de documentos por chamada de classificação em lote. Título +
/// trecho truncado (~300 chars) são curtos, então este valor cabe bem dentro
/// do orçamento de tokens (`ports::ia::MAX_TOKENS_LOTE`); ajustável.
const TAMANHO_LOTE_CLASSIFICACAO: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModoClassificacao {
    #[default]
    Keyword,
    Llm,
}

impl ModoClassificacao {
    /// Interpreta a entrada do IPC (`"keyword"`/`"llm"`, sem diferenciar
    /// maiúsculas/minúsculas); qualquer outro valor — incluindo ausente —
    /// cai no default `Keyword`. Validação defensiva: nunca falha por causa
    /// de um valor inesperado vindo do frontend.
    pub fn from_entrada(valor: Option<&str>) -> Self {
        match valor.map(str::to_lowercase).as_deref() {
            Some("llm") => ModoClassificacao::Llm,
            _ => ModoClassificacao::Keyword,
        }
    }
}

/// Classifica cada documento de `docs`, em lugar. Ver decisão de design no
/// doc do módulo para o papel de `chat`/`cache` conforme `modo`.
///
/// Spec 010 (modo `Llm`): os documentos já cacheados (R6) são resolvidos na
/// hora, sem entrar em nenhum lote; os demais são divididos em blocos de até
/// `TAMANHO_LOTE_CLASSIFICACAO` e cada bloco vira **uma** chamada de IA
/// (`ClassificadorLlm::classificar_lote_via_ia`), em vez de uma chamada por
/// documento — reduz drasticamente o nº de chamadas em buscas grandes
/// (FR-001). Cada item é ancorado por índice (FR-002); um índice ausente na
/// resposta — ou o bloco inteiro falhando — cai no caminho por-documento já
/// existente (`classificar_com_cache`, que por sua vez cai em palavra-chave
/// se a IA falhar de novo), sem abortar o restante da busca (FR-004/R11).
pub fn classificar_lote(
    docs: &mut [Documento],
    categorias: &[Categoria],
    modo: ModoClassificacao,
    chat: Option<&dyn ChatIa>,
    mut cache: Option<&mut CacheArquivo>,
) {
    // Sem `chat` configurado (ex.: chave de IA ausente) o modo `Llm`
    // degrada para `Keyword` no lote inteiro — nunca aborta a busca (R11).
    let Some(llm) = (match modo {
        ModoClassificacao::Llm => chat.map(ClassificadorLlm::novo),
        ModoClassificacao::Keyword => None,
    }) else {
        for doc in docs.iter_mut() {
            doc.categoria = Some(ClassificadorPalavraChave.classificar(doc, categorias));
        }
        return;
    };

    // 1) Cache hits (R6) resolvidos na hora — não entram em nenhum lote.
    let mut pendentes = Vec::new();
    for (i, doc) in docs.iter_mut().enumerate() {
        match cache.as_deref().and_then(|c| c.obter(&doc.link)) {
            Some(categoria) => doc.categoria = Some(categoria.to_string()),
            None => pendentes.push(i),
        }
    }

    // 2) Os não-cacheados vão em blocos de TAMANHO_LOTE_CLASSIFICACAO.
    for bloco in pendentes.chunks(TAMANHO_LOTE_CLASSIFICACAO) {
        classificar_bloco_llm(docs, bloco, categorias, &llm, cache.as_deref_mut());
    }
}

/// Classifica um único bloco (`indices`, todos cache-miss) via
/// `classificar_lote_via_ia`. Cada item confirmado por índice é cacheado
/// (R6); um índice ausente na resposta — ou a chamada do bloco falhando por
/// completo (`Err`) — cai no caminho por-documento (`classificar_com_cache`)
/// só para aquele item, sem afetar os demais do bloco (FR-004).
fn classificar_bloco_llm<C: ChatIa + ?Sized>(
    docs: &mut [Documento],
    indices: &[usize],
    categorias: &[Categoria],
    llm: &ClassificadorLlm<'_, C>,
    mut cache: Option<&mut CacheArquivo>,
) {
    let refs: Vec<&Documento> = indices.iter().map(|&i| &docs[i]).collect();
    let resultados = llm
        .classificar_lote_via_ia(&refs, categorias)
        .unwrap_or_else(|_| vec![None; indices.len()]);

    for (pos, &i) in indices.iter().enumerate() {
        let categoria = match resultados.get(pos).cloned().flatten() {
            Some(categoria) => {
                if let Some(cache) = cache.as_deref_mut() {
                    cache.inserir(docs[i].link.clone(), categoria.clone());
                    // Falha ao persistir o cache não pode abortar a
                    // classificação do bloco (R11).
                    let _ = cache.salvar();
                }
                categoria
            }
            None => classificar_com_cache(llm, cache.as_deref_mut(), &docs[i], categorias),
        };
        docs[i].categoria = Some(categoria);
    }
}

/// Resolve a categoria de `doc` via cache (hit) ou via IA (miss); só grava
/// no cache o resultado de uma chamada de IA bem-sucedida (R6). Uma falha na
/// chamada cai no classificador por palavra-chave só para este documento,
/// sem interromper o restante do lote (R11).
fn classificar_com_cache<C: ChatIa + ?Sized>(
    llm: &ClassificadorLlm<'_, C>,
    cache: Option<&mut CacheArquivo>,
    doc: &Documento,
    categorias: &[Categoria],
) -> String {
    if let Some(categoria) = cache.as_deref().and_then(|c| c.obter(&doc.link)) {
        return categoria.to_string();
    }

    match llm.classificar_via_ia(doc, categorias) {
        Ok(categoria) => {
            if let Some(cache) = cache {
                cache.inserir(doc.link.clone(), categoria.clone());
                // Falha ao persistir o cache não pode abortar a classificação
                // do lote (R11) — na pior hipótese, reclassifica no futuro.
                let _ = cache.salvar();
            }
            categoria
        }
        Err(_) => ClassificadorPalavraChave.classificar(doc, categorias),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use tempfile::tempdir;

    use crate::error::AppError;

    use super::*;

    /// Dublê de `ChatIa` com filas independentes para a chamada de lote
    /// (`chat_lote`, usada por `classificar_lote_via_ia`) e para o fallback
    /// por-documento (`chat`, usado por `classificar_via_ia`) — permite a
    /// cada teste controlar precisamente o que a IA responde em cada
    /// caminho, já que spec 010 troca 1-chamada-por-doc por 1-chamada-por-bloco
    /// mais o fallback por item ausente/bloco com erro.
    struct ChatFake {
        respostas_lote: RefCell<VecDeque<Result<String, AppError>>>,
        respostas_doc: RefCell<VecDeque<Result<String, AppError>>>,
        chamadas: RefCell<u32>,
    }

    impl ChatFake {
        /// Configura só as respostas do caminho de lote (a maioria dos
        /// testes não precisa do fallback por-documento).
        fn com_respostas_lote(respostas: Vec<Result<String, AppError>>) -> Self {
            Self {
                respostas_lote: RefCell::new(respostas.into()),
                respostas_doc: RefCell::new(VecDeque::new()),
                chamadas: RefCell::new(0),
            }
        }

        /// Acrescenta respostas do fallback por-documento (`chat`).
        fn com_respostas_doc(self, respostas: Vec<Result<String, AppError>>) -> Self {
            Self {
                respostas_doc: RefCell::new(respostas.into()),
                ..self
            }
        }
    }

    impl ChatIa for ChatFake {
        fn chat(&self, _sistema: &str, _usuario: &str) -> Result<String, AppError> {
            *self.chamadas.borrow_mut() += 1;
            self.respostas_doc
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Ok("{}".to_string()))
        }

        fn chat_lote(
            &self,
            _sistema: &str,
            _usuario: &str,
            _temperatura: f64,
        ) -> Result<String, AppError> {
            *self.chamadas.borrow_mut() += 1;
            self.respostas_lote
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Ok(r#"{"itens":[]}"#.to_string()))
        }
    }

    fn doc(link: &str, titulo: &str) -> Documento {
        Documento::novo(link, titulo)
    }

    fn categorias_teste() -> Vec<Categoria> {
        vec![
            Categoria::nova("Progressão", None),
            Categoria::nova("Outros", None),
        ]
    }

    // --- from_entrada --------------------------------------------------------- //

    #[test]
    fn from_entrada_interpreta_llm_sem_diferenciar_caixa_e_ausente_como_keyword() {
        assert_eq!(
            ModoClassificacao::from_entrada(Some("llm")),
            ModoClassificacao::Llm
        );
        assert_eq!(
            ModoClassificacao::from_entrada(Some("LLM")),
            ModoClassificacao::Llm
        );
        assert_eq!(
            ModoClassificacao::from_entrada(Some("keyword")),
            ModoClassificacao::Keyword
        );
        assert_eq!(
            ModoClassificacao::from_entrada(Some("qualquercoisa")),
            ModoClassificacao::Keyword
        );
        assert_eq!(
            ModoClassificacao::from_entrada(None),
            ModoClassificacao::Keyword
        );
    }

    // --- classificar_lote: keyword -------------------------------------------- //

    #[test]
    fn modo_keyword_classifica_sem_chat_nem_cache() {
        let mut docs = vec![doc("l1", "PORTARIA Nº 1 - 2024 - Progressão funcional")];
        classificar_lote(
            &mut docs,
            &categorias_teste(),
            ModoClassificacao::Keyword,
            None,
            None,
        );
        assert_eq!(docs[0].categoria.as_deref(), Some("Progressão"));
    }

    // --- classificar_lote: llm ------------------------------------------------- //

    #[test]
    fn modo_llm_usa_a_resposta_da_ia_em_1_chamada_de_lote() {
        let chat = ChatFake::com_respostas_lote(vec![Ok(
            r#"{"itens":[{"i":0,"categoria":"Progressão"}]}"#.to_string(),
        )]);
        let mut docs = vec![doc("l1", "algo qualquer")];
        classificar_lote(
            &mut docs,
            &categorias_teste(),
            ModoClassificacao::Llm,
            Some(&chat),
            None,
        );
        assert_eq!(docs[0].categoria.as_deref(), Some("Progressão"));
        assert_eq!(
            *chat.chamadas.borrow(),
            1,
            "1 documento -> 1 chamada de lote, sem precisar de fallback"
        );
    }

    #[test]
    fn modo_llm_classifica_varios_documentos_em_1_unica_chamada_de_lote() {
        let chat = ChatFake::com_respostas_lote(vec![Ok(r#"{"itens":[
            {"i":0,"categoria":"Progressão"},
            {"i":1,"categoria":"Outros"}
        ]}"#
        .to_string())]);
        let mut docs = vec![doc("l1", "algo"), doc("l2", "outra coisa")];
        classificar_lote(
            &mut docs,
            &categorias_teste(),
            ModoClassificacao::Llm,
            Some(&chat),
            None,
        );
        assert_eq!(docs[0].categoria.as_deref(), Some("Progressão"));
        assert_eq!(docs[1].categoria.as_deref(), Some("Outros"));
        assert_eq!(
            *chat.chamadas.borrow(),
            1,
            "N documentos no mesmo bloco -> 1 única chamada (SC-001)"
        );
    }

    #[test]
    fn modo_llm_sem_chat_configurado_degrada_o_lote_inteiro_para_keyword_r11() {
        let mut docs = vec![doc("l1", "PORTARIA Nº 1 - 2024 - Progressão funcional")];
        classificar_lote(
            &mut docs,
            &categorias_teste(),
            ModoClassificacao::Llm,
            None,
            None,
        );
        assert_eq!(
            docs[0].categoria.as_deref(),
            Some("Progressão"),
            "sem chat configurado, cai no keyword (R11)"
        );
    }

    #[test]
    fn modo_llm_cache_evita_chamar_a_ia_de_novo_para_o_mesmo_link_r6() {
        let dir = tempdir().expect("tempdir");
        let mut cache = CacheArquivo::carregar(dir.path().join("cache.json"));
        let chat = ChatFake::com_respostas_lote(vec![Ok(
            r#"{"itens":[{"i":0,"categoria":"Progressão"}]}"#.to_string(),
        )]);
        let categorias = categorias_teste();

        let mut primeira = vec![doc("l1", "x")];
        classificar_lote(
            &mut primeira,
            &categorias,
            ModoClassificacao::Llm,
            Some(&chat),
            Some(&mut cache),
        );
        assert_eq!(*chat.chamadas.borrow(), 1);

        let mut segunda = vec![doc("l1", "x")]; // mesmo link, "nova busca"
        classificar_lote(
            &mut segunda,
            &categorias,
            ModoClassificacao::Llm,
            Some(&chat),
            Some(&mut cache),
        );

        assert_eq!(segunda[0].categoria.as_deref(), Some("Progressão"));
        assert_eq!(
            *chat.chamadas.borrow(),
            1,
            "cache hit não deve chamar a IA de novo (R6)"
        );
    }

    #[test]
    fn modo_llm_falha_do_bloco_inteiro_cai_no_fallback_por_documento_sem_abortar_r11() {
        // A chamada de lote falha por completo (erro de comunicação) -> cada
        // documento do bloco cai no fallback por-documento
        // (`classificar_via_ia`); se este também falhar, cai em keyword.
        let chat = ChatFake::com_respostas_lote(vec![Err(AppError::FalhaIA {
            motivo: "instável".to_string(),
        })])
        .com_respostas_doc(vec![
            Err(AppError::FalhaIA {
                motivo: "instável".to_string(),
            }), // fallback do doc1 também falha -> keyword
            Ok(r#"{"categoria":"Progressão"}"#.to_string()), // fallback do doc2 funciona
        ]);
        let mut docs = vec![
            doc("l1", "PORTARIA Nº 1 - 2024 - Progressão funcional"), // IA falha (lote+fallback) -> keyword
            doc("l2", "qualquer coisa"), // IA responde certo no fallback por-documento
        ];
        classificar_lote(
            &mut docs,
            &categorias_teste(),
            ModoClassificacao::Llm,
            Some(&chat),
            None,
        );

        assert_eq!(
            docs[0].categoria.as_deref(),
            Some("Progressão"),
            "falha do bloco inteiro cai no fallback por-documento; falha deste cai em keyword (R11)"
        );
        assert_eq!(docs[1].categoria.as_deref(), Some("Progressão"));
    }

    #[test]
    fn modo_llm_item_ausente_na_resposta_do_bloco_cai_no_fallback_por_documento() {
        // A chamada de lote responde só para o item 1 -> o item 0 (ausente)
        // cai no fallback por-documento (`classificar_via_ia`), sem afetar o
        // item que já veio confirmado no bloco (FR-002/FR-004).
        let chat = ChatFake::com_respostas_lote(vec![Ok(
            r#"{"itens":[{"i":1,"categoria":"Progressão"}]}"#.to_string(),
        )])
        .com_respostas_doc(vec![Ok(r#"{"categoria":"Outros"}"#.to_string())]);
        let mut docs = vec![
            doc("l1", "qualquer coisa"), // ausente no lote -> fallback por-documento
            doc("l2", "outra coisa"),    // confirmado no lote
        ];
        classificar_lote(
            &mut docs,
            &categorias_teste(),
            ModoClassificacao::Llm,
            Some(&chat),
            None,
        );

        assert_eq!(docs[0].categoria.as_deref(), Some("Outros"));
        assert_eq!(
            docs[1].categoria.as_deref(),
            Some("Progressão"),
            "item confirmado no lote não deve ser afetado pelo fallback do outro"
        );
    }

    #[test]
    fn modo_llm_nao_cacheia_resultado_de_falha_permitindo_nova_tentativa() {
        let dir = tempdir().expect("tempdir");
        let mut cache = CacheArquivo::carregar(dir.path().join("cache.json"));
        // 1ª busca: o lote falha E o fallback por-documento falha → o
        // documento é classificado por palavra-chave, mas nada é cacheado
        // (só resultado de IA bem-sucedido entra no cache, R6). 2ª busca: o
        // lote responde e o resultado é cacheado.
        let chat = ChatFake::com_respostas_lote(vec![
            Err(AppError::FalhaIA {
                motivo: "instável".to_string(),
            }),
            Ok(r#"{"itens":[{"i":0,"categoria":"Progressão"}]}"#.to_string()),
        ])
        .com_respostas_doc(vec![Err(AppError::FalhaIA {
            motivo: "instável".to_string(),
        })]);
        let categorias = categorias_teste();
        let titulo = "PORTARIA Nº 1 - 2024 - Progressão funcional";

        let mut primeira = vec![doc("l1", titulo)];
        classificar_lote(
            &mut primeira,
            &categorias,
            ModoClassificacao::Llm,
            Some(&chat),
            Some(&mut cache),
        );
        assert_eq!(cache.obter("l1"), None, "falha não deve ser cacheada");

        let mut segunda = vec![doc("l1", titulo)];
        classificar_lote(
            &mut segunda,
            &categorias,
            ModoClassificacao::Llm,
            Some(&chat),
            Some(&mut cache),
        );
        assert_eq!(
            cache.obter("l1"),
            Some("Progressão"),
            "sem cache de erro, a 2ª tentativa via lote é cacheada"
        );
    }
}
