//! R11/R6 (integração, via API pública do crate): spec 011 — o resumo por IA
//! processa os documentos EM LOTE (`ChatIa::chat_lote`), ancorando cada
//! resumo por índice; um item não confirmado no lote cai no resumo
//! por-documento (`resumir`), sem abortar os demais (R11); o cache por link
//! evita chamar a IA de novo (R6). Sem rede real (dublê de `ChatIa`).

use std::cell::RefCell;
use std::collections::VecDeque;

use gedocs_core::domain::documento::Documento;
use gedocs_core::error::AppError;
use gedocs_core::ports::ia::ChatIa;
use gedocs_core::services::cache::CacheArquivo;
use gedocs_core::services::resumidor::resumir_lote;

const SIAPE: &str = "1998547";

/// Dublê com filas separadas: `lote` (respostas de `chat_lote`) e `doc`
/// (respostas do fallback por-documento, `resumir`). `chamadas` conta ambas.
struct ChatFake {
    lote: RefCell<VecDeque<Result<String, AppError>>>,
    doc: RefCell<VecDeque<Result<String, AppError>>>,
    chamadas: RefCell<u32>,
}

impl ChatFake {
    fn novo(lote: Vec<Result<String, AppError>>, doc: Vec<Result<String, AppError>>) -> Self {
        Self {
            lote: RefCell::new(lote.into()),
            doc: RefCell::new(doc.into()),
            chamadas: RefCell::new(0),
        }
    }
}

impl ChatIa for ChatFake {
    fn chat(&self, _sistema: &str, _usuario: &str) -> Result<String, AppError> {
        unreachable!("resumir_lote usa chat_lote()/resumir(), não chat()")
    }

    fn resumir(&self, _sistema: &str, _usuario: &str) -> Result<String, AppError> {
        *self.chamadas.borrow_mut() += 1;
        self.doc
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| Ok("resumo padrão".to_string()))
    }

    fn chat_lote(
        &self,
        _sistema: &str,
        _usuario: &str,
        _temperatura: f64,
    ) -> Result<String, AppError> {
        *self.chamadas.borrow_mut() += 1;
        self.lote
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| Ok(r#"{"itens":[]}"#.to_string()))
    }
}

fn doc_com_trecho(link: &str, titulo: &str, trecho: &str) -> Documento {
    let mut d = Documento::novo(link, titulo);
    d.trecho = Some(trecho.to_string());
    d
}

