//! Download de documentos (US4) — grava PDFs no diretório de dados do app,
//! **fora do repositório** (Princípio II/LGPD, R7: PDFs contêm PII de
//! terceiros e nunca podem ser versionados). Reaproveita o nome
//! determinístico de `domain::nome_arquivo` (R3); a resolução do diretório
//! real (`app_data_dir`) é responsabilidade de `commands::documento`, que
//! injeta o `HttpPort` e o diretório aqui — nenhum teste deste módulo toca
//! rede ou o disco do repositório (Princípio VII): a rede é um `HttpPort`
//! dublê e o disco é um `tempfile::TempDir`.

use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::domain::documento::Documento;
use crate::domain::nome_arquivo::nome_arquivo;
use crate::domain::siape;
use crate::error::AppError;
use crate::ports::http::HttpPort;

/// Host único de onde é permitido baixar documentos (Princípio I — fidelidade
/// à fonte oficial). Barra SSRF: o `link` chega pela fronteira IPC e não pode
/// apontar para um host arbitrário.
const HOST_CONFIAVEL: &str = "gedoc.ifes.edu.br";

/// Assinatura de arquivo PDF — todo PDF válido começa com `%PDF`.
const ASSINATURA_PDF: &[u8] = b"%PDF";

/// Baixa `doc.link` (via `http`) para dentro de `dir_destino`, criando o
/// diretório se necessário, e grava o PDF com o nome determinístico de R3.
/// **Idempotente**: se o arquivo final já existe em `dir_destino`, não baixa
/// de novo — apenas retorna o nome. A escrita é **atômica** (grava em
/// `<nome>.part` e renomeia) para que um crash no meio nunca deixe um arquivo
/// final parcial que passaria a ser tratado como "já baixado". Retorna só o
/// **nome** do arquivo (nunca o caminho absoluto — R7).
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

    if !link_do_portal(&doc.link) {
        return Err(AppError::FalhaPortal {
            motivo: "Link fora do portal GeDoc — download recusado.".to_string(),
        });
    }

    let bytes = http.get_bytes(&doc.link)?;
    validar_conteudo_pdf(&bytes)?;

    fs::create_dir_all(dir_destino).map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Falha ao criar o diretório de dados: {e}"),
    })?;

    // Escrita atômica: grava no `.part` e só renomeia após o corpo completo.
    // `rename` é atômico no mesmo filesystem — o nome final só passa a existir
    // quando o download terminou por inteiro.
    let temporario = dir_destino.join(format!("{nome}.part"));
    fs::write(&temporario, &bytes).map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Falha ao gravar '{nome}': {e}"),
    })?;
    fs::rename(&temporario, &caminho).map_err(|e| {
        let _ = fs::remove_file(&temporario); // limpa o `.part` órfão
        AppError::FalhaArquivo {
            motivo: format!("Falha ao finalizar '{nome}': {e}"),
        }
    })?;

    Ok(nome)
}

/// Verdadeiro se `link` aponta para o host oficial do portal (http/https,
/// porta opcional). Comparação de host só; caminho/query são irrelevantes.
fn link_do_portal(link: &str) -> bool {
    let sem_esquema = link
        .strip_prefix("https://")
        .or_else(|| link.strip_prefix("http://"));
    match sem_esquema {
        Some(resto) => {
            let host = resto.split(['/', ':', '?', '#']).next().unwrap_or("");
            host.eq_ignore_ascii_case(HOST_CONFIAVEL)
        }
        None => false,
    }
}

/// Rejeita corpo vazio ou que não seja um PDF (ex.: uma página de sessão
/// expirada devolvida com status 200), evitando persistir lixo que a
/// idempotência passaria a tratar como "já baixado".
fn validar_conteudo_pdf(bytes: &[u8]) -> Result<(), AppError> {
    if bytes.starts_with(ASSINATURA_PDF) {
        Ok(())
    } else {
        Err(AppError::FalhaArquivo {
            motivo: "Conteúdo baixado não é um PDF válido (resposta vazia ou inesperada)."
                .to_string(),
        })
    }
}

/// R7 — resolve `base/siape/arquivo`, validando que `siape` é uma matrícula
/// válida (R10) e que `arquivo` é um único componente de caminho normal (sem
/// separadores, sem `..`/`.`, sem raiz nem prefixo de unidade). Função pura,
/// não toca o disco.
pub fn caminho_seguro(base: &Path, siape: &str, arquivo: &str) -> Result<PathBuf, AppError> {
    siape::validar(siape)?;

    if !nome_arquivo_seguro(arquivo) {
        return Err(AppError::FalhaArquivo {
            motivo: format!("Nome de arquivo inválido: '{arquivo}'"),
        });
    }

    Ok(base.join(siape).join(arquivo))
}

