//! Use-case de busca por SIAPE (US1/US2/US3/US5/US6).
//!
//! `executar_com_repo` é o núcleo síncrono e testável (recebe o
//! `GedocRepository`, as `Categoria`s, o modo e — no modo `llm` — o
//! `ChatIa`/os dois `CacheArquivo` por parâmetro; nenhum dublê de teste toca
//! rede ou disco reais — Princípio VII). `executar` é a fronteira async: toda
//! a I/O bloqueante roda dentro de um único `spawn_blocking`.
//!
//! **Decisão (US5/US6)**: modo default `Keyword` — grátis, sem chave. `llm`
//! só quando pedido explicitamente **e** com chave configurada; sem chave,
//! degrada para `keyword` (R11). A falha de IA nunca aborta a busca.

use std::path::{Path, PathBuf};

use crate::domain::categoria::{Categoria, OUTROS};
use crate::domain::documento::Documento;
use crate::domain::siape;
use crate::dto::{CategoriaGrupo, DocView, ResultadoView};
use crate::error::AppError;
use crate::ports::gedoc_repository::{GedocRepository, REPOSITORIO_PADRAO};
use crate::ports::http::ReqwestHttp;
use crate::ports::ia::{resolver_api_key, ChatIa, MistralClient};
use crate::services::cache::CacheArquivo;
use crate::services::classificador::{classificar_lote, ModoClassificacao};
use crate::services::gedoc_repository::GedocRepositoryHttp;
use crate::services::{categorias, filtro, resumidor};

const SEM_CATEGORIA: &str = "Sem categoria";

/// Caches de IA (US5/US6), dentro do diretório de dados. Um único arquivo
/// global por tipo (chave = link). As bordas resolvem o caminho final usando
/// estes nomes.
pub const SUBPASTA_CACHE: &str = "cache";
pub const ARQUIVO_CACHE_CLASSIFICACAO: &str = "classificacao.json";
pub const ARQUIVO_CACHE_RESUMO: &str = "resumo.json";

/// Monta a `ResultadoView` a partir dos documentos já validados (R2) e
/// classificados (US5). Agrupa na ordem de `ordem_categorias`; documentos sem
/// categoria (ou fora da lista) formam grupos extras ao final. Grupos vazios
/// são omitidos.
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

/// Núcleo síncrono e testável do caso de uso: valida o SIAPE (R10), busca via
/// `repo` (US1), filtra por SIAPE (US2/R2), classifica (US5) e — no modo `llm`
/// — resume (US6) cada documento válido, e agrupa (US3) na ordem de
/// `categorias`.
#[allow(clippy::too_many_arguments)]
pub fn executar_com_repo<R: GedocRepository>(
    siape: &str,
    repositorio: Option<&str>,
    repo: &R,
    categorias: &[Categoria],
    modo: ModoClassificacao,
    chat: Option<&dyn ChatIa>,
    cache_categoria: Option<&mut CacheArquivo>,
    dir_documentos: &Path,
    cache_resumo: Option<&mut CacheArquivo>,
) -> Result<ResultadoView, AppError> {
    siape::validar(siape)?;
    let repositorio = repositorio.unwrap_or(REPOSITORIO_PADRAO);

    let (total, mut docs) = repo.buscar(siape, repositorio)?;
    filtro::filtrar_por_siape(&mut docs, siape);
    let (mut validos, _descartados) = filtro::separar(docs);

    classificar_lote(&mut validos, categorias, modo, chat, cache_categoria);

    // US6: resumo só no modo `llm` e só quando há `ChatIa` configurado; sem
    // chat, `doc.resumo` permanece `None` (R11: ausência de resumo nunca
    // aborta a busca).
    if modo == ModoClassificacao::Llm {
        if let Some(chat) = chat {
            resumidor::resumir_lote(&mut validos, siape, chat, dir_documentos, cache_resumo);
        }
    }

    let ordem: Vec<String> = categorias.iter().map(|c| c.nome.clone()).collect();
    Ok(montar_resultado(siape, total, validos, &ordem))
}

