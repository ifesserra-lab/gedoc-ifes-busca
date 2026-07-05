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
pub fn classificar_lote(
    docs: &mut [Documento],
    categorias: &[Categoria],
    modo: ModoClassificacao,
    chat: Option<&dyn ChatIa>,
    mut cache: Option<&mut CacheArquivo>,
) {
    // Sem `chat` configurado (ex.: chave de IA ausente) o modo `Llm`
    // degrada para `Keyword` no lote inteiro — nunca aborta a busca (R11).
    let classificador_llm = match modo {
        ModoClassificacao::Llm => chat.map(ClassificadorLlm::novo),
        ModoClassificacao::Keyword => None,
    };

    for doc in docs.iter_mut() {
        let categoria = match &classificador_llm {
            Some(llm) => classificar_com_cache(llm, cache.as_deref_mut(), doc, categorias),
            None => ClassificadorPalavraChave.classificar(doc, categorias),
        };
        doc.categoria = Some(categoria);
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

    struct ChatFake {
        respostas: RefCell<VecDeque<Result<String, AppError>>>,
        chamadas: RefCell<u32>,
    }

    impl ChatFake {
        fn com_respostas(respostas: Vec<Result<String, AppError>>) -> Self {
            Self {
                respostas: RefCell::new(respostas.into()),
                chamadas: RefCell::new(0),
            }
        }
    }

    impl ChatIa for ChatFake {
        fn chat(&self, _sistema: &str, _usuario: &str) -> Result<String, AppError> {
            *self.chamadas.borrow_mut() += 1;
            self.respostas
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Ok("{}".to_string()))
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
    fn modo_llm_usa_a_resposta_da_ia() {
        let chat = ChatFake::com_respostas(vec![Ok(r#"{"categoria":"Progressão"}"#.to_string())]);
        let mut docs = vec![doc("l1", "algo qualquer")];
        classificar_lote(
            &mut docs,
            &categorias_teste(),
            ModoClassificacao::Llm,
            Some(&chat),
            None,
        );
        assert_eq!(docs[0].categoria.as_deref(), Some("Progressão"));
        assert_eq!(*chat.chamadas.borrow(), 1);
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
        let chat = ChatFake::com_respostas(vec![Ok(r#"{"categoria":"Progressão"}"#.to_string())]);
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
    fn modo_llm_falha_de_um_doc_cai_em_keyword_sem_abortar_o_lote_r11() {
        let chat = ChatFake::com_respostas(vec![
            Err(AppError::FalhaIA {
                motivo: "instável".to_string(),
            }),
            Ok(r#"{"categoria":"Progressão"}"#.to_string()),
        ]);
        let mut docs = vec![
            doc("l1", "PORTARIA Nº 1 - 2024 - Progressão funcional"), // IA falha -> keyword
            doc("l2", "qualquer coisa"),                              // IA responde certo
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
            "falha na IA cai em keyword (R11)"
        );
        assert_eq!(docs[1].categoria.as_deref(), Some("Progressão"));
    }

    #[test]
    fn modo_llm_nao_cacheia_resultado_de_falha_permitindo_nova_tentativa() {
        let dir = tempdir().expect("tempdir");
        let mut cache = CacheArquivo::carregar(dir.path().join("cache.json"));
        let chat = ChatFake::com_respostas(vec![
            Err(AppError::FalhaIA {
                motivo: "instável".to_string(),
            }),
            Ok(r#"{"categoria":"Progressão"}"#.to_string()),
        ]);
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
            *chat.chamadas.borrow(),
            2,
            "sem cache de erro, tenta a IA de novo"
        );
        assert_eq!(cache.obter("l1"), Some("Progressão"));
    }
}
