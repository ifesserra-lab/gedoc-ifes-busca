//! Comando `buscar_por_siape` — ver `contracts/ipc-commands.md`.
//!
//! Cobre US1 (coleta completa via `GedocRepository`), US2 (filtro por SIAPE,
//! R2), US3 (consumo pela View) e US5 (classificação por categoria, R4/R5/
//! R6/R9/R11). Documentos válidos são classificados antes de agrupar; o
//! resumo (US6) ainda não existe neste MVP e `tem_pdf` é sempre `false`.
//!
//! **Decisão de design (US5)**: o modo de classificação default é
//! `ModoClassificacao::Keyword` — grátis, instantâneo, sem tocar API nem
//! exigir chave. `buscar_por_siape` NÃO liga o modo `llm` sozinho em toda
//! busca (custo/latência de chamadas de IA); ele só é usado quando o input
//! pede explicitamente `modo: "llm"` **e** uma chave de IA está configurada
//! (env `MISTRAL_API_KEY`/`MISTRAL_KEY` ou `config/.env`) — sem chave, o
//! pedido de `llm` degrada silenciosamente para `keyword` (R11: a busca
//! nunca falha por causa da classificação). No modo `llm`, o cache por link
//! (R6) e o throttle embutido no cliente Mistral (R9) evitam reclassificar e
//! evitam rate limit; a falha ao classificar 1 documento cai no
//! classificador `keyword` só para aquele documento (R11), sem abortar o
//! lote. Falha/ausência de `config/categoria.json` também não aborta a
//! busca: sem categorias, todo documento cai em "Outros" (R11).
//!
//! `executar_com_repo` é o núcleo síncrono e testável (recebe o
//! `GedocRepository`, as `Categoria`s, o modo e — no modo `llm` — o
//! `ChatIa`/`CacheArquivo` por parâmetro; nenhum dublê de teste toca rede ou
//! disco reais — Princípio VII). `executar`/`buscar_por_siape` são a
//! fronteira async/IPC: toda a I/O bloqueante (ler `config/categoria.json`,
//! resolver a chave de IA, buscar no portal, ler/gravar o cache) roda dentro
//! de um único `tokio::task::spawn_blocking` para nunca travar o runtime.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::domain::categoria::{Categoria, OUTROS};
use crate::domain::documento::Documento;
use crate::domain::siape;
use crate::error::AppError;
use crate::ports::gedoc_repository::{GedocRepository, REPOSITORIO_PADRAO};
use crate::ports::http::ReqwestHttp;
use crate::ports::ia::{resolver_api_key, ChatIa, MistralClient};
use crate::services::cache::CacheArquivo;
use crate::services::classificador::{classificar_lote, ModoClassificacao};
use crate::services::gedoc_repository::GedocRepositoryHttp;
use crate::services::{categorias, filtro};

const SEM_CATEGORIA: &str = "Sem categoria";
/// Cache de classificação (US5), dentro de `app_data_dir` — fora do VCS
/// (Princípio II/LGPD); só guarda link→categoria, nunca título/trecho/SIAPE.
const SUBPASTA_CACHE: &str = "cache";
const ARQUIVO_CACHE_CLASSIFICACAO: &str = "classificacao.json";