/// Fronteira async: valida rápido (sem tocar rede/disco) e roda, num único
/// `spawn_blocking`, toda a I/O bloqueante. Recebe os caminhos já resolvidos
/// pela borda (comando Tauri ou handler HTTP). `categorias_path` é o arquivo
/// de categorias (semeado de `caminho_padrao`); `None` só em testes.
pub async fn executar(
    siape: &str,
    repositorio: Option<&str>,
    modo: ModoClassificacao,
    cache_categoria_path: Option<PathBuf>,
    dir_documentos: PathBuf,
    cache_resumo_path: Option<PathBuf>,
    categorias_path: Option<PathBuf>,
) -> Result<ResultadoView, AppError> {
    siape::validar(siape)?;

    let siape = siape.to_string();
    let repositorio = repositorio.map(str::to_string);
    tokio::task::spawn_blocking(move || {
        let http = ReqwestHttp::novo()?;
        let repo = GedocRepositoryHttp::novo(http);

        // R11: config ausente/malformada não derruba a busca.
        let caminho_categorias = categorias_path.unwrap_or_else(categorias::caminho_padrao);
        let categorias =
            categorias::resolver_com_semente(&caminho_categorias, &categorias::caminho_padrao())
                .unwrap_or_else(|e| {
                    eprintln!(
                        "[gedocs] aviso: não foi possível carregar as categorias ({e}); \
                 documentos cairão em '{OUTROS}'."
                    );
                    Vec::new()
                });

        let mut cache_categoria = cache_categoria_path.map(CacheArquivo::carregar);
        let mut cache_resumo = cache_resumo_path.map(CacheArquivo::carregar);
        // Sem chave, `resolver_api_key`/`MistralClient::novo` devolvem `None`
        // e o modo `llm` degrada para `keyword` (R11).
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
            cache_categoria.as_mut(),
            &dir_documentos,
            cache_resumo.as_mut(),
        )
    })
    .await
    .map_err(|e| AppError::FalhaPortal {
        motivo: format!("Falha interna ao executar a busca: {e}"),
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    type ResultadoBusca = Result<(u32, Vec<Documento>), AppError>;

    /// Dublê de `GedocRepository`: devolve o resultado configurado uma única
    /// vez. `RefCell` evita exigir `Clone` de `AppError` só para teste.
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

    fn dir_documentos_neutro() -> PathBuf {
        PathBuf::from(".")
    }

    /// Dublê de `ChatIa`: `chat` classifica (JSON) e `resumir` resume.
    struct ChatFake {
        respostas_resumir: std::cell::RefCell<std::collections::VecDeque<Result<String, AppError>>>,
        chamadas_resumir: std::cell::RefCell<u32>,
    }

    impl ChatFake {
        fn com_resumos(respostas: Vec<Result<String, AppError>>) -> Self {
            Self {
                respostas_resumir: std::cell::RefCell::new(respostas.into()),
                chamadas_resumir: std::cell::RefCell::new(0),
            }
        }
    }

    impl ChatIa for ChatFake {
        fn chat(&self, _sistema: &str, _usuario: &str) -> Result<String, AppError> {
            Ok(r#"{"categoria":"Progressão"}"#.to_string())
        }

        fn resumir(&self, _sistema: &str, _usuario: &str) -> Result<String, AppError> {
            *self.chamadas_resumir.borrow_mut() += 1;
            self.respostas_resumir
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Ok("resumo padrão".to_string()))
        }
    }

    #[tokio::test]
    async fn rejeita_siape_invalido_sem_tocar_rede() {
        let erro = executar(
            "abc",
            None,
            ModoClassificacao::Keyword,
            None,
            dir_documentos_neutro(),
            None,
            None,
        )
        .await
        .unwrap_err();
        assert!(matches!(erro, AppError::SiapeInvalido { .. }));
    }

    #[tokio::test]
    async fn rejeita_siape_curto_demais() {
        let erro = executar(
            "123",
            None,
            ModoClassificacao::Keyword,
            None,
            dir_documentos_neutro(),
            None,
            None,
        )
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
            &dir_documentos_neutro(),
            None,
        )
        .expect("deve montar resultado");

        assert_eq!(resultado.termo, "1998547");
        assert_eq!(resultado.total, 1);
        assert_eq!(resultado.categorias.len(), 1);
        assert_eq!(resultado.categorias[0].categoria, OUTROS);
        assert_eq!(resultado.categorias[0].qtd, 1);
        assert!(!resultado.tem_pdf);
        assert!(
            resultado.categorias[0].itens[0].resumo.is_none(),
            "modo keyword não resume (US6)"
        );
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
            &dir_documentos_neutro(),
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
            Categoria::nova("Comissão", None),
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
            &dir_documentos_neutro(),
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

    #[test]
    fn modo_llm_com_chat_resume_cada_documento_valido() {
        let doc1 = doc_com_siape(
            "https://gedoc.ifes.edu.br/documento/aaaa?inline",
            "PORTARIA Nº 1 - 2024 - Progressão funcional",
            "1998547",
        );
        let repo = RepoFake::novo(Ok((1, vec![doc1])));
        let chat = ChatFake::com_resumos(vec![Ok("Resumo gerado pela IA.".to_string())]);

        let resultado = executar_com_repo(
            "1998547",
            None,
            &repo,
            &[],
            ModoClassificacao::Llm,
            Some(&chat),
            None,
            &dir_documentos_neutro(),
            None,
        )
        .expect("deve montar resultado");

        assert_eq!(
            resultado.categorias[0].itens[0].resumo.as_deref(),
            Some("Resumo gerado pela IA.")
        );
        assert_eq!(*chat.chamadas_resumir.borrow(), 1);
    }

    #[test]
    fn modo_llm_sem_chat_nao_resume_e_nao_aborta_a_busca_r11() {
        let doc1 = doc_com_siape(
            "https://gedoc.ifes.edu.br/documento/aaaa?inline",
            "PORTARIA Nº 1 - 2024 - Progressão funcional",
            "1998547",
        );
        let repo = RepoFake::novo(Ok((1, vec![doc1])));

        let resultado = executar_com_repo(
            "1998547",
            None,
            &repo,
            &[],
            ModoClassificacao::Llm,
            None,
            None,
            &dir_documentos_neutro(),
            None,
        )
        .expect("busca não deve abortar sem chat (R11)");

        assert!(resultado.categorias[0].itens[0].resumo.is_none());
    }

    #[test]
    fn modo_llm_falha_ao_resumir_1_doc_nao_aborta_o_lote_r11() {
        let doc1 = doc_com_siape(
            "https://gedoc.ifes.edu.br/documento/aaaa?inline",
            "PORTARIA Nº 1 - 2024 - Assunto A",
            "1998547",
        );
        let doc2 = doc_com_siape(
            "https://gedoc.ifes.edu.br/documento/bbbb?inline",
            "PORTARIA Nº 2 - 2024 - Assunto B",
            "1998547",
        );
        let repo = RepoFake::novo(Ok((2, vec![doc1, doc2])));
        let chat = ChatFake::com_resumos(vec![
            Err(AppError::FalhaIA {
                motivo: "instável".to_string(),
            }),
            Ok("Resumo do segundo documento.".to_string()),
        ]);

        let resultado = executar_com_repo(
            "1998547",
            None,
            &repo,
            &[],
            ModoClassificacao::Llm,
            Some(&chat),
            None,
            &dir_documentos_neutro(),
            None,
        )
        .expect("falha ao resumir 1 doc não pode abortar a busca (R11)");

        let itens = &resultado.categorias[0].itens;
        assert_eq!(itens.len(), 2);
        assert!(itens.iter().any(|d| d.resumo.is_none()));
        assert!(itens
            .iter()
            .any(|d| d.resumo.as_deref() == Some("Resumo do segundo documento.")));
    }

    #[test]
    fn modo_llm_cache_de_resumo_e_independente_do_cache_de_categoria_r6() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut cache_categoria =
            CacheArquivo::carregar(dir.path().join(ARQUIVO_CACHE_CLASSIFICACAO));
        let mut cache_resumo = CacheArquivo::carregar(dir.path().join(ARQUIVO_CACHE_RESUMO));
        let chat = ChatFake::com_resumos(vec![Ok("Resumo cacheado.".to_string())]);

        let doc = || {
            doc_com_siape(
                "https://gedoc.ifes.edu.br/documento/aaaa?inline",
                "PORTARIA Nº 1 - 2024 - Progressão funcional",
                "1998547",
            )
        };

        let repo1 = RepoFake::novo(Ok((1, vec![doc()])));
        executar_com_repo(
            "1998547",
            None,
            &repo1,
            &[],
            ModoClassificacao::Llm,
            Some(&chat),
            Some(&mut cache_categoria),
            &dir_documentos_neutro(),
            Some(&mut cache_resumo),
        )
        .expect("primeira busca");
        assert_eq!(*chat.chamadas_resumir.borrow(), 1);

        let repo2 = RepoFake::novo(Ok((1, vec![doc()])));
        let resultado2 = executar_com_repo(
            "1998547",
            None,
            &repo2,
            &[],
            ModoClassificacao::Llm,
            Some(&chat),
            Some(&mut cache_categoria),
            &dir_documentos_neutro(),
            Some(&mut cache_resumo),
        )
        .expect("segunda busca");

        assert_eq!(*chat.chamadas_resumir.borrow(), 1, "cache hit (R6)");
        assert_eq!(
            resultado2.categorias[0].itens[0].resumo.as_deref(),
            Some("Resumo cacheado.")
        );
        assert!(dir.path().join(ARQUIVO_CACHE_CLASSIFICACAO).is_file());
        assert!(dir.path().join(ARQUIVO_CACHE_RESUMO).is_file());
    }

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
    fn montar_resultado_repassa_arquivo_e_resumo_do_documento_a_view() {
        let mut doc = doc_com_siape(
            "https://gedoc.ifes.edu.br/documento/cccc?inline",
            "PORTARIA Nº 3 - 2024 - Assunto",
            "1998547",
        );
        doc.arquivo = Some("2024_3_Assunto.pdf".to_string());
        doc.resumo = Some("Determina a progressão do servidor.".to_string());

        let resultado = montar_resultado("1998547", 1, vec![doc], &[]);
        let item = &resultado.categorias[0].itens[0];

        assert_eq!(item.arquivo.as_deref(), Some("2024_3_Assunto.pdf"));
        assert_eq!(
            item.resumo.as_deref(),
            Some("Determina a progressão do servidor.")
        );
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
