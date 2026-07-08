//! R1 (integração, via API pública do crate): o resumo deriva do texto real
//! do documento — nunca de um texto diferente/inventado — e cai no trecho da
//! busca quando não há um PDF já baixado em disco. Um dublê de `ChatIa`
//! captura literalmente o texto que recebeu, provando que é o mesmo texto do
//! documento (trecho ou PDF extraído), sem tocar rede real (Princípio VII).

use std::cell::RefCell;
use std::fs;

use gedocs_core::domain::documento::Documento;
use gedocs_core::error::AppError;
use gedocs_core::ports::ia::ChatIa;
use gedocs_core::services::resumidor::resumir_lote;

const SIAPE: &str = "1998547";

/// Dublê de `ChatIa` que devolve um resumo fixo e guarda o `usuario`
/// recebido em cada chamada — permite provar que o texto enviado à IA é
/// exatamente o texto-fonte do documento (R1: nunca inventa, nunca troca a
/// fonte por outra coisa).
struct ChatCaptura {
    resumo: String,
    recebido: RefCell<Vec<String>>,
}

impl ChatCaptura {
    fn com_resumo(resumo: &str) -> Self {
        Self {
            resumo: resumo.to_string(),
            recebido: RefCell::new(Vec::new()),
        }
    }
}

impl ChatIa for ChatCaptura {
    fn chat(&self, _sistema: &str, _usuario: &str) -> Result<String, AppError> {
        unreachable!("resumir_lote usa resumir(), não chat()")
    }

    fn resumir(&self, _sistema: &str, usuario: &str) -> Result<String, AppError> {
        self.recebido.borrow_mut().push(usuario.to_string());
        Ok(self.resumo.clone())
    }
}

#[test]
fn sem_pdf_baixado_o_resumo_usa_o_trecho_da_busca_como_texto_fonte() {
    let dir = tempfile::tempdir().expect("tempdir");
    let chat = ChatCaptura::com_resumo("Resumo fiel ao trecho.");
    let mut doc = Documento::novo(
        "https://gedoc.ifes.edu.br/documento/aaaa?inline",
        "PORTARIA Nº 1 - 2024 - Progressão funcional",
    );
    doc.trecho = Some("Determina a progressão do servidor SIAPE 1998547.".to_string());
    let mut docs = vec![doc];

    resumir_lote(&mut docs, SIAPE, &chat, dir.path(), None);

    assert_eq!(docs[0].resumo.as_deref(), Some("Resumo fiel ao trecho."));
    assert_eq!(
        chat.recebido.borrow().as_slice(),
        ["Determina a progressão do servidor SIAPE 1998547.".to_string()],
        "o texto enviado à IA deve ser exatamente o trecho do documento (R1)"
    );
}

#[test]
fn com_pdf_baixado_o_resumo_usa_o_texto_extraido_do_pdf_em_vez_do_trecho() {
    let dir = tempfile::tempdir().expect("tempdir");
    let pasta_siape = dir.path().join(SIAPE);
    fs::create_dir_all(&pasta_siape).expect("cria pasta do siape");
    let pdf = include_bytes!("fixtures/documento_teste.pdf");
    fs::write(pasta_siape.join("2024_1_Assunto.pdf"), pdf).expect("grava fixture");

    let chat = ChatCaptura::com_resumo("Resumo fiel ao PDF.");
    let mut doc = Documento::novo(
        "https://gedoc.ifes.edu.br/documento/bbbb?inline",
        "PORTARIA Nº 1 - 2024 - Assunto",
    );
    doc.arquivo = Some("2024_1_Assunto.pdf".to_string());
    doc.trecho = Some("trecho não deveria ser usado quando há PDF baixado".to_string());
    let mut docs = vec![doc];

    resumir_lote(&mut docs, SIAPE, &chat, dir.path(), None);

    assert_eq!(docs[0].resumo.as_deref(), Some("Resumo fiel ao PDF."));
    let recebido = chat.recebido.borrow();
    assert!(
        recebido[0].contains("Documento de teste"),
        "deveria enviar o texto extraído do PDF, enviou: {:?}",
        recebido[0]
    );
    assert!(
        !recebido[0].contains("trecho não deveria ser usado"),
        "não deveria usar o trecho quando há PDF baixado"
    );
}

#[test]
fn sem_trecho_e_sem_pdf_o_resumo_e_o_marcador_sem_texto_sem_chamar_a_ia() {
    let dir = tempfile::tempdir().expect("tempdir");
    let chat = ChatCaptura::com_resumo("não deveria ser usado");
    let mut docs = vec![Documento::novo(
        "https://gedoc.ifes.edu.br/documento/cccc?inline",
        "PORTARIA Nº 1 - 2024 - Assunto",
    )];

    resumir_lote(&mut docs, SIAPE, &chat, dir.path(), None);

    assert_eq!(docs[0].resumo.as_deref(), Some("(sem texto)"));
    assert!(
        chat.recebido.borrow().is_empty(),
        "sem texto-fonte, não deve chamar a IA"
    );
}