#[derive(Debug, Deserialize)]
pub struct BuscarPorSiapeInput {
    pub siape: String,
    pub repositorio: Option<String>,
    /// Estratégia de classificação (US5): `"keyword"` (default) ou `"llm"`.
    /// Ausente ou desconhecido => `keyword` (nunca falha por valor inesperado).
    pub modo: Option<String>,
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
/// validados (R2) e já classificados (`doc.categoria`, US5). Agrupa na
/// ordem de `ordem_categorias` (a ordem de `config/categoria.json`);
/// documentos sem categoria (ou com uma fora dessa lista) formam grupos
/// extras, na ordem em que aparecem, ao final. Grupos vazios são omitidos.
pub fn montar_resultado(
    termo: &str,
    total: u32,
    validos: Vec<Documento>,
    ordem_categorias: &[String],
) -> ResultadoView {
    let mut ordem: Vec<String> = ordem_categorias.to_vec();
    let mut grupos: std::collections::HashMap<String, Vec<Documento>> =
        std::collections::HashMap::new();

    for doc in validos {
        let categoria = doc
            .categoria
            .clone()
            .unwrap_or_else(|| SEM_CATEGORIA.to_string());
        if !ordem.contains(&categoria) {
            ordem.push(categoria.clone());
        }
        grupos.entry(categoria).or_default().push(doc);
    }

    let categorias = ordem
        .into_iter()
        .filter_map(|nome| {
            let docs = grupos.remove(&nome).filter(|docs| !docs.is_empty())?;
            Some(CategoriaGrupo {
                qtd: docs.len(),
                itens: docs.into_iter().map(DocView::from).collect(),
                categoria: nome,
            })
        })
        .collect();

    ResultadoView {
        termo: termo.to_string(),
        total,
        categorias,
        tem_pdf: false,
    }
}

/// Núcleo síncrono e testável do comando: valida o SIAPE (R10), busca todas
/// as páginas via `repo` (US1/FR-001), filtra por SIAPE (US2/R2), classifica
/// cada documento válido (US5) e agrupa o resultado (US3) na ordem de
/// `categorias`. Recebe repositório/categorias/estratégia de IA por
/// parâmetro — nenhum dublê de teste precisa tocar rede ou disco reais
/// (Princípio VII).
#[allow(clippy::too_many_arguments)]
pub fn executar_com_repo<R: GedocRepository>(
    siape: &str,
    repositorio: Option<&str>,
    repo: &R,
    categorias: &[Categoria],
    modo: ModoClassificacao,
    chat: Option<&dyn ChatIa>,
    cache: Option<&mut CacheArquivo>,
) -> Result<ResultadoView, AppError> {
    siape::validar(siape)?;
    let repositorio = repositorio.unwrap_or(REPOSITORIO_PADRAO);

    let (total, mut docs) = repo.buscar(siape, repositorio)?;
    filtro::filtrar_por_siape(&mut docs, siape);
    let (mut validos, _descartados) = filtro::separar(docs);

    classificar_lote(&mut validos, categorias, modo, chat, cache);

    let ordem: Vec<String> = categorias.iter().map(|c| c.nome.clone()).collect();
    Ok(montar_resultado(siape, total, validos, &ordem))
}

/// Fronteira async do comando: valida rápido (sem tocar rede/disco) e só
/// então roda, num único `spawn_blocking`, toda a I/O bloqueante — ler
/// `config/categoria.json`, resolver a chave de IA (se `modo == Llm`),
/// buscar no portal e classificar (lendo/gravando o cache, se aplicável).
pub async fn executar(
    siape: &str,
    repositorio: Option<&str>,
    modo: ModoClassificacao,
    cache_path: Option<PathBuf>,
) -> Result<ResultadoView, AppError> {
    siape::validar(siape)?;

    let siape = siape.to_string();
    let repositorio = repositorio.map(str::to_string);
    tokio::task::spawn_blocking(move || {
        let http = ReqwestHttp::novo()?;
        let repo = GedocRepositoryHttp::novo(http);

        // R11: config ausente/malformada não derruba a busca — sem
        // categorias configuradas, todo documento cai em "Outros".
        let categorias = categorias::carregar_categorias(&categorias::caminho_padrao())
            .unwrap_or_else(|e| {
                eprintln!(
                    "[gedocs] aviso: não foi possível carregar as categorias ({e}); \
                     documentos cairão em '{OUTROS}'."
                );
                Vec::new()
            });

        let mut cache = cache_path.map(CacheArquivo::carregar);
        // Sem chave configurada, `resolver_api_key`/`MistralClient::novo`
        // devolvem `None` e o modo `llm` degrada para `keyword` (R11).
        let cliente_ia = (modo == ModoClassificacao::Llm)
            .then(resolver_api_key)
            .flatten()
            .and_then(|chave| MistralClient::novo(chave).ok());
        let chat: Option<&dyn ChatIa> = cliente_ia.as_ref().map(|c| c as &dyn ChatIa);

        executar_com_repo(
            &siape,
            repositorio.as_deref(),
            &repo,
            &categorias,
            modo,
            chat,
            cache.as_mut(),
        )
    })
    .await
    .map_err(|e| AppError::FalhaPortal {
        motivo: format!("Falha interna ao executar a busca: {e}"),
    })?
}

#[tauri::command]
pub async fn buscar_por_siape(
    app: AppHandle,
    input: BuscarPorSiapeInput,
) -> Result<ResultadoView, AppError> {
    siape::validar(&input.siape)?;

    let modo = ModoClassificacao::from_entrada(input.modo.as_deref());
    // Cache só é relevante (e só é resolvido) no modo `llm` — o modo
    // `keyword` é instantâneo e não se beneficia de cache (ver doc do
    // módulo).
    let cache_path = (modo == ModoClassificacao::Llm)
        .then(|| app.path().app_data_dir().ok())
        .flatten()
        .map(|dir| dir.join(SUBPASTA_CACHE).join(ARQUIVO_CACHE_CLASSIFICACAO));

    executar(&input.siape, input.repositorio.as_deref(), modo, cache_path).await
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
        let erro = executar("abc", None, ModoClassificacao::Keyword, None)
            .await
            .unwrap_err();
        assert!(matches!(erro, AppError::SiapeInvalido { .. }));
    }

