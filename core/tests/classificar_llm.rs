//! R4/R6 (integração, via API pública do crate): a estratégia via IA cai em
//! "Outros" quando a resposta não está na lista de categorias (ou não é um
//! JSON válido), e o cache por link evita reclassificar um documento já
//! visto — tudo sem tocar rede real (dublê de `ChatIa`, Princípio VII).

use std::cell::RefCell;

use gedocs_core::domain::categoria::{Categoria, OUTROS};
use gedocs_core::domain::documento::Documento;
use gedocs_core::error::AppError;
use gedocs_core::ports::classificador::{Classificador, ClassificadorLlm};
use gedocs_core::ports::ia::ChatIa;
use gedocs_core::services::cache::CacheArquivo;
use gedocs_core::services::classificador::{classificar_lote, ModoClassificacao};

struct ChatFake {
    resposta: String,
    chamadas: RefCell<u32>,
}

impl ChatFake {
    fn com_resposta(resposta: &str) -> Self {
        Self {
            resposta: resposta.to_string(),
            chamadas: RefCell::new(0),
        }
    }
}

impl ChatIa for ChatFake {
    fn chat(&self, _sistema: &str, _usuario: &str) -> Result<String, AppError> {
        *self.chamadas.borrow_mut() += 1;
        Ok(self.resposta.clone())
    }
}

fn categorias_padrao() -> Vec<Categoria> {
    vec![
        Categoria::nova("Progressão", Some("Progressão funcional.".to_string())),
        Categoria::nova(OUTROS, None),
    ]
}

#[test]
fn resposta_fora_da_lista_de_categorias_cai_em_outros() {
    let chat = ChatFake::com_resposta(r#"{"categoria":"Categoria Inexistente"}"#);
    let doc = Documento::novo("l1", "Qualquer título");

    let categoria = ClassificadorLlm::novo(&chat).classificar(&doc, &categorias_padrao());

    assert_eq!(categoria, OUTROS);
}

#[test]
fn json_invalido_cai_em_outros_sem_entrar_em_panico() {
    let chat = ChatFake::com_resposta("isto não é JSON");
    let doc = Documento::novo("l2", "Qualquer título");

    let categoria = ClassificadorLlm::novo(&chat).classificar(&doc, &categorias_padrao());

    assert_eq!(categoria, OUTROS);
}

#[test]
fn cache_por_link_evita_reclassificar_o_mesmo_documento_r6() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut cache = CacheArquivo::carregar(dir.path().join("classificacao.json"));
    let chat = ChatFake::com_resposta(r#"{"categoria":"Progressão"}"#);
    let categorias = categorias_padrao();

    let mut docs = vec![Documento::novo("l1", "Qualquer título")];
    classificar_lote(
        &mut docs,
        &categorias,
        ModoClassificacao::Llm,
        Some(&chat),
        Some(&mut cache),
    );
    assert_eq!(*chat.chamadas.borrow(), 1);

    // "nova busca", mais tarde, encontra o mesmo link.
    let mut docs_de_novo = vec![Documento::novo("l1", "Qualquer título")];
    classificar_lote(
        &mut docs_de_novo,
        &categorias,
        ModoClassificacao::Llm,
        Some(&chat),
        Some(&mut cache),
    );

    assert_eq!(
        *chat.chamadas.borrow(),
        1,
        "documento já visto não deve chamar a IA de novo (R6)"
    );
    assert_eq!(docs_de_novo[0].categoria.as_deref(), Some("Progressão"));
}
