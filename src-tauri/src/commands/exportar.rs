//! Comandos `gerar_relatorio`/`baixar_zip` (US7) — ver
//! `contracts/ipc-commands.md`. Relatório (HTML+MD) e ZIP contêm PII de
//! terceiros (Princípio II/LGPD, R7): gravados sempre sob
//! `AppHandle.path().app_data_dir()/relatorios/`, nunca no repositório.
//!
//! `executar_gerar_relatorio` é o núcleo síncrono e testável (recebe o
//! diretório de saída já resolvido, sem `AppHandle` — Princípio VII); os
//! `#[tauri::command]` são a fronteira async/IPC que resolvem os diretórios
//! reais, rodam a I/O bloqueante em `spawn_blocking` e acionam o plugin
//! `opener` (abre o relatório HTML; revela o ZIP no gerenciador de arquivos
//! do SO). `montar_zip` (US7, `services::empacotador`) já é testável por si
//! só com `tempfile`, então `baixar_zip` não precisa de um núcleo síncrono
//! próprio além de resolver os dois diretórios.

use std::fs;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::commands::buscar::ResultadoView;
use crate::commands::documento;
use crate::domain::siape;
use crate::error::AppError;
use crate::services::{empacotador, relatorio};

/// Subpasta, dentro do diretório de dados do app, onde o relatório
/// (HTML+MD) e o ZIP de documentos ficam — fora do VCS (Princípio II/LGPD).
const SUBPASTA_RELATORIOS: &str = "relatorios";

/// `<app_data_dir>/relatorios` — único ponto deste módulo que conhece o
/// `AppHandle` para este diretório; o resto recebe o caminho já resolvido.
fn dir_relatorios(app: &AppHandle) -> Result<PathBuf, AppError> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::FalhaArquivo {
            motivo: format!("Falha ao localizar o diretório de dados do app: {e}"),
        })?;
    Ok(base.join(SUBPASTA_RELATORIOS))
}

/// Núcleo síncrono e testável de `gerar_relatorio`: gera o Markdown
/// (`services::relatorio::gerar_markdown`) e o HTML (`markdown_para_html`) a
/// partir da `ResultadoView` já pronta (US3/US5/US6) e grava os dois em
/// `dir_saida` (criando-o se necessário). Nome determinístico (R3, análogo a
/// `domain::nome_arquivo`): `<siape>_relatorio.{md,html}` — sempre o mesmo
/// para o mesmo SIAPE, sobrescrevendo a versão anterior (relatório reflete
/// sempre a busca mais recente, R1). Retorna só o **nome** do HTML (nunca o
/// caminho absoluto, R7).
pub fn executar_gerar_relatorio(
    resultado: &ResultadoView,
    dir_saida: &Path,
) -> Result<String, AppError> {
    siape::validar(&resultado.termo)?;

    fs::create_dir_all(dir_saida).map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Falha ao criar o diretório de relatórios: {e}"),
    })?;

    let md = relatorio::gerar_markdown(resultado);
    let titulo = format!("Relatório de documentos — SIAPE {}", resultado.termo);
    let html = relatorio::markdown_para_html(&md, &titulo);

    let nome_md = format!("{}_relatorio.md", resultado.termo);
    let nome_html = format!("{}_relatorio.html", resultado.termo);

    fs::write(dir_saida.join(&nome_md), md).map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Falha ao gravar '{nome_md}': {e}"),
    })?;
    fs::write(dir_saida.join(&nome_html), html).map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Falha ao gravar '{nome_html}': {e}"),
    })?;

    Ok(nome_html)
}

/// Gera o relatório consolidado (US7: Markdown + HTML self-contained — ver
/// decisão de PDF em `services::relatorio`) a partir do `ResultadoView` que a
/// View já tem em mãos (a mesma busca mostrada na tela, R1: reflete os
/// resumos reais) e abre o HTML com o app padrão do sistema (`opener`), onde
/// "Imprimir → Salvar como PDF" produz um PDF equivalente sem depender de
/// Chrome/binário externo. Retorna o nome do arquivo HTML gravado.
#[tauri::command]
pub async fn gerar_relatorio(app: AppHandle, resultado: ResultadoView) -> Result<String, AppError> {
    siape::validar(&resultado.termo)?;
    let dir_saida = dir_relatorios(&app)?;
    let dir_saida_task = dir_saida.clone();

    let nome =
        tokio::task::spawn_blocking(move || executar_gerar_relatorio(&resultado, &dir_saida_task))
            .await
            .map_err(|e| AppError::FalhaArquivo {
                motivo: format!("Falha interna ao gerar o relatório: {e}"),
            })??;

    let caminho = dir_saida.join(&nome);
    app.opener()
        .open_path(caminho.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|e| AppError::FalhaArquivo {
            motivo: format!("Falha ao abrir o relatório: {e}"),
        })?;

    Ok(nome)
}

