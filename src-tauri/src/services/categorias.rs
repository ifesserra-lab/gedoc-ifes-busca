//! Carrega e persiste `categoria.json` (R5 — categorias vêm de configuração,
//! não de código: Princípio IV). Formato:
//! `{ "categorias": [ { "nome": ..., "descricao": ... }, ... ] }`.
//!
//! `carregar_categorias`/`caminho_padrao` (US5) leem o `config/categoria.json`
//! versionado (a "semente"). `salvar_categorias` (US8) grava o arquivo que o
//! CRUD da tela de categorias edita — em produção isso é
//! `AppHandle.path().app_config_dir()/categoria.json`, nunca o arquivo
//! versionado (decisão e resolução do caminho ficam em `commands::categorias`,
//! que é quem conhece o `AppHandle`). `resolver_com_semente` é o elo entre os
//! dois: na primeira execução, se o arquivo do app_config ainda não existe,
//! copia a semente para lá; depois disso o arquivo do app_config passa a ser
//! a única fonte de verdade (edições do usuário nunca são sobrescritas).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::categoria::Categoria;
use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize)]
struct CategoriasArquivo {
    categorias: Vec<Categoria>,
}

/// Lê e valida `caminho`: erro claro (nunca panic) se o arquivo estiver
/// ausente, malformado ou sem nenhuma categoria — uma lista vazia por engano
/// faria todo documento cair em "Outros" silenciosamente.
pub fn carregar_categorias(caminho: &Path) -> Result<Vec<Categoria>, AppError> {
    let conteudo = fs::read_to_string(caminho).map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Não foi possível ler '{}': {e}", caminho.display()),
    })?;
    let arquivo: CategoriasArquivo =
        serde_json::from_str(&conteudo).map_err(|e| AppError::FalhaArquivo {
            motivo: format!(
                "'{}' não é um JSON válido de categorias: {e}",
                caminho.display()
            ),
        })?;
    if arquivo.categorias.is_empty() {
        return Err(AppError::FalhaArquivo {
            motivo: format!("'{}' não contém categorias.", caminho.display()),
        });
    }
    Ok(arquivo.categorias)
}

/// Caminho padrão de `config/categoria.json`, tentando alguns candidatos
/// relativos: o app pode rodar com cwd na raiz do repositório ou em
/// `src-tauri/` (mesma ambiguidade resolvida por `ports::ia::resolver_api_key`
/// para o `.env`). Não toca a rede; só verifica quais candidatos existem —
/// glue de runtime, não testado unitariamente (como `dir_documentos` em
/// `commands::documento`), pois depende do cwd real do processo.
pub fn caminho_padrao() -> PathBuf {
    for candidato in ["config/categoria.json", "../config/categoria.json"] {
        let caminho = PathBuf::from(candidato);
        if caminho.is_file() {
            return caminho;
        }
    }
    PathBuf::from("config/categoria.json")
}

/// R5: nome obrigatório (trim) e único (case-insensitive). Devolve a lista já
/// normalizada (nomes com espaços nas pontas removidos); não grava nada — só
/// valida, para `salvar_categorias` falhar rápido, antes de qualquer I/O.
///
/// Unicidade é só case-insensitive, sem dobra de acentuação (R5 literal): logo
/// "Férias" e "ferias" são categorias distintas. Diverge do casamento
/// accent-insensitive de `services::classificador` — se o usuário criar as duas,
/// a classificação por nome fica ambígua. Aceito por ora (exige o usuário
/// cadastrar deliberadamente as duas); alinhar caso vire requisito.
fn validar(categorias: &[Categoria]) -> Result<Vec<Categoria>, AppError> {
    let mut vistos: Vec<String> = Vec::with_capacity(categorias.len());
    let mut normalizadas = Vec::with_capacity(categorias.len());

    for categoria in categorias {
        let nome = categoria.nome.trim();
        if nome.is_empty() {
            return Err(AppError::CategoriaSemNome);
        }
        let chave = nome.to_lowercase();
        if vistos.contains(&chave) {
            return Err(AppError::NomeDuplicado {
                nome: nome.to_string(),
            });
        }
        vistos.push(chave);
        normalizadas.push(Categoria::nova(
            nome.to_string(),
            categoria.descricao.clone(),
        ));
    }

    Ok(normalizadas)
}

