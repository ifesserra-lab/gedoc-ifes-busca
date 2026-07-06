//! Empacotamento em ZIP dos PDFs já baixados (US7) — `commands::exportar::
//! baixar_zip` monta `<app_data_dir>/documentos/<siape>/*.pdf` num único
//! `.zip` em `<app_data_dir>/relatorios/`. `montar_zip` recebe os diretórios
//! já resolvidos (nenhum `AppHandle`), então nenhum dublê de teste toca disco
//! fora de um `tempfile::TempDir` (Princípio VII).

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::error::AppError;

/// Monta, em `saida`, um ZIP com todo `*.pdf` (case-insensitive) direto sob
/// `dir_siape` (sem recursão em subpastas). Entradas em ordem determinística
/// (nome do arquivo) para o ZIP resultante ser reprodutível. `dir_siape`
/// ausente ou sem nenhum PDF é tratado da mesma forma — erro claro em vez de
/// um ZIP vazio silencioso (o usuário pediria o download de novo sem saber
/// por quê o ZIP "sumiu" o conteúdo). Retorna a quantidade de arquivos
/// incluídos.
pub fn montar_zip(dir_siape: &Path, saida: &Path) -> Result<usize, AppError> {
    let mut pdfs = listar_pdfs(dir_siape)?;
    pdfs.sort();

    if pdfs.is_empty() {
        return Err(AppError::FalhaArquivo {
            motivo:
                "Nenhum PDF baixado para este SIAPE ainda — baixe ao menos um documento antes de gerar o ZIP."
                    .to_string(),
        });
    }

    if let Some(pai) = saida.parent() {
        fs::create_dir_all(pai).map_err(|e| AppError::FalhaArquivo {
            motivo: format!("Falha ao criar o diretório de saída do ZIP: {e}"),
        })?;
    }

    let arquivo_zip = File::create(saida).map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Falha ao criar '{}': {e}", saida.display()),
    })?;
    let mut zip = ZipWriter::new(arquivo_zip);
    let opcoes = SimpleFileOptions::default();

    for caminho in &pdfs {
        // `listar_pdfs` só devolve caminhos com `file_name()` válido (ver
        // abaixo), então este `unwrap_or` nunca deveria disparar na prática;
        // mantido só como salvaguarda contra panic (nunca `unwrap` direto).
        let nome = caminho
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("documento.pdf");

        zip.start_file(nome, opcoes)
            .map_err(|e| AppError::FalhaArquivo {
                motivo: format!("Falha ao adicionar '{nome}' ao ZIP: {e}"),
            })?;

        let bytes = fs::read(caminho).map_err(|e| AppError::FalhaArquivo {
            motivo: format!("Falha ao ler '{nome}': {e}"),
        })?;
        zip.write_all(&bytes).map_err(|e| AppError::FalhaArquivo {
            motivo: format!("Falha ao gravar '{nome}' no ZIP: {e}"),
        })?;
    }

    zip.finish().map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Falha ao finalizar o ZIP: {e}"),
    })?;

    Ok(pdfs.len())
}

/// Lista os `*.pdf` (extensão case-insensitive) diretamente sob `dir`, sem
/// recursão. `dir` inexistente devolve uma lista vazia (não é um erro em si
/// — `montar_zip` decide o que fazer com "nenhum PDF").
fn listar_pdfs(dir: &Path) -> Result<Vec<PathBuf>, AppError> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let entradas = fs::read_dir(dir).map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Falha ao ler '{}': {e}", dir.display()),
    })?;

    let mut pdfs = Vec::new();
    for entrada in entradas {
        let entrada = entrada.map_err(|e| AppError::FalhaArquivo {
            motivo: format!("Falha ao ler entrada de diretório: {e}"),
        })?;
        let caminho = entrada.path();
        let eh_pdf = caminho
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"));
        if caminho.is_file() && eh_pdf {
            pdfs.push(caminho);
        }
    }
    Ok(pdfs)
}

#[cfg(test)]
mod tests {
    use std::io::Read as _;

    use tempfile::tempdir;
    use zip::ZipArchive;

    use super::*;

    fn escrever(dir: &Path, nome: &str, conteudo: &[u8]) {
        fs::write(dir.join(nome), conteudo).expect("grava arquivo de teste");
    }