#[test]
fn falha_ao_resumir_1_documento_nao_aborta_o_lote_os_demais_sao_resumidos() {
    let dir = tempfile::tempdir().expect("tempdir");
    // O lote confirma os índices 1 e 2, mas NÃO o 0 → o doc 0 cai no fallback
    // por-documento, que aqui falha (Err) → resumo None (R11); os demais ok.
    let chat = ChatFake::novo(
        vec![Ok(r#"{"itens":[{"i":1,"resumo":"Resumo do segundo documento."},{"i":2,"resumo":"Resumo do terceiro documento."}]}"#.to_string())],
        vec![Err(AppError::FalhaIA { motivo: "instável".to_string() })],
    );
    let mut docs = vec![
        doc_com_trecho("l1", "PORTARIA Nº 1 - 2024 - A", "trecho A"),
        doc_com_trecho("l2", "PORTARIA Nº 2 - 2024 - B", "trecho B"),
        doc_com_trecho("l3", "PORTARIA Nº 3 - 2024 - C", "trecho C"),
    ];

    resumir_lote(&mut docs, SIAPE, &chat, dir.path(), None);

    assert_eq!(
        docs[0].resumo, None,
        "item não confirmado + fallback falho = None (R11)"
    );
    assert_eq!(
        docs[1].resumo.as_deref(),
        Some("Resumo do segundo documento.")
    );
    assert_eq!(
        docs[2].resumo.as_deref(),
        Some("Resumo do terceiro documento.")
    );
    // 1 chamada de lote + 1 fallback por-documento (doc 0).
    assert_eq!(*chat.chamadas.borrow(), 2);
}

#[test]
fn resume_os_3_documentos_em_1_unica_chamada_de_lote() {
    let dir = tempfile::tempdir().expect("tempdir");
    let chat = ChatFake::novo(
        vec![Ok(
            r#"{"itens":[{"i":0,"resumo":"R0"},{"i":1,"resumo":"R1"},{"i":2,"resumo":"R2"}]}"#
                .to_string(),
        )],
        vec![],
    );
    let mut docs = vec![
        doc_com_trecho("l1", "A", "trecho A"),
        doc_com_trecho("l2", "B", "trecho B"),
        doc_com_trecho("l3", "C", "trecho C"),
    ];

    resumir_lote(&mut docs, SIAPE, &chat, dir.path(), None);

    assert_eq!(
        [
            docs[0].resumo.as_deref(),
            docs[1].resumo.as_deref(),
            docs[2].resumo.as_deref()
        ],
        [Some("R0"), Some("R1"), Some("R2")]
    );
    assert_eq!(
        *chat.chamadas.borrow(),
        1,
        "3 documentos, 1 chamada de lote (spec 011)"
    );
}

#[test]
fn cache_por_link_evita_chamar_a_ia_de_novo_para_o_mesmo_documento_r6() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut cache = CacheArquivo::carregar(dir.path().join("resumo.json"));
    let chat = ChatFake::novo(
        vec![Ok(
            r#"{"itens":[{"i":0,"resumo":"Resumo cacheado."}]}"#.to_string()
        )],
        vec![],
    );

    let mut primeira = vec![doc_com_trecho("l1", "PORTARIA Nº 1 - 2024 - X", "trecho")];
    resumir_lote(&mut primeira, SIAPE, &chat, dir.path(), Some(&mut cache));
    assert_eq!(*chat.chamadas.borrow(), 1);

    let mut segunda = vec![doc_com_trecho("l1", "PORTARIA Nº 1 - 2024 - X", "trecho")];
    resumir_lote(&mut segunda, SIAPE, &chat, dir.path(), Some(&mut cache));

    assert_eq!(
        *chat.chamadas.borrow(),
        1,
        "documento já resumido não deve chamar a IA de novo (R6)"
    );
    assert_eq!(segunda[0].resumo.as_deref(), Some("Resumo cacheado."));
}

#[test]
fn cache_nao_persiste_falha_permitindo_nova_tentativa_numa_busca_futura() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut cache = CacheArquivo::carregar(dir.path().join("resumo.json"));
    // 1ª busca: lote falha e o fallback por-doc também → None, não cacheia.
    // 2ª busca: lote responde → cacheia.
    let chat = ChatFake::novo(
        vec![
            Err(AppError::FalhaIA {
                motivo: "instável".to_string(),
            }),
            Ok(r#"{"itens":[{"i":0,"resumo":"Resumo na 2ª tentativa."}]}"#.to_string()),
        ],
        vec![Err(AppError::FalhaIA {
            motivo: "instável".to_string(),
        })],
    );

    let mut primeira = vec![doc_com_trecho("l1", "PORTARIA Nº 1 - 2024 - X", "trecho")];
    resumir_lote(&mut primeira, SIAPE, &chat, dir.path(), Some(&mut cache));
    assert_eq!(primeira[0].resumo, None);

    let mut segunda = vec![doc_com_trecho("l1", "PORTARIA Nº 1 - 2024 - X", "trecho")];
    resumir_lote(&mut segunda, SIAPE, &chat, dir.path(), Some(&mut cache));

    assert_eq!(
        segunda[0].resumo.as_deref(),
        Some("Resumo na 2ª tentativa."),
        "sem cache de erro, a 2ª tentativa via lote resume e cacheia"
    );
}
