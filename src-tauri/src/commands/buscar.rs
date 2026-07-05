//! Comando `buscar_por_siape` — ver `contracts/ipc-commands.md`.
//!
//! Cobre US1 (coleta completa via `GedocRepository`), US2 (filtro por SIAPE,
//! R2) e US3 (consumo pela View). A classificação por categoria (US5) e o
//! resumo (US6) ainda não existem neste MVP: todo documento válido cai num
//! único grupo "Sem categoria" e `tem_pdf` é sempre `false` (download é
//! US4/US7, TODO).
//!
//! `executar_com_repo` é o núcleo síncrono e testável (recebe o
//! `GedocRepository` por parâmetro, sem tocar rede — Princípio VII);
//! `executar`/`buscar_por_siape` são a fronteira async/IPC que injeta a
//! implementação real (`GedocRepositoryHttp<ReqwestHttp>`) e roda a busca
//! (bloqueante, de rede) em `tokio::task::spawn_blocking`.

use serde::{Deserialize, Serialize};

use crate::domain::documento::Documento;
use crate::domain::siape;
use crate::error::AppError;
use crate::ports::gedoc_repository::{GedocRepository, REPOSITORIO_PADRAO};
use crate::ports::http::ReqwestHttp;
use crate::services::filtro;
use crate::services::gedoc_repository::GedocRepositoryHttp;

const SEM_CATEGORIA: &str = "Sem categoria";