/// Persiste `categorias` em `caminho` (US8): valida R5 antes de tocar disco
/// (nada é escrito se inválido) e grava de forma atômica — serializa para
/// `caminho.part` e só então `rename` sobre o destino final, para nunca
/// deixar um arquivo truncado/corrompido em caso de falha no meio da escrita.
/// Devolve o total de categorias gravadas.
pub fn salvar_categorias(caminho: &Path, categorias: &[Categoria]) -> Result<usize, AppError> {
    let normalizadas = validar(categorias)?;

    if let Some(pai) = caminho.parent() {
        if !pai.as_os_str().is_empty() {
            fs::create_dir_all(pai).map_err(|e| AppError::FalhaArquivo {
                motivo: format!(
                    "Não foi possível criar o diretório '{}': {e}",
                    pai.display()
                ),
            })?;
        }
    }

    let total = normalizadas.len();
    let arquivo = CategoriasArquivo {
        categorias: normalizadas,
    };
    let json = serde_json::to_string_pretty(&arquivo).map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Falha ao serializar categorias: {e}"),
    })?;

    let mut nome_temporario = caminho.as_os_str().to_owned();
    nome_temporario.push(".part");
    let temporario = PathBuf::from(nome_temporario);

    fs::write(&temporario, json).map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Não foi possível gravar '{}': {e}", temporario.display()),
    })?;
    fs::rename(&temporario, caminho).map_err(|e| AppError::FalhaArquivo {
        motivo: format!(
            "Não foi possível concluir a gravação de '{}': {e}",
            caminho.display()
        ),
    })?;

    Ok(total)
}

/// Resolve a lista de categorias para leitura, semeando `caminho_app` a
/// partir de `caminho_empacotado` (o `config/categoria.json` versionado)
/// quando `caminho_app` ainda não existe — sem jamais sobrescrever um arquivo
/// já presente (edição do usuário via `salvar_categorias`). Se nem o arquivo
/// empacotado existir (ex.: ambiente sem `config/`), devolve uma lista vazia
/// em vez de erro: tanto a tela de categorias (US8, estado "vazio") quanto a
/// classificação de busca (US5, R11 — cai em "Outros") já lidam bem com isso.
/// `commands::categorias::listar_categorias` e `commands::buscar::executar`
/// chamam esta função com o MESMO `caminho_app`, para que uma categoria
/// criada/editada na tela apareça na próxima busca.
pub fn resolver_com_semente(
    caminho_app: &Path,
    caminho_empacotado: &Path,
) -> Result<Vec<Categoria>, AppError> {
    if !caminho_app.is_file() {
        if caminho_empacotado.is_file() {
            semear(caminho_empacotado, caminho_app)?;
        } else {
            return Ok(Vec::new());
        }
    }
    carregar_categorias(caminho_app)
}

