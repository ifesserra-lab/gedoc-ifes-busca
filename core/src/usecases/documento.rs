//! Use-cases de download/abertura de PDF (US4). Núcleos síncronos e testáveis:
//! recebem `HttpPort`/diretório base por parâmetro — sem tocar rede real nem
//! saber ONDE fica o diretório de dados (isso é da borda). PDFs contêm PII de
//! terceiros (Princípio II/LGPD, R7): sempre sob o diretório passado.

use std::path::{Path, PathBuf};

use crate::domain::documento::Documento;
use crate::domain::siape;
use crate::dto::{AbrirDocumentoInput, BaixarDocumentoInput};
use crate::error::AppError;
use crate::ports::http::HttpPort;
use crate::services::downloader;

/// Valida o SIAPE (R10), monta o `Documento` mínimo e delega o download a
/// `services::downloader`, dentro de `dir_base/<siape>/`. Retorna só o nome do
/// arquivo gravado (nunca o caminho absoluto, R7).
pub fn executar_download<H: HttpPort>(
    http: &H,
    dir_base: &Path,
    input: &BaixarDocumentoInput,
) -> Result<String, AppError> {
    siape::validar(&input.siape)?;

    let mut doc = Documento::novo(input.link.clone(), input.titulo.clone());
    doc.data = input.data.clone();

    let dir_destino = dir_base.join(&input.siape);
    downloader::baixar_documento(http, &doc, &dir_destino)
}

/// Sanitiza `siape`/`arquivo` (R7, via `downloader::caminho_seguro`) e confere
/// que o arquivo existe. Não abre nada — abrir/servir é da borda.
pub fn resolver_caminho_abertura(
    dir_base: &Path,
    input: &AbrirDocumentoInput,
) -> Result<PathBuf, AppError> {
    let caminho = downloader::caminho_seguro(dir_base, &input.siape, &input.arquivo)?;
    if !caminho.is_file() {
        return Err(AppError::FalhaArquivo {
            motivo: format!("Arquivo não encontrado: '{}'", input.arquivo),
        });
    }
    Ok(caminho)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fs;

    use tempfile::tempdir;

    use super::*;

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
            unreachable!("executar_download não usa get()")
        }

        fn post_form(&self, _url: &str, _campos: &[(String, String)]) -> Result<String, AppError> {
            unreachable!("executar_download não usa post_form()")
        }

        fn get_bytes(&self, _url: &str) -> Result<Vec<u8>, AppError> {
            *self.chamadas.borrow_mut() += 1;
            Ok(self.bytes.clone())
        }
    }

    const PDF_FALSO: &[u8] = b"%PDF-1.4 fake";
    const SIAPE_FICTICIO: &str = "1998547";

    fn input_download() -> BaixarDocumentoInput {
        BaixarDocumentoInput {
            siape: SIAPE_FICTICIO.to_string(),
            link: "https://gedoc.ifes.edu.br/documento/aaaa?inline".to_string(),
            titulo: "PORTARIA Nº 9 - 2024 - Designação de função".to_string(),
            data: None,
        }
    }

    #[test]
    fn rejeita_siape_invalido_sem_tocar_http_ou_disco() {
        let dir = tempdir().expect("cria tempdir");
        let http = FakeHttp::novo(PDF_FALSO);
        let mut input = input_download();
        input.siape = "abc".to_string();

        let erro = executar_download(&http, dir.path(), &input).unwrap_err();

        assert!(matches!(erro, AppError::SiapeInvalido { .. }));
        assert_eq!(*http.chamadas.borrow(), 0);
    }

    #[test]
    fn baixa_o_documento_dentro_da_pasta_do_siape() {
        let dir = tempdir().expect("cria tempdir");
        let http = FakeHttp::novo(PDF_FALSO);
        let input = input_download();

        let nome = executar_download(&http, dir.path(), &input).expect("deve baixar");

        let caminho = dir.path().join(SIAPE_FICTICIO).join(&nome);
        assert_eq!(fs::read(&caminho).expect("arquivo deve existir"), PDF_FALSO);
    }

    #[test]
    fn e_idempotente_atraves_do_comando() {
        let dir = tempdir().expect("cria tempdir");
        let http = FakeHttp::novo(PDF_FALSO);
        let input = input_download();

        executar_download(&http, dir.path(), &input).expect("1ª chamada baixa");
        executar_download(&http, dir.path(), &input).expect("2ª chamada é idempotente");

        assert_eq!(*http.chamadas.borrow(), 1);
    }

    #[test]
    fn resolve_o_caminho_quando_arquivo_existe() {
        let dir = tempdir().expect("cria tempdir");
        let pasta_siape = dir.path().join(SIAPE_FICTICIO);
        fs::create_dir_all(&pasta_siape).expect("cria pasta do siape");
        fs::write(pasta_siape.join("2024_1_Assunto.pdf"), PDF_FALSO).expect("grava arquivo fake");

        let input = AbrirDocumentoInput {
            siape: SIAPE_FICTICIO.to_string(),
            arquivo: "2024_1_Assunto.pdf".to_string(),
        };

        let caminho = resolver_caminho_abertura(dir.path(), &input).expect("deve resolver");
        assert_eq!(caminho, pasta_siape.join("2024_1_Assunto.pdf"));
    }

    #[test]
    fn erro_claro_quando_arquivo_nao_existe() {
        let dir = tempdir().expect("cria tempdir");
        let input = AbrirDocumentoInput {
            siape: SIAPE_FICTICIO.to_string(),
            arquivo: "inexistente.pdf".to_string(),
        };

        let erro = resolver_caminho_abertura(dir.path(), &input).unwrap_err();
        assert!(matches!(erro, AppError::FalhaArquivo { .. }));
    }

    #[test]
    fn rejeita_arquivo_com_path_traversal() {
        let dir = tempdir().expect("cria tempdir");
        let input = AbrirDocumentoInput {
            siape: SIAPE_FICTICIO.to_string(),
            arquivo: "../../etc/passwd".to_string(),
        };

        let erro = resolver_caminho_abertura(dir.path(), &input).unwrap_err();
        assert!(matches!(erro, AppError::FalhaArquivo { .. }));
    }
}