/// Monta o ZIP dos PDFs já baixados para `siape` (US4:
/// `<app_data_dir>/documentos/<siape>/*.pdf`) em
/// `<app_data_dir>/relatorios/<siape>_documentos.zip` (R3: nome
/// determinístico) e revela o arquivo no gerenciador de arquivos do SO
/// (`opener`, "mostrar no Finder/Explorer" — diferente do relatório, um ZIP
/// não tem uma boa experiência de "abrir direto"). Sem nenhum PDF baixado
/// ainda, `services::empacotador::montar_zip` devolve um erro amigável em vez
/// de um ZIP vazio (ver doc daquele módulo). Retorna só o **nome** do ZIP
/// (nunca o caminho absoluto, R7).
#[tauri::command]
pub async fn baixar_zip(app: AppHandle, siape: String) -> Result<String, AppError> {
    siape::validar(&siape)?;

    let dir_documentos = documento::dir_documentos(&app)?;
    let dir_saida = dir_relatorios(&app)?;
    let dir_saida_task = dir_saida.clone();

    let nome = tokio::task::spawn_blocking(move || -> Result<String, AppError> {
        let dir_siape = dir_documentos.join(&siape);
        let nome_zip = format!("{siape}_documentos.zip");
        let caminho_zip = dir_saida_task.join(&nome_zip);
        empacotador::montar_zip(&dir_siape, &caminho_zip)?;
        Ok(nome_zip)
    })
    .await
    .map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Falha interna ao montar o ZIP: {e}"),
    })??;

    let caminho = dir_saida.join(&nome);
    // Best-effort: se o SO não conseguir revelar o item (ex.: gerenciador de
    // arquivos indisponível), o ZIP já foi gravado com sucesso — não faz
    // sentido falhar o comando inteiro por causa disso.
    let _ = app.opener().reveal_item_in_dir(&caminho);

    Ok(nome)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::commands::buscar::{CategoriaGrupo, DocView};

    fn resultado_fake() -> ResultadoView {
        ResultadoView {
            termo: "1998547".to_string(),
            total: 1,
            categorias: vec![CategoriaGrupo {
                categoria: "Progressão".to_string(),
                qtd: 1,
                itens: vec![DocView {
                    titulo: "PORTARIA Nº 1 - 2024 - Progressão".to_string(),
                    data: Some("10/01/2024".to_string()),
                    link: "https://gedoc.ifes.edu.br/documento/aaaa?inline".to_string(),
                    arquivo: Some("2024_1_Progressao.pdf".to_string()),
                    resumo: Some("Determina a progressão do servidor.".to_string()),
                }],
            }],
            tem_pdf: false,
        }
    }

    #[test]
    fn rejeita_siape_invalido_sem_tocar_disco() {
        let dir = tempdir().expect("tempdir");
        let mut resultado = resultado_fake();
        resultado.termo = "abc".to_string();

        let erro = executar_gerar_relatorio(&resultado, dir.path()).unwrap_err();

        assert!(matches!(erro, AppError::SiapeInvalido { .. }));
        assert!(
            fs::read_dir(dir.path())
                .expect("dir existe")
                .next()
                .is_none(),
            "não deve gravar nada com SIAPE inválido"
        );
    }

    #[test]
    fn gera_o_html_e_o_markdown_com_nomes_deterministicos_r3() {
        let dir = tempdir().expect("tempdir");
        let resultado = resultado_fake();

        let nome = executar_gerar_relatorio(&resultado, dir.path()).expect("deve gerar");

        assert_eq!(nome, "1998547_relatorio.html");
        assert!(dir.path().join("1998547_relatorio.html").is_file());
        assert!(dir.path().join("1998547_relatorio.md").is_file());
    }

    #[test]
    fn o_html_gerado_reflete_o_resumo_real_da_resultado_view_r1() {
        let dir = tempdir().expect("tempdir");
        let resultado = resultado_fake();

        let nome = executar_gerar_relatorio(&resultado, dir.path()).expect("deve gerar");
        let html = fs::read_to_string(dir.path().join(&nome)).expect("lê o html gerado");

        assert!(html.contains("Determina a progressão do servidor."));
        assert!(html.contains("Progressão"));
    }

    #[test]
    fn regerar_o_mesmo_siape_sobrescreve_em_vez_de_acumular() {
        let dir = tempdir().expect("tempdir");
        let resultado = resultado_fake();

        executar_gerar_relatorio(&resultado, dir.path()).expect("1ª geração");
        executar_gerar_relatorio(&resultado, dir.path()).expect("2ª geração");

        let entradas: Vec<_> = fs::read_dir(dir.path())
            .expect("lê dir")
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(entradas.len(), 2, "só md+html, sem duplicar por geração");
    }
}