#[derive(Debug, Deserialize)]
pub struct BuscarPorSiapeInput {
    pub siape: String,
    pub repositorio: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DocView {
    pub titulo: String,
    pub data: Option<String>,
    pub link: String,
    pub arquivo: Option<String>,
    pub resumo: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CategoriaGrupo {
    pub categoria: String,
    pub qtd: usize,
    pub itens: Vec<DocView>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ResultadoView {
    pub termo: String,
    pub total: u32,
    pub categorias: Vec<CategoriaGrupo>,
    pub tem_pdf: bool,
}

impl From<Documento> for DocView {
    fn from(doc: Documento) -> Self {
        DocView {
            titulo: doc.titulo,
            data: doc.data,
            link: doc.link,
            arquivo: None, // download ainda não existe neste MVP (US4/US7)
            resumo: None,  // resumo ainda não existe neste MVP (US6)
        }
    }
}

/// Monta a `ResultadoView` do contrato IPC a partir dos documentos já
/// validados (R2). Nesta US nenhum documento tem `categoria` classificada
/// (US5, TODO): todos caem num único grupo `"Sem categoria"`.
pub fn montar_resultado(termo: &str, total: u32, validos: Vec<Documento>) -> ResultadoView {
    let categorias = if validos.is_empty() {
        Vec::new()
    } else {
        vec![CategoriaGrupo {
            categoria: SEM_CATEGORIA.to_string(),
            qtd: validos.len(),
            itens: validos.into_iter().map(DocView::from).collect(),
        }]
    };

    ResultadoView {
        termo: termo.to_string(),
        total,
        categorias,
        tem_pdf: false,
    }
}

/// Núcleo síncrono e testável do comando: valida o SIAPE (R10), busca todas
/// as páginas via `repo` (US1/FR-001), filtra por SIAPE (US2/R2) e agrupa o
/// resultado (US3). Recebe o repositório por parâmetro — nenhum dublê de
/// teste precisa tocar rede (Princípio VII).
pub fn executar_com_repo<R: GedocRepository>(
    siape: &str,
    repositorio: Option<&str>,
    repo: &R,
) -> Result<ResultadoView, AppError> {
    siape::validar(siape)?;
    let repositorio = repositorio.unwrap_or(REPOSITORIO_PADRAO);

    let (total, mut docs) = repo.buscar(siape, repositorio)?;
    filtro::filtrar_por_siape(&mut docs, siape);
    let (validos, _descartados) = filtro::separar(docs);

    Ok(montar_resultado(siape, total, validos))
}

/// Fronteira async do comando: valida rápido (sem tocar rede) e só então
/// injeta o repositório HTTP real, rodando a busca bloqueante em
/// `spawn_blocking` (o runtime Tauri/tokio não pode ser bloqueado).
pub async fn executar(siape: &str, repositorio: Option<&str>) -> Result<ResultadoView, AppError> {
    siape::validar(siape)?;

    let siape = siape.to_string();
    let repositorio = repositorio.map(str::to_string);
    tokio::task::spawn_blocking(move || {
        let http = ReqwestHttp::novo()?;
        let repo = GedocRepositoryHttp::novo(http);
        executar_com_repo(&siape, repositorio.as_deref(), &repo)
    })
    .await
    .map_err(|e| AppError::FalhaPortal {
        motivo: format!("Falha interna ao executar a busca: {e}"),
    })?
}

#[tauri::command]
pub async fn buscar_por_siape(input: BuscarPorSiapeInput) -> Result<ResultadoView, AppError> {
    executar(&input.siape, input.repositorio.as_deref()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    type ResultadoBusca = Result<(u32, Vec<Documento>), AppError>;

    /// Dublê de `GedocRepository`: devolve o resultado configurado uma única
    /// vez (`executar_com_repo` chama `buscar` exatamente uma vez por
    /// execução). `RefCell` evita exigir `Clone` de `AppError` só para teste.
    struct RepoFake {
        resultado: std::cell::RefCell<Option<ResultadoBusca>>,
    }

    impl RepoFake {
        fn novo(resultado: ResultadoBusca) -> Self {
            Self {
                resultado: std::cell::RefCell::new(Some(resultado)),
            }
        }
    }

    impl GedocRepository for RepoFake {
        fn buscar(&self, _termo: &str, _repositorio: &str) -> ResultadoBusca {
            self.resultado
                .borrow_mut()
                .take()
                .expect("RepoFake.buscar não deveria ser chamado mais de uma vez")
        }
    }

    fn doc_com_siape(link: &str, titulo: &str, siape: &str) -> Documento {
        let mut d = Documento::novo(link, titulo);
        d.trecho = Some(format!("Designa o servidor SIAPE {siape} para a função"));
        d.siapes = vec![siape.to_string()];
        d
    }

    #[tokio::test]
    async fn rejeita_siape_invalido_sem_tocar_rede() {
        let erro = executar("abc", None).await.unwrap_err();
        assert!(matches!(erro, AppError::SiapeInvalido { .. }));
    }

    #[tokio::test]
    async fn rejeita_siape_curto_demais() {
        let erro = executar("123", None).await.unwrap_err();
        assert!(matches!(erro, AppError::SiapeInvalido { .. }));
    }

    #[test]
    fn siape_valido_busca_via_repo_filtra_e_monta_resultado_sem_tocar_rede() {
        let doc = doc_com_siape(
            "https://gedoc.ifes.edu.br/documento/aaaa?inline",
            "PORTARIA Nº 1 - 2024 - Designação de função",
            "1998547",
        );
        let repo = RepoFake::novo(Ok((1, vec![doc])));

        let resultado = executar_com_repo("1998547", None, &repo).expect("deve montar resultado");

        assert_eq!(resultado.termo, "1998547");
        assert_eq!(resultado.total, 1);
        assert_eq!(resultado.categorias.len(), 1);
        assert_eq!(resultado.categorias[0].categoria, SEM_CATEGORIA);
        assert_eq!(resultado.categorias[0].qtd, 1);
        assert!(!resultado.tem_pdf);
    }

    #[test]
    fn propaga_falha_do_repositorio_como_falha_portal() {
        let repo = RepoFake::novo(Err(AppError::FalhaPortal {
            motivo: "indisponível".to_string(),
        }));
        let erro = executar_com_repo("1998547", None, &repo).unwrap_err();
        assert!(matches!(erro, AppError::FalhaPortal { .. }));
    }

    // --- montar_resultado ------------------------------------------------ //

    #[test]
    fn montar_resultado_agrupa_documentos_validos_em_sem_categoria() {
        let doc = doc_com_siape(
            "https://gedoc.ifes.edu.br/documento/bbbb?inline",
            "DESPACHO Nº 2 - 2024 - Encaminhamento",
            "1998547",
        );
        let resultado = montar_resultado("1998547", 1, vec![doc]);

        assert_eq!(resultado.total, 1);
        assert_eq!(resultado.categorias.len(), 1);
        let grupo = &resultado.categorias[0];
        assert_eq!(grupo.categoria, SEM_CATEGORIA);
        assert_eq!(grupo.qtd, 1);
        assert_eq!(
            grupo.itens[0].titulo,
            "DESPACHO Nº 2 - 2024 - Encaminhamento"
        );
        assert!(grupo.itens[0].arquivo.is_none());
        assert!(grupo.itens[0].resumo.is_none());
        assert!(!resultado.tem_pdf);
    }

    #[test]
    fn montar_resultado_sem_documentos_validos_nao_cria_grupo() {
        let resultado = montar_resultado("1998547", 5, Vec::new());
        assert_eq!(resultado.total, 5);
        assert!(resultado.categorias.is_empty());
    }
}