/// Verdadeiro sse `arquivo` é seguro como último componente de caminho:
/// não-vazio, sem `/`, `\` ou `:` (fecha o prefixo de unidade do Windows,
/// ex.: `C:evil.pdf`, que `Path::push` trataria como caminho absoluto e
/// escaparia do diretório de destino), e exatamente **um** componente
/// `Normal` (fecha `..`, `.`, raiz e prefixo). Checar componentes de `Path`
/// (em vez de só substrings) é o que torna a validação robusta em todas as
/// plataformas.
fn nome_arquivo_seguro(arquivo: &str) -> bool {
    if arquivo.is_empty() || arquivo.contains(['/', '\\', ':']) {
        return false;
    }
    let mut comps = Path::new(arquivo).components();
    matches!(
        (comps.next(), comps.next()),
        (Some(Component::Normal(c)), None) if c == OsStr::new(arquivo)
    )
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

    #[test]
    fn nao_grava_arquivo_final_quando_corpo_nao_e_pdf() {
        let dir = tempdir().expect("cria tempdir");
        let http = FakeHttp::novo(b""); // corpo vazio (ex.: sessão expirada em 200)
        let doc = doc_fake();

        let erro = baixar_documento(&http, &doc, dir.path()).unwrap_err();

        assert!(matches!(erro, AppError::FalhaArquivo { .. }));
        assert!(
            !dir.path().join(nome_arquivo(&doc)).exists(),
            "corpo inválido não pode deixar arquivo final"
        );
    }

    #[test]
    fn part_orfao_nao_bloqueia_nem_e_confundido_com_o_final() {
        let dir = tempdir().expect("cria tempdir");
        let doc = doc_fake();
        let nome = nome_arquivo(&doc);
        // simula um `.part` deixado por um crash anterior
        fs::write(dir.path().join(format!("{nome}.part")), b"parcial").expect("cria .part");

        let http = FakeHttp::novo(PDF_FALSO);
        let baixado =
            baixar_documento(&http, &doc, dir.path()).expect("deve baixar mesmo com .part");

        assert_eq!(baixado, nome);
        assert_eq!(*http.chamadas.borrow(), 1, ".part não conta como baixado");
        assert_eq!(fs::read(dir.path().join(&nome)).unwrap(), PDF_FALSO);
    }

    #[test]
    fn recusa_link_fora_do_portal_sem_tocar_a_rede() {
        let dir = tempdir().expect("cria tempdir");
        let http = FakeHttp::novo(PDF_FALSO);
        let mut doc = doc_fake();
        doc.link = "https://evil.example.com/documento/x?inline".to_string();

        let erro = baixar_documento(&http, &doc, dir.path()).unwrap_err();

        assert!(matches!(erro, AppError::FalhaPortal { .. }));
        assert_eq!(
            *http.chamadas.borrow(),
            0,
            "não deve baixar de host arbitrário"
        );
    }

    #[test]
    fn link_do_portal_aceita_host_oficial_e_recusa_o_resto() {
        assert!(link_do_portal(
            "https://gedoc.ifes.edu.br/documento/aaaa?inline"
        ));
        assert!(link_do_portal("http://gedoc.ifes.edu.br:80/documento/aaaa"));
        assert!(!link_do_portal("https://evil.com/documento/aaaa"));
        assert!(!link_do_portal("https://gedoc.ifes.edu.br.evil.com/x"));
        assert!(!link_do_portal("ftp://gedoc.ifes.edu.br/x"));
        assert!(!link_do_portal("/documento/aaaa"));
    }

    // --- caminho_seguro (R7) ------------------------------------------------ //

    #[test]
    fn monta_caminho_para_nome_limpo() {
        let base = Path::new("/dados/documentos");
        let caminho = caminho_seguro(base, "1998547", "2024_1_Assunto.pdf").expect("nome válido");
        assert_eq!(caminho, base.join("1998547").join("2024_1_Assunto.pdf"));
    }

    #[test]
    fn aceita_nome_com_pontos_internos_sem_ser_traversal() {
        // `nome_arquivo` pode produzir nomes com ".." no assunto; isso não é
        // path traversal e não pode ser rejeitado (regressão).
        let base = Path::new("/dados/documentos");
        let caminho = caminho_seguro(base, "1998547", "2024_1_Assunto .. final.pdf")
            .expect("pontos internos são válidos");
        assert_eq!(
            caminho,
            base.join("1998547").join("2024_1_Assunto .. final.pdf")
        );
    }

    #[test]
    fn rejeita_traversal_separadores_e_prefixo_de_unidade() {
        let base = Path::new("/dados/documentos");
        for arquivo in [
            "..",
            ".",
            "../../etc/passwd",
            "/etc/x",
            "a/b",
            "a\\b",
            "foo/../bar",
            "C:evil.pdf", // prefixo de unidade Windows — Path::push escaparia
            "C:",
            "",
        ] {
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