    #[tokio::test]
    async fn rejeita_siape_curto_demais() {
        let erro = executar("123", None, ModoClassificacao::Keyword, None)
            .await
            .unwrap_err();
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

        let resultado = executar_com_repo(
            "1998547",
            None,
            &repo,
            &[],
            ModoClassificacao::Keyword,
            None,
            None,
        )
        .expect("deve montar resultado");

        assert_eq!(resultado.termo, "1998547");
        assert_eq!(resultado.total, 1);
        assert_eq!(resultado.categorias.len(), 1);
        // Sem categorias configuradas (`&[]`), o classificador por
        // palavra-chave (default) cai em "Outros" — não mais "Sem
        // categoria", que só se aplica a quem nunca passou por classificação
        // (ver `montar_resultado_agrupa_documentos_validos_em_sem_categoria`).
        assert_eq!(resultado.categorias[0].categoria, OUTROS);
        assert_eq!(resultado.categorias[0].qtd, 1);
        assert!(!resultado.tem_pdf);
    }

    #[test]
    fn propaga_falha_do_repositorio_como_falha_portal() {
        let repo = RepoFake::novo(Err(AppError::FalhaPortal {
            motivo: "indisponível".to_string(),
        }));
        let erro = executar_com_repo(
            "1998547",
            None,
            &repo,
            &[],
            ModoClassificacao::Keyword,
            None,
            None,
        )
        .unwrap_err();
        assert!(matches!(erro, AppError::FalhaPortal { .. }));
    }

    #[test]
    fn executar_com_repo_classifica_por_palavra_chave_e_agrupa_na_ordem_do_config() {
        let doc1 = doc_com_siape(
            "https://gedoc.ifes.edu.br/documento/aaaa?inline",
            "PORTARIA Nº 1 - 2024 - Progressão funcional",
            "1998547",
        );
        let doc2 = doc_com_siape(
            "https://gedoc.ifes.edu.br/documento/bbbb?inline",
            "DESPACHO Nº 2 - 2024 - Comunicado qualquer",
            "1998547",
        );
        let repo = RepoFake::novo(Ok((2, vec![doc1, doc2])));
        let categorias = vec![
            Categoria::nova("Progressão", None),
            Categoria::nova("Comissão", None), // sem documentos -> grupo omitido
            Categoria::nova(OUTROS, None),
        ];

        let resultado = executar_com_repo(
            "1998547",
            None,
            &repo,
            &categorias,
            ModoClassificacao::Keyword,
            None,
            None,
        )
        .expect("deve montar resultado");

        assert_eq!(
            resultado.categorias.len(),
            2,
            "grupo vazio (Comissão) é omitido"
        );
        assert_eq!(resultado.categorias[0].categoria, "Progressão");
        assert_eq!(resultado.categorias[1].categoria, OUTROS);
    }

    // --- montar_resultado ------------------------------------------------ //

    #[test]
    fn montar_resultado_agrupa_documentos_validos_em_sem_categoria() {
        let doc = doc_com_siape(
            "https://gedoc.ifes.edu.br/documento/bbbb?inline",
            "DESPACHO Nº 2 - 2024 - Encaminhamento",
            "1998547",
        );
        let resultado = montar_resultado("1998547", 1, vec![doc], &[]);

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
        let resultado = montar_resultado("1998547", 5, Vec::new(), &[]);
        assert_eq!(resultado.total, 5);
        assert!(resultado.categorias.is_empty());
    }

    #[test]
    fn montar_resultado_segue_a_ordem_informada_e_omite_grupos_vazios() {
        let mut doc_a = doc_com_siape(
            "https://gedoc.ifes.edu.br/documento/aaaa?inline",
            "PORTARIA Nº 1 - 2024 - A",
            "1998547",
        );
        doc_a.categoria = Some("Outros".to_string());
        let mut doc_b = doc_com_siape(
            "https://gedoc.ifes.edu.br/documento/bbbb?inline",
            "PORTARIA Nº 2 - 2024 - B",
            "1998547",
        );
        doc_b.categoria = Some("Progressão".to_string());

        let ordem = vec![
            "Progressão".to_string(),
            "Comissão".to_string(),
            "Outros".to_string(),
        ];
        let resultado = montar_resultado("1998547", 2, vec![doc_a, doc_b], &ordem);

        assert_eq!(
            resultado.categorias.len(),
            2,
            "Comissão fica vazia e é omitida"
        );
        assert_eq!(resultado.categorias[0].categoria, "Progressão");
        assert_eq!(resultado.categorias[1].categoria, "Outros");
    }

    #[test]
    fn montar_resultado_acrescenta_categoria_fora_da_ordem_configurada_ao_final() {
        let mut doc = doc_com_siape(
            "https://gedoc.ifes.edu.br/documento/cccc?inline",
            "PORTARIA Nº 3 - 2024 - C",
            "1998547",
        );
        doc.categoria = Some("Categoria Legada".to_string());

        let ordem = vec!["Progressão".to_string()];
        let resultado = montar_resultado("1998547", 1, vec![doc], &ordem);

        assert_eq!(resultado.categorias.len(), 1);
        assert_eq!(resultado.categorias[0].categoria, "Categoria Legada");
    }
}
