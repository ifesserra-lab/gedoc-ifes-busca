//! Download de documentos (US4) — grava PDFs no diretório de dados do app,
//! **fora do repositório** (Princípio II/LGPD, R7: PDFs contêm PII de
//! terceiros e nunca podem ser versionados). Reaproveita o nome
//! determinístico de `domain::nome_arquivo` (R3); a resolução do diretório
//! real (`app_data_dir`) é responsabilidade de `commands::documento`, que
//! injeta o `HttpPort` e o diretório aqui — nenhum teste deste módulo toca
//! rede ou o disco do repositório (Princípio VII): a rede é um `HttpPort`
//! dublê e o disco é um `tempfile::TempDir`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::documento::Documento;
use crate::domain::nome_arquivo::nome_arquivo;
use crate::domain::siape;
use crate::error::AppError;
use crate::ports::http::HttpPort;

/// Baixa `doc.link` (via `http`) para dentro de `dir_destino`, criando o
/// diretório se necessário, e grava o PDF com o nome determinístico de R3.
/// **Idempotente**: se o arquivo já existe em `dir_destino`, não baixa de
/// novo — apenas retorna o nome. Retorna só o **nome** do arquivo (nunca o
/// caminho absoluto — R7): quem cruza o IPC recompõe o caminho a partir do
/// SIAPE, no backend, nunca a partir de um caminho vindo do cliente.
pub fn baixar_documento<H: HttpPort>(
    http: &H,
    doc: &Documento,
    dir_destino: &Path,
) -> Result<String, AppError> {
    let nome = nome_arquivo(doc);
    let caminho = dir_destino.join(&nome);

    if caminho.is_file() {
        return Ok(nome);
    }

    fs::create_dir_all(dir_destino).map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Falha ao criar diretório '{}': {e}", dir_destino.display()),
    })?;

    let bytes = http.get_bytes(&doc.link)?;

    fs::write(&caminho, bytes).map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Falha ao gravar '{nome}': {e}"),
    })?;

    Ok(nome)
}

/// R7 — resolve `base/siape/arquivo`, validando que `siape` é uma matrícula
/// válida (R10) e que `arquivo` não escapa do diretório do SIAPE: rejeita
/// vazio, `/`, `\` e `..` (path traversal). Função pura, não toca o disco —
/// só valida e monta o caminho; a checagem de existência é de quem chama.
pub fn caminho_seguro(base: &Path, siape: &str, arquivo: &str) -> Result<PathBuf, AppError> {
    siape::validar(siape)?;

    let invalido = arquivo.is_empty()
        || arquivo.contains('/')
        || arquivo.contains('\\')
        || arquivo.contains("..");
    if invalido {
        return Err(AppError::FalhaArquivo {
            motivo: format!("Nome de arquivo inválido: '{arquivo}'"),
        });
    }

    Ok(base.join(siape).join(arquivo))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use tempfile::tempdir;

    use super::*;

    /// Dublê síncrono de `HttpPort`: `get_bytes` devolve sempre os mesmos
    /// bytes configurados e conta quantas vezes foi chamado (para provar a
    /// idempotência do download). `get`/`post_form` não são usados aqui.
    struct FakeHttp {
        bytes: Vec<u8>,
        chamadas: RefCell<u32>,
    }

    impl FakeHttp {
        fn novo(bytes: &[u8]) -> Self {
            Self {
                bytes: bytes.to_vec(),
                chamadas: RefCell::new(0),
            }
        }
    }

    impl HttpPort for FakeHttp {
        fn get(&self, _url: &str) -> Result<String, AppError> {
            unreachable!("baixar_documento não usa get()")
        }

        fn post_form(&self, _url: &str, _campos: &[(String, String)]) -> Result<String, AppError> {
            unreachable!("baixar_documento não usa post_form()")
        }

        fn get_bytes(&self, _url: &str) -> Result<Vec<u8>, AppError> {
            *self.chamadas.borrow_mut() += 1;
            Ok(self.bytes.clone())
        }
    }

    const PDF_FALSO: &[u8] = b"%PDF-1.4 fake";

    fn doc_fake() -> Documento {
        Documento::novo(
            "https://gedoc.ifes.edu.br/documento/aaaa?inline",
            "PORTARIA Nº 7 - 2024 - Progressão funcional",
        )
    }

    // --- baixar_documento -------------------------------------------------- //

    #[test]
    fn grava_o_pdf_com_o_nome_deterministico_r3() {
        let dir = tempdir().expect("cria tempdir");
        let http = FakeHttp::novo(PDF_FALSO);
        let doc = doc_fake();

        let nome = baixar_documento(&http, &doc, dir.path()).expect("deve baixar");

        assert_eq!(nome, nome_arquivo(&doc));
        let conteudo = fs::read(dir.path().join(&nome)).expect("arquivo deve existir");
        assert_eq!(conteudo, PDF_FALSO);
        assert_eq!(*http.chamadas.borrow(), 1);
    }

    #[test]
    fn e_idempotente_nao_rebaixa_arquivo_existente() {
        let dir = tempdir().expect("cria tempdir");
        let http = FakeHttp::novo(PDF_FALSO);
        let doc = doc_fake();

        let primeiro = baixar_documento(&http, &doc, dir.path()).expect("1ª chamada baixa");
        let segundo = baixar_documento(&http, &doc, dir.path()).expect("2ª chamada é idempotente");

        assert_eq!(primeiro, segundo);
        assert_eq!(
            *http.chamadas.borrow(),
            1,
            "não deve chamar a rede de novo com o arquivo já em disco"
        );
    }

    #[test]
    fn cria_o_diretorio_de_destino_quando_nao_existe() {
        let dir = tempdir().expect("cria tempdir");
        let dir_destino = dir.path().join("1998547");
        assert!(!dir_destino.exists());

        let http = FakeHttp::novo(PDF_FALSO);
        let nome = baixar_documento(&http, &doc_fake(), &dir_destino).expect("deve criar e baixar");

        assert!(dir_destino.join(nome).is_file());
    }

    // --- caminho_seguro (R7) ------------------------------------------------ //

    #[test]
    fn monta_caminho_para_nome_limpo() {
        let base = Path::new("/dados/documentos");
        let caminho = caminho_seguro(base, "1998547", "2024_1_Assunto.pdf").expect("nome válido");
        assert_eq!(caminho, base.join("1998547").join("2024_1_Assunto.pdf"));
    }

    #[test]
    fn rejeita_traversal_e_separadores_de_caminho() {
        let base = Path::new("/dados/documentos");
        for arquivo in ["..", "../../etc/passwd", "/etc/x", "a/b", "a\\b", ""] {
            let erro = caminho_seguro(base, "1998547", arquivo)
                .expect_err(&format!("deveria rejeitar '{arquivo}'"));
            assert!(matches!(erro, AppError::FalhaArquivo { .. }));
        }
    }

    #[test]
    fn rejeita_siape_invalido_antes_de_montar_o_caminho() {
        let base = Path::new("/dados/documentos");
        let erro = caminho_seguro(base, "abc", "arquivo.pdf").unwrap_err();
        assert!(matches!(erro, AppError::SiapeInvalido { .. }));
    }
}
