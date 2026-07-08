//! R11/R6 (integração, via API pública do crate): a falha ao resumir 1
//! documento não aborta o lote inteiro — os demais são resumidos
//! normalmente — e o cache por link evita chamar a IA de novo para um
//! documento já resumido, sem tocar rede real (dublê de `ChatIa`, Princípio
//! VII).

use std::cell::RefCell;
use std::collections::VecDeque;

use gedocs_core::domain::documento::Documento;
use gedocs_core::error::AppError;
use gedocs_core::ports::ia::ChatIa;
use gedocs_core::services::cache::CacheArquivo;
use gedocs_core::services::resumidor::resumir_lote;

const SIAPE: &str = "1998547";

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
        unreachable!("resumir_lote usa resumir(), não chat()")
    }

    fn resumir(&self, _sistema: &str, _usuario: &str) -> Result<String, AppError> {
        *self.chamadas.borrow_mut() += 1;
        self.respostas
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| Ok("resumo padrão".to_string()))
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
    let chat = ChatFake::com_respostas(vec![
        Err(AppError::FalhaIA {
            motivo: "instável".to_string(),
        }),
        Ok("Resumo do segundo documento.".to_string()),
        Ok("Resumo do terceiro documento.".to_string()),
    ]);
    let mut docs = vec![
        doc_com_trecho("l1", "PORTARIA Nº 1 - 2024 - A", "trecho A"),
        doc_com_trecho("l2", "PORTARIA Nº 2 - 2024 - B", "trecho B"),
        doc_com_trecho("l3", "PORTARIA Nº 3 - 2024 - C", "trecho C"),
    ];

    resumir_lote(&mut docs, SIAPE, &chat, dir.path(), None);

    assert_eq!(
        docs[0].resumo, None,
        "documento cuja chamada de IA falhou fica com resumo None (R11)"
    );
    assert_eq!(
        docs[1].resumo.as_deref(),
        Some("Resumo do segundo documento.")
    );
    assert_eq!(
        docs[2].resumo.as_deref(),
        Some("Resumo do terceiro documento.")
    );
    assert_eq!(*chat.chamadas.borrow(), 3, "tentou resumir os 3 documentos");
}

#[test]
fn cache_por_link_evita_chamar_a_ia_de_novo_para_o_mesmo_documento_r6() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut cache = CacheArquivo::carregar(dir.path().join("resumo.json"));
    let chat = ChatFake::com_respostas(vec![Ok("Resumo cacheado.".to_string())]);

    let mut primeira = vec![doc_com_trecho("l1", "PORTARIA Nº 1 - 2024 - X", "trecho")];
    resumir_lote(&mut primeira, SIAPE, &chat, dir.path(), Some(&mut cache));
    assert_eq!(*chat.chamadas.borrow(), 1);

    // "nova busca", mais tarde, encontra o mesmo link.
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
    let chat = ChatFake::com_respostas(vec![
        Err(AppError::FalhaIA {
            motivo: "instável".to_string(),
        }),
        Ok("Resumo na 2ª tentativa.".to_string()),
    ]);

    let mut primeira = vec![doc_com_trecho("l1", "PORTARIA Nº 1 - 2024 - X", "trecho")];
    resumir_lote(&mut primeira, SIAPE, &chat, dir.path(), Some(&mut cache));
    assert_eq!(primeira[0].resumo, None);

    let mut segunda = vec![doc_com_trecho("l1", "PORTARIA Nº 1 - 2024 - X", "trecho")];
    resumir_lote(&mut segunda, SIAPE, &chat, dir.path(), Some(&mut cache));

    assert_eq!(
        *chat.chamadas.borrow(),
        2,
        "sem cache de erro, tenta a IA de novo na busca seguinte"
    );
    assert_eq!(
        segunda[0].resumo.as_deref(),
        Some("Resumo na 2ª tentativa.")
    );
}