/// Copia `origem` para `destino`, criando o diretório pai se necessário.
/// Usa `create_new` (O_EXCL — checagem de existência + criação atômicas): se
/// `destino` já existir (uma execução concorrente semeou, ou um
/// `salvar_categorias` gravou a edição do usuário entre o `is_file()` do
/// chamador e este ponto), retorna `Ok` sem escrever — NUNCA sobrescreve o
/// arquivo, fechando a corrida TOCTOU que poderia apagar a edição do usuário.
fn semear(origem: &Path, destino: &Path) -> Result<(), AppError> {
    use std::io::Write;

    if let Some(pai) = destino.parent() {
        if !pai.as_os_str().is_empty() {
            fs::create_dir_all(pai).map_err(|e| AppError::FalhaArquivo {
                motivo: format!("Não foi possível criar '{}': {e}", pai.display()),
            })?;
        }
    }

    let conteudo = fs::read(origem).map_err(|e| AppError::FalhaArquivo {
        motivo: format!("Não foi possível ler a semente '{}': {e}", origem.display()),
    })?;

    let falha = |e: std::io::Error| AppError::FalhaArquivo {
        motivo: format!("Não foi possível semear '{}': {e}", destino.display()),
    };
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destino)
    {
        Ok(mut f) => f.write_all(&conteudo).map_err(falha),
        // Alguém criou o arquivo nesse meio-tempo — respeita o que está lá.
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(falha(e)),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    fn escrever(dir: &Path, nome: &str, conteudo: &str) -> PathBuf {
        let caminho = dir.join(nome);
        fs::write(&caminho, conteudo).expect("grava fixture");
        caminho
    }

    #[test]
    fn carrega_categorias_do_json_no_formato_esperado() {
        let dir = tempdir().expect("tempdir");
        let caminho = escrever(
            dir.path(),
            "categoria.json",
            r#"{
                "categorias": [
                    {"nome": "Progressão", "descricao": "Progressão funcional."},
                    {"nome": "Outros", "descricao": null}
                ]
            }"#,
        );

        let categorias = carregar_categorias(&caminho).expect("deve carregar");

        assert_eq!(categorias.len(), 2);
        assert_eq!(categorias[0].nome, "Progressão");
        assert_eq!(
            categorias[0].descricao.as_deref(),
            Some("Progressão funcional.")
        );
        assert_eq!(categorias[1].nome, "Outros");
        assert_eq!(categorias[1].descricao, None);
    }

    #[test]
    fn erro_claro_quando_arquivo_nao_existe() {
        let dir = tempdir().expect("tempdir");
        let erro = carregar_categorias(&dir.path().join("nao_existe.json")).unwrap_err();
        assert!(matches!(erro, AppError::FalhaArquivo { .. }));
    }

    #[test]
    fn erro_claro_quando_json_malformado() {
        let dir = tempdir().expect("tempdir");
        let caminho = escrever(dir.path(), "categoria.json", "isto não é json");
        let erro = carregar_categorias(&caminho).unwrap_err();
        assert!(matches!(erro, AppError::FalhaArquivo { .. }));
    }

    #[test]
    fn erro_claro_quando_lista_de_categorias_vazia() {
        let dir = tempdir().expect("tempdir");
        let caminho = escrever(dir.path(), "categoria.json", r#"{"categorias": []}"#);
        let erro = carregar_categorias(&caminho).unwrap_err();
        assert!(matches!(erro, AppError::FalhaArquivo { .. }));
    }

    // --- salvar_categorias ---------------------------------------------------- //

    #[test]
    fn salvar_e_recarregar_preserva_nome_e_descricao_round_trip() {
        let dir = tempdir().expect("tempdir");
        let caminho = dir.path().join("categoria.json");
        let categorias = vec![
            Categoria::nova("Progressão", Some("Progressão funcional.".to_string())),
            Categoria::nova("Outros", None),
        ];

        let total = salvar_categorias(&caminho, &categorias).expect("deve salvar");
        assert_eq!(total, 2);

        let recarregadas = carregar_categorias(&caminho).expect("deve recarregar");
        assert_eq!(recarregadas, categorias);
    }

    #[test]
    fn salvar_rejeita_nome_vazio_ou_so_espacos_sem_gravar_nada() {
        let dir = tempdir().expect("tempdir");
        let caminho = dir.path().join("categoria.json");

        let erro = salvar_categorias(&caminho, &[Categoria::nova("   ", None)]).unwrap_err();

        assert!(matches!(erro, AppError::CategoriaSemNome));
        assert!(!caminho.exists(), "nada deve ser gravado quando inválido");
    }

    #[test]
    fn salvar_rejeita_nome_duplicado_case_insensitive() {
        let dir = tempdir().expect("tempdir");
        let caminho = dir.path().join("categoria.json");
        let categorias = vec![
            Categoria::nova("Férias", None),
            Categoria::nova("férias", None),
        ];

        let erro = salvar_categorias(&caminho, &categorias).unwrap_err();

        assert_eq!(
            erro,
            AppError::NomeDuplicado {
                nome: "férias".to_string()
            }
        );
        assert!(!caminho.exists(), "nada deve ser gravado quando inválido");
    }

    #[test]
    fn salvar_nao_altera_arquivo_existente_quando_a_entrada_e_invalida() {
        let dir = tempdir().expect("tempdir");
        let caminho = escrever(
            dir.path(),
            "categoria.json",
            r#"{"categorias":[{"nome":"Outros","descricao":null}]}"#,
        );

        let erro = salvar_categorias(&caminho, &[Categoria::nova("", None)]).unwrap_err();

        assert!(matches!(erro, AppError::CategoriaSemNome));
        let ainda = carregar_categorias(&caminho).expect("arquivo original intacto");
        assert_eq!(ainda, vec![Categoria::nova("Outros", None)]);
    }

    #[test]
    fn salvar_normaliza_espacos_nas_pontas_do_nome() {
        let dir = tempdir().expect("tempdir");
        let caminho = dir.path().join("categoria.json");

        salvar_categorias(&caminho, &[Categoria::nova("  Diária  ", None)]).expect("deve salvar");

        let recarregadas = carregar_categorias(&caminho).expect("deve recarregar");
        assert_eq!(recarregadas[0].nome, "Diária");
    }

    // --- resolver_com_semente --------------------------------------------------- //

    #[test]
    fn resolver_com_semente_copia_do_empacotado_quando_o_arquivo_do_app_nao_existe() {
        let dir = tempdir().expect("tempdir");
        let empacotado = escrever(
            dir.path(),
            "empacotado.json",
            r#"{"categorias":[{"nome":"Outros","descricao":null}]}"#,
        );
        let caminho_app = dir.path().join("app_config").join("categoria.json");

        let categorias =
            resolver_com_semente(&caminho_app, &empacotado).expect("deve semear e carregar");

        assert_eq!(categorias, vec![Categoria::nova("Outros", None)]);
        assert!(caminho_app.is_file(), "deve copiar para o caminho do app");
    }

    #[test]
    fn resolver_com_semente_nao_sobrescreve_edicoes_ja_salvas_pelo_usuario() {
        let dir = tempdir().expect("tempdir");
        let empacotado = escrever(
            dir.path(),
            "empacotado.json",
            r#"{"categorias":[{"nome":"Outros","descricao":null}]}"#,
        );
        let caminho_app = escrever(
            dir.path(),
            "categoria.json",
            r#"{"categorias":[{"nome":"Personalizada","descricao":null}]}"#,
        );

        let categorias =
            resolver_com_semente(&caminho_app, &empacotado).expect("deve carregar o existente");

        assert_eq!(categorias, vec![Categoria::nova("Personalizada", None)]);
    }

    #[test]
    fn semear_nao_sobrescreve_destino_que_ja_existe_create_new() {
        // Fecha a corrida TOCTOU: mesmo chamando semear diretamente sobre um
        // destino já existente (edição do usuário), o conteúdo é preservado.
        let dir = tempdir().expect("tempdir");
        let origem = escrever(
            dir.path(),
            "seed.json",
            r#"{"categorias":[{"nome":"Semente","descricao":null}]}"#,
        );
        let destino = escrever(
            dir.path(),
            "app.json",
            r#"{"categorias":[{"nome":"Usuário","descricao":null}]}"#,
        );

        semear(&origem, &destino).expect("não deve falhar nem sobrescrever");

        let cats = carregar_categorias(&destino).expect("carrega o destino");
        assert_eq!(cats, vec![Categoria::nova("Usuário", None)]);
    }

    #[test]
    fn resolver_com_semente_devolve_lista_vazia_quando_nada_existe() {
        let dir = tempdir().expect("tempdir");
        let caminho_app = dir.path().join("categoria.json");
        let empacotado = dir.path().join("nao_existe.json");

        let categorias = resolver_com_semente(&caminho_app, &empacotado).expect("não deve falhar");

        assert!(categorias.is_empty());
    }
}
