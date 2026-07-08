//! Use-case de geração do relatório consolidado (US7). Núcleo síncrono e
//! testável: recebe o diretório de saída já resolvido (sem saber ONDE fica) e
//! devolve o nome do HTML gerado (nunca caminho absoluto, R7). O empacotamento
//! ZIP usa `services::empacotador::montar_zip` direto (já testável), então não
//! precisa de use-case próprio.

use std::fs;
use std::path::Path;

use crate::domain::siape;
use crate::dto::ResultadoView;
use crate::error::AppError;
use crate::services::relatorio;

/// Gera Markdown + HTML a partir da `ResultadoView` já pronta (US3/US5/US6) e
/// grava os dois em `dir_saida` (criando-o se necessário). Nomes
/// determinísticos (R3): `<siape>_relatorio.{md,html}` — sobrescreve a versão
/// anterior (R1). Retorna só o nome do HTML.
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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::dto::{CategoriaGrupo, DocView};
    use std::fs;

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