    #[test]
    fn monta_zip_com_todos_os_pdfs_do_diretorio() {
        let dir = tempdir().expect("tempdir");
        let dir_siape = dir.path().join("1998547");
        fs::create_dir_all(&dir_siape).expect("cria pasta do siape");
        escrever(&dir_siape, "2024_1_A.pdf", b"%PDF-1.4 A");
        escrever(&dir_siape, "2024_2_B.pdf", b"%PDF-1.4 B");
        escrever(&dir_siape, "notas.txt", b"nao e PDF, nao deve entrar");

        let saida = dir.path().join("saida").join("1998547_documentos.zip");
        let qtd = montar_zip(&dir_siape, &saida).expect("deve montar o zip");

        assert_eq!(qtd, 2);
        assert!(saida.is_file());

        let arquivo = File::open(&saida).expect("abre o zip gerado");
        let mut zip = ZipArchive::new(arquivo).expect("lê o zip gerado");
        assert_eq!(zip.len(), 2);

        let mut nomes: Vec<String> = (0..zip.len())
            .map(|i| zip.by_index(i).expect("entrada do zip").name().to_string())
            .collect();
        nomes.sort();
        assert_eq!(nomes, vec!["2024_1_A.pdf", "2024_2_B.pdf"]);
    }

    #[test]
    fn conteudo_de_cada_entrada_bate_com_o_arquivo_original() {
        let dir = tempdir().expect("tempdir");
        let dir_siape = dir.path().join("1998547");
        fs::create_dir_all(&dir_siape).expect("cria pasta do siape");
        escrever(&dir_siape, "2024_1_A.pdf", b"%PDF-1.4 conteudo-A");

        let saida = dir.path().join("1998547_documentos.zip");
        montar_zip(&dir_siape, &saida).expect("deve montar o zip");

        let arquivo = File::open(&saida).expect("abre o zip gerado");
        let mut zip = ZipArchive::new(arquivo).expect("lê o zip gerado");
        let mut entrada = zip.by_name("2024_1_A.pdf").expect("entrada existe");
        let mut conteudo = Vec::new();
        entrada.read_to_end(&mut conteudo).expect("lê o conteúdo");

        assert_eq!(conteudo, b"%PDF-1.4 conteudo-A");
    }

    #[test]
    fn diretorio_sem_nenhum_pdf_e_erro_claro_nao_zip_vazio() {
        let dir = tempdir().expect("tempdir");
        let dir_siape = dir.path().join("1998547");
        fs::create_dir_all(&dir_siape).expect("cria pasta do siape");
        escrever(&dir_siape, "notas.txt", b"sem PDFs aqui");

        let saida = dir.path().join("1998547_documentos.zip");
        let erro = montar_zip(&dir_siape, &saida).unwrap_err();

        assert!(matches!(erro, AppError::FalhaArquivo { .. }));
        assert!(
            !saida.exists(),
            "não deve deixar um zip parcial/vazio para trás"
        );
    }

    #[test]
    fn diretorio_do_siape_inexistente_e_o_mesmo_erro_claro() {
        let dir = tempdir().expect("tempdir");
        let dir_siape = dir.path().join("1998547"); // nunca criado
        let saida = dir.path().join("1998547_documentos.zip");

        let erro = montar_zip(&dir_siape, &saida).unwrap_err();

        assert!(matches!(erro, AppError::FalhaArquivo { .. }));
    }

    #[test]
    fn entradas_ficam_em_ordem_deterministica_r3() {
        let dir = tempdir().expect("tempdir");
        let dir_siape = dir.path().join("1998547");
        fs::create_dir_all(&dir_siape).expect("cria pasta do siape");
        escrever(&dir_siape, "2024_9_Z.pdf", b"%PDF-1.4 Z");
        escrever(&dir_siape, "2024_1_A.pdf", b"%PDF-1.4 A");

        let saida = dir.path().join("1998547_documentos.zip");
        montar_zip(&dir_siape, &saida).expect("deve montar o zip");

        let arquivo = File::open(&saida).expect("abre o zip gerado");
        let mut zip = ZipArchive::new(arquivo).expect("lê o zip gerado");
        let primeiro = zip
            .by_index(0)
            .expect("primeira entrada")
            .name()
            .to_string();
        assert_eq!(primeiro, "2024_1_A.pdf", "ordem alfabética, não de criação");
    }

    #[test]
    fn extensao_pdf_maiuscula_tambem_e_incluida() {
        let dir = tempdir().expect("tempdir");
        let dir_siape = dir.path().join("1998547");
        fs::create_dir_all(&dir_siape).expect("cria pasta do siape");
        escrever(&dir_siape, "2024_1_A.PDF", b"%PDF-1.4 A");

        let saida = dir.path().join("1998547_documentos.zip");
        let qtd = montar_zip(&dir_siape, &saida).expect("deve montar o zip");

        assert_eq!(qtd, 1);
    }
}
