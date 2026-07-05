//! Comandos `baixar_documento` e `abrir_documento` (US4) — ver
//! `contracts/ipc-commands.md`. PDFs baixados contêm PII de terceiros
//! (Princípio II/LGPD, R7): ficam sempre sob o diretório de dados do app
//! (`AppHandle.path().app_data_dir()`), nunca no repositório.
//!
//! `executar_download`/`resolver_caminho_abertura` são os núcleos
//! síncronos e testáveis (recebem `HttpPort`/diretório base por parâmetro,
//! sem tocar rede real nem `AppHandle` — Princípio VII); os
//! `#[tauri::command]` são a fronteira async/IPC que resolve o diretório
//! real e aciona o download (bloqueante, em `spawn_blocking`) e o plugin
//! `opener`.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::domain::documento::Documento;
use crate::domain::siape;
use crate::error::AppError;
use crate::ports::http::{HttpPort, ReqwestHttp};
use crate::services::downloader;

/// Subpasta, dentro do diretório de dados do app, onde os PDFs baixados
/// ficam organizados por SIAPE (`<app_data_dir>/documentos/<siape>/`).
const SUBPASTA_DOCUMENTOS: &str = "documentos";

#[derive(Debug, Deserialize)]
pub struct BaixarDocumentoInput {
    pub siape: String,
    pub link: String,
    pub titulo: String,
    pub data: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AbrirDocumentoInput {
    pub siape: String,
    pub arquivo: String,
}

/// Núcleo síncrono e testável de `baixar_documento`: valida o SIAPE (R10),
/// monta o `Documento` mínimo a partir do input e delega o download a
/// `services::downloader`, dentro de `dir_base/<siape>/`.
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

/// Núcleo puro e testável de `abrir_documento`: sanitiza `siape`/`arquivo`
/// (R7, via `downloader::caminho_seguro`) e confere que o arquivo existe em
/// disco. Não abre nada — abrir de fato é responsabilidade exclusiva do
/// comando (plugin `opener`), fora deste núcleo.
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

/// `<app_data_dir>/documentos` — raiz de todos os downloads (fora do VCS,
/// Princípio II/LGPD). Único ponto que conhece o `AppHandle`; todo o resto
/// deste módulo recebe o diretório já resolvido, o que o mantém testável.
fn dir_documentos(app: &AppHandle) -> Result<PathBuf, AppError> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::FalhaArquivo {
            motivo: format!("Falha ao localizar o diretório de dados do app: {e}"),
        })?;
    Ok(base.join(SUBPASTA_DOCUMENTOS))
}

/// Baixa o PDF de um documento (US4). Valida o SIAPE antes de tocar
/// qualquer I/O; a chamada de rede/disco (bloqueante) roda em
/// `spawn_blocking` para não travar o runtime async do Tauri. Retorna
/// apenas o nome do arquivo gravado (R7 — nunca o caminho absoluto).
#[tauri::command]
pub async fn baixar_documento(
    app: AppHandle,
    input: BaixarDocumentoInput,
) -> Result<String, AppError> {
    siape::validar(&input.siape)?;
    let dir_base = dir_documentos(&app)?;

    tokio::task::spawn_blocking(move || {
        let http = ReqwestHttp::novo()?;
        executar_download(&http, &dir_base, &input)
    })
    .await
    .map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Falha interna ao baixar o documento: {e}"),
    })?
}

/// Abre, com o aplicativo padrão do sistema, um PDF já baixado (US4).
/// `arquivo` é sanitizado (R7) antes de qualquer acesso a disco; o comando
/// nunca expõe o caminho absoluto de volta ao cliente. Usa a API Rust do
/// plugin `opener` (`OpenerExt`) diretamente — o comando do próprio plugin
/// não é exposto ao frontend (nenhuma permissão `opener:*` na capability),
/// então a única porta de entrada para abrir arquivos é este comando, com a
/// sanitização acima sempre aplicada.
#[tauri::command]
pub fn abrir_documento(app: AppHandle, input: AbrirDocumentoInput) -> Result<(), AppError> {
    let dir_base = dir_documentos(&app)?;
    let caminho = resolver_caminho_abertura(&dir_base, &input)?;

    app.opener()
        .open_path(caminho.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|e| AppError::FalhaArquivo {
            motivo: format!("Falha ao abrir o documento: {e}"),
        })
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

    // --- executar_download -------------------------------------------------- //

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

    // --- resolver_caminho_abertura ------------------------------------------- //

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
