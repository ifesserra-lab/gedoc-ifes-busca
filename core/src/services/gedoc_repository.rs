//! Implementação real do `GedocRepository` (`ports::gedoc_repository`) sobre
//! o portal JSF/PrimeFaces do GeDoc IFES — sessão, `ViewState`, IDs
//! descobertos em runtime (R8) e paginação completa (FR-001).
//!
//! Fonte de referência (legado, comprovado em produção):
//! `src/buscar_gedoc.py`, classe `GedocClient` (`abrir`, `_post`,
//! `_atualizar_viewstate`, `buscar`, `_campos_form`, `pagina`, `coletar`).
//!
//! O parser da resposta (`services::gedoc_parse::parse_resposta`) já existe e
//! é reaproveitado aqui sem alterações. Toda a orquestração deste módulo é
//! testada com um `HttpPort` dublê (`FakeHttp`, no módulo de testes) e
//! fixtures locais — nenhum teste toca a rede (Princípio VII). O único trecho
//! não coberto por teste unitário é o adapter `ReqwestHttp`
//! (`ports::http`), que é a fronteira de I/O.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use crate::error::AppError;
use crate::ports::gedoc_repository::GedocRepository;
use crate::ports::http::HttpPort;
use crate::services::gedoc_parse::{parse_resposta, DocumentoParseado};

use crate::domain::documento::Documento;

const BASE: &str = "https://gedoc.ifes.edu.br";
const PAGE: &str = "/faces/pesquisarDocumentos/pesquisarHistorico.xhtml";
const ROWS_POR_PAGINA: u32 = 10;

// Regexes de descoberta compiladas uma vez (padrão do crate — ver
// `gedoc_parse.rs`). A regex de `action` não entra aqui: seu padrão depende do
// `form` descoberto em runtime, então é montada por chamada em `descobrir_ids`.
static RE_VIEWSTATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"name="javax\.faces\.ViewState"[^>]*value="([^"]+)""#).unwrap());
static RE_BTN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"PrimeFaces\.ab\(\{source:'([^']+)'[^)]*panelResultado").unwrap());
static RE_VIEWSTATE_PARCIAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"ViewState[^>]*><!\[CDATA\[([^\]]+)\]\]>").unwrap());

/// IDs JSF descobertos dinamicamente na página inicial (R8): eles mudam a
/// cada deploy do portal, então nunca são fixados em código.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ids {
    /// ID do `<form>` que envolve a busca (ex.: `j_idt65`).
    form: String,
    /// ID do botão de busca, fonte do evento AJAX (ex.: `j_idt65:j_idt115`).
    btn: String,
    /// ID do `DataList` que pagina os resultados (ex.: `j_idt65:dataList`).
    datalist: String,
    /// Caminho de ação do formulário (relativo à `BASE`).
    action: String,
    /// `ViewState` inicial, antes de qualquer POST.
    viewstate: String,
}

/// Repositório HTTP real: injeta qualquer `HttpPort` (produção: `ReqwestHttp`;
/// teste: um dublê síncrono), sem interior mutability — todo o estado que
/// muda durante a busca (ViewState corrente, links já vistos) fica em
/// variáveis locais de `buscar`, nunca em campos da struct.
pub struct GedocRepositoryHttp<H: HttpPort> {
    http: H,
}

impl<H: HttpPort> GedocRepositoryHttp<H> {
    pub fn novo(http: H) -> Self {
        Self { http }
    }
}

impl<H: HttpPort> GedocRepositoryHttp<H> {
    /// Repositórios (shards) do portal: `0`=Boletim, `1`=GeDoc,
    /// `2`=Site/Reitoria. Cada um é um acervo distinto — documentos antigos
    /// (ex.: 2006) vivem no Boletim, não no GeDoc.
    const SHARDS: [&'static str; 3] = ["0", "1", "2"];

    /// Busca num único repositório (shard), paginando até o total.
    fn buscar_shard(
        &self,
        termo: &str,
        repositorio: &str,
    ) -> Result<(u32, Vec<Documento>), AppError> {
        let home = self.http.get(&format!("{BASE}{PAGE}"))?;
        let ids = descobrir_ids(&home)?;
        let url_action = format!("{BASE}{}", ids.action);
        let mut viewstate = ids.viewstate.clone();

        let mut campos = campos_busca(&ids, termo, repositorio);
        campos.push(("javax.faces.ViewState".to_string(), viewstate.clone()));
        let xml = self.http.post_form(&url_action, &campos)?;
        if let Some(vs) = extrair_viewstate_parcial(&xml) {
            viewstate = vs;
        }
        let resposta = parse_resposta(&xml)?;

        let mut vistos: HashSet<String> =
            resposta.documentos.iter().map(|d| d.link.clone()).collect();
        let mut documentos = resposta.documentos;
        let total = resposta.total.unwrap_or(documentos.len() as u32);

        let mut first = ROWS_POR_PAGINA;
        while (documentos.len() as u32) < total {
            let mut campos = campos_pagina(&ids, first, ROWS_POR_PAGINA);
            campos.push(("javax.faces.ViewState".to_string(), viewstate.clone()));
            let xml = self.http.post_form(&url_action, &campos)?;
            if let Some(vs) = extrair_viewstate_parcial(&xml) {
                viewstate = vs;
            }
            let pagina = parse_resposta(&xml)?;

            let novos: Vec<DocumentoParseado> = pagina
                .documentos
                .into_iter()
                .filter(|d| vistos.insert(d.link.clone()))
                .collect();
            if novos.is_empty() {
                break; // página não trouxe nada novo -- evita loop infinito
            }
            documentos.extend(novos);
            first += ROWS_POR_PAGINA;
        }

        let docs = documentos
            .into_iter()
            .map(DocumentoParseado::para_documento)
            .collect();
        Ok((total, docs))
    }
}

impl<H: HttpPort> GedocRepository for GedocRepositoryHttp<H> {
    fn buscar(&self, termo: &str, repositorio: &str) -> Result<(u32, Vec<Documento>), AppError> {
        // Repositório concreto (0/1/2) → só ele. Caso contrário ("todos",
        // default) → agrega TODOS os repositórios como o portal faz; sem isso,
        // documentos antigos (Boletim, shard 0) não apareceriam.
        if Self::SHARDS.contains(&repositorio) {
            return self.buscar_shard(termo, repositorio);
        }
        let mut vistos: HashSet<String> = HashSet::new();
        let mut docs: Vec<Documento> = Vec::new();
        let mut total = 0u32;
        let mut ultimo_erro = None;
        for shard in Self::SHARDS {
            match self.buscar_shard(termo, shard) {
                Ok((t, ds)) => {
                    total += t;
                    for d in ds {
                        if vistos.insert(d.link.clone()) {
                            docs.push(d);
                        }
                    }
                }
                // R11: falha de um repositório não derruba os demais.
                Err(e) => ultimo_erro = Some(e),
            }
        }
        if docs.is_empty() {
            if let Some(e) = ultimo_erro {
                return Err(e);
            }
        }
        Ok((total, docs))
    }
}

/// Descobre `form`/`btn`/`datalist`/`action`/`viewstate` a partir do HTML da
/// página inicial. Erro claro (não pânico) se o layout do portal mudou.
fn descobrir_ids(page: &str) -> Result<Ids, AppError> {
    let erro_layout = || AppError::FalhaPortal {
        motivo: "Não localizei ViewState/botão de busca -- o layout do portal pode ter mudado."
            .to_string(),
    };

    let viewstate = RE_VIEWSTATE
        .captures(page)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(erro_layout)?;
    let btn = RE_BTN
        .captures(page)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(erro_layout)?;

    let form = btn
        .rsplit_once(':')
        .map(|(f, _)| f.to_string())
        .unwrap_or_else(|| btn.clone());
    let datalist = format!("{form}:dataList");

    let re_action = Regex::new(&format!(
        r#"<form\b[^>]*id="{}"[^>]*action="([^"]+)""#,
        regex::escape(&form)
    ))
    .unwrap();
    let action = re_action
        .captures(page)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| PAGE.to_string());

    Ok(Ids {
        form,
        btn,
        datalist,
        action,
        viewstate,
    })
}

/// Extrai o `ViewState` atualizado de uma resposta parcial (AJAX), se houver.
/// Sem casar, o `ViewState` corrente é mantido (o portal nem sempre o
/// reenvia).
fn extrair_viewstate_parcial(xml: &str) -> Option<String> {
    RE_VIEWSTATE_PARCIAL
        .captures(xml)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Campos do POST de busca inicial (equivalente a `GedocClient.buscar` +
/// `_campos_form` do legado). Não inclui `javax.faces.ViewState`: quem
/// orquestra (`buscar`) o acrescenta, pois é o único campo que muda a cada
/// requisição.
fn campos_busca(ids: &Ids, termo: &str, repositorio: &str) -> Vec<(String, String)> {
    let mut campos = vec![
        ("javax.faces.partial.ajax".to_string(), "true".to_string()),
        ("javax.faces.source".to_string(), ids.btn.clone()),
        ("javax.faces.partial.execute".to_string(), ids.form.clone()),
        (
            "javax.faces.partial.render".to_string(),
            format!("{form}:panelResultado {form}:messages", form = ids.form),
        ),
        (ids.btn.clone(), ids.btn.clone()),
        (ids.form.clone(), ids.form.clone()),
        (format!("{}:nome", ids.form), termo.to_string()),
    ];

    let selecionados = [
        ("campo", "RELEVANCIA"),
        ("ordem", "DECRESCENTE"),
        ("shardItems", repositorio),
    ];
    for nome in ["mes", "ano", "campo", "campus", "ordem", "shardItems"] {
        let valor = selecionados
            .iter()
            .find(|(n, _)| *n == nome)
            .map(|(_, v)| *v)
            .unwrap_or("");
        campos.push((format!("{}:{nome}_focus", ids.form), String::new()));
        campos.push((format!("{}:{nome}_input", ids.form), valor.to_string()));
    }
    campos
}

/// Campos do POST de paginação do `DataList` (equivalente a
/// `GedocClient.pagina` do legado).
fn campos_pagina(ids: &Ids, first: u32, rows: u32) -> Vec<(String, String)> {
    vec![
        ("javax.faces.partial.ajax".to_string(), "true".to_string()),
        ("javax.faces.source".to_string(), ids.datalist.clone()),
        (
            "javax.faces.partial.execute".to_string(),
            ids.datalist.clone(),
        ),
        (
            "javax.faces.partial.render".to_string(),
            ids.datalist.clone(),
        ),
        (format!("{}_pagination", ids.datalist), "true".to_string()),
        (format!("{}_first", ids.datalist), first.to_string()),
        (format!("{}_rows", ids.datalist), rows.to_string()),
        (
            format!("{}_encodeFeature", ids.datalist),
            "true".to_string(),
        ),
        (ids.form.clone(), ids.form.clone()),
    ]
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use super::*;

    const FIXTURE_HOME: &str = include_str!("../../tests/fixtures/home_pesquisa.html");
    const FIXTURE_OK: &str = include_str!("../../tests/fixtures/resposta_ok.xml");
    const FIXTURE_PAGINA2: &str = include_str!("../../tests/fixtures/resposta_pagina2.xml");
    const FIXTURE_DUPLICADA: &str = include_str!("../../tests/fixtures/resposta_duplicada.xml");

    type Campos = Vec<(String, String)>;
    type ChamadaPost = (String, Campos);

    /// Dublê síncrono de `HttpPort`: GET sempre devolve `home`; POST devolve,
    /// em ordem, cada resposta enfileirada em `respostas_post`. Guarda todas
    /// as chamadas para os testes inspecionarem URL e campos enviados.
    struct FakeHttp {
        home: String,
        respostas_post: RefCell<VecDeque<String>>,
        chamadas_get: RefCell<Vec<String>>,
        chamadas_post: RefCell<Vec<ChamadaPost>>,
    }

    impl FakeHttp {
        fn novo(home: &str, respostas_post: &[&str]) -> Self {
            Self {
                home: home.to_string(),
                respostas_post: RefCell::new(
                    respostas_post.iter().map(|s| s.to_string()).collect(),
                ),
                chamadas_get: RefCell::new(Vec::new()),
                chamadas_post: RefCell::new(Vec::new()),
            }
        }
    }

    impl HttpPort for FakeHttp {
        fn get(&self, url: &str) -> Result<String, AppError> {
            self.chamadas_get.borrow_mut().push(url.to_string());
            Ok(self.home.clone())
        }

        fn post_form(&self, url: &str, campos: &[(String, String)]) -> Result<String, AppError> {
            self.chamadas_post
                .borrow_mut()
                .push((url.to_string(), campos.to_vec()));
            self.respostas_post
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| AppError::FalhaPortal {
                    motivo: "FakeHttp: nenhuma resposta configurada para este POST".to_string(),
                })
        }

        fn get_bytes(&self, _url: &str) -> Result<Vec<u8>, AppError> {
            // Não exercitado pelos testes de busca (US1/US2/US3); download é
            // US4, coberto em `services::downloader`.
            Err(AppError::FalhaPortal {
                motivo: "FakeHttp: get_bytes não implementado neste dublê".to_string(),
            })
        }
    }

    fn ids_teste() -> Ids {
        Ids {
            form: "j_idt65".to_string(),
            btn: "j_idt65:j_idt115".to_string(),
            datalist: "j_idt65:dataList".to_string(),
            action: PAGE.to_string(),
            viewstate: "vs-inicial".to_string(),
        }
    }

    // --- descobrir_ids ------------------------------------------------- //

    #[test]
    fn descobre_ids_do_fixture_home() {
        let ids = descobrir_ids(FIXTURE_HOME).expect("deve descobrir os ids");
        assert_eq!(ids.btn, "j_idt65:j_idt115");
        assert_eq!(ids.form, "j_idt65");
        assert_eq!(ids.datalist, "j_idt65:dataList");
        assert_eq!(
            ids.action,
            "/faces/pesquisarDocumentos/pesquisarHistorico.xhtml"
        );
        assert_eq!(ids.viewstate, "-1234567890123456789:-9876543210987654321");
    }

    #[test]
    fn erro_de_layout_quando_home_sem_viewstate_ou_botao() {
        let erro =
            descobrir_ids("<html><body>layout completamente diferente</body></html>").unwrap_err();
        assert!(matches!(erro, AppError::FalhaPortal { .. }));
    }

    // --- extrair_viewstate_parcial -------------------------------------- //

    #[test]
    fn extrai_viewstate_de_resposta_parcial() {
        let xml =
            r#"<update id="j_idt1:javax.faces.ViewState:0"><![CDATA[abc123:def456]]></update>"#;
        assert_eq!(
            extrair_viewstate_parcial(xml),
            Some("abc123:def456".to_string())
        );
    }

    #[test]
    fn extrai_viewstate_none_quando_resposta_nao_traz() {
        assert_eq!(extrair_viewstate_parcial(FIXTURE_OK), None);
    }

    // --- campos_busca / campos_pagina ------------------------------------ //

    #[test]
    fn campos_busca_inclui_termo_repositorio_e_ordenacao_padrao() {
        let ids = ids_teste();
        let campos = campos_busca(&ids, "1998547", "1");

        assert!(campos.contains(&("j_idt65:nome".to_string(), "1998547".to_string())));
        assert!(campos.contains(&("j_idt65:shardItems_input".to_string(), "1".to_string())));
        assert!(campos.contains(&("j_idt65:campo_input".to_string(), "RELEVANCIA".to_string())));
        assert!(campos.contains(&("j_idt65:ordem_input".to_string(), "DECRESCENTE".to_string())));
        assert!(campos.contains(&(
            "javax.faces.source".to_string(),
            "j_idt65:j_idt115".to_string()
        )));
        assert!(
            !campos.iter().any(|(k, _)| k == "javax.faces.ViewState"),
            "ViewState é acrescentado por buscar(), não por campos_busca"
        );
    }

    #[test]
    fn campos_pagina_inclui_offset_e_tamanho_da_pagina() {
        let ids = ids_teste();
        let campos = campos_pagina(&ids, 20, ROWS_POR_PAGINA);

        assert!(campos.contains(&("j_idt65:dataList_first".to_string(), "20".to_string())));
        assert!(campos.contains(&("j_idt65:dataList_rows".to_string(), "10".to_string())));
        assert!(campos.contains(&(
            "javax.faces.source".to_string(),
            "j_idt65:dataList".to_string()
        )));
    }

    // --- GedocRepositoryHttp::buscar (orquestração, sem rede) ------------ //

    #[test]
    fn buscar_descobre_ids_e_envia_termo_e_repositorio_no_post() {
        let http = FakeHttp::novo(FIXTURE_HOME, &[FIXTURE_DUPLICADA]);
        let repo = GedocRepositoryHttp::novo(http);

        let (total, docs) = repo
            .buscar("1998547", "1")
            .expect("busca sem rede deve funcionar");

        assert_eq!(total, 1);
        assert_eq!(
            docs.len(),
            1,
            "resposta com link duplicado já vem deduplicada"
        );

        let chamadas_get = repo.http.chamadas_get.borrow();
        assert_eq!(chamadas_get[0], format!("{BASE}{PAGE}"));

        let chamadas_post = repo.http.chamadas_post.borrow();
        let (url, campos) = &chamadas_post[0];
        assert_eq!(url, &format!("{BASE}{PAGE}"));
        assert!(campos.contains(&("j_idt65:nome".to_string(), "1998547".to_string())));
        assert!(campos.contains(&("j_idt65:shardItems_input".to_string(), "1".to_string())));
    }

    #[test]
    fn buscar_agrega_documentos_de_multiplas_paginas() {
        // Página 1 (busca): 3 docs, total relatado 1.234. Página 2: mais 2
        // docs novos. Página 3: repete a 1ª resposta (nenhum link novo) --
        // para a paginação mesmo sem ter alcançado o total (FR-001 + dedup).
        let http = FakeHttp::novo(FIXTURE_HOME, &[FIXTURE_OK, FIXTURE_PAGINA2, FIXTURE_OK]);
        let repo = GedocRepositoryHttp::novo(http);

        let (total, docs) = repo.buscar("1998547", "1").expect("deve paginar sem rede");

        assert_eq!(total, 1234);
        assert_eq!(docs.len(), 5, "3 da 1ª página + 2 novos da 2ª página");
        assert!(docs
            .iter()
            .any(|d| d.link.contains("aaaa1111bbbb2222cccc3333dddd4444")));
        assert!(docs
            .iter()
            .any(|d| d.link.contains("eeee5555ffff6666aaaa7777bbbb8888")));
    }

    /// Resposta mínima de 1 documento, `total=1` (sem paginação) — para testar
    /// a agregação de repositórios sem enfileirar páginas.
    fn resp_shard(hex32: &str, titulo: &str, data: &str) -> String {
        format!(
            "1 registro <a href=\"/faces/documento/{hex32}?inline\" \
             class=\"resultadoBuscaLinhaAzul\">{titulo}</a> \
             <span class=\"resultadoBuscaLinhaVerde\"> {data}</span>"
        )
    }

    #[test]
    fn buscar_todos_agrega_os_repositorios_e_inclui_docs_antigos() {
        // Default ("todos"): agrega os 3 shards. Shard 0 (Boletim) traz um doc
        // de 2006 que a busca só-GeDoc (shard 1) perdia — causa do bug real.
        let boletim = resp_shard(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "PORTARIA 483 - 2006",
            "02/08/2006",
        );
        let gedoc = resp_shard(
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "PORTARIA 10 - 2020",
            "10/01/2020",
        );
        let site = resp_shard(
            "cccccccccccccccccccccccccccccccc",
            "PORTARIA 5 - 2019",
            "05/05/2019",
        );
        let http = FakeHttp::novo(
            FIXTURE_HOME,
            &[boletim.as_str(), gedoc.as_str(), site.as_str()],
        );
        let repo = GedocRepositoryHttp::novo(http);

        let (total, docs) = repo
            .buscar("1466728", "todos")
            .expect("deve agregar os repositórios");

        assert_eq!(total, 3, "soma dos totais (1 por repositório)");
        assert_eq!(docs.len(), 3);
        assert!(
            docs.iter().any(|d| d.data.as_deref() == Some("02/08/2006")),
            "inclui o documento antigo do Boletim"
        );
    }

    #[test]
    fn buscar_repositorio_concreto_usa_so_aquele_shard() {
        // Repositório "0" (Boletim) → um único shard (uma busca), sem agregar.
        let boletim = resp_shard(
            "dddddddddddddddddddddddddddddddd",
            "PORTARIA 2006",
            "02/08/2006",
        );
        let http = FakeHttp::novo(FIXTURE_HOME, &[boletim.as_str()]);
        let repo = GedocRepositoryHttp::novo(http);

        let (total, docs) = repo.buscar("1466728", "0").expect("shard único");

        assert_eq!(total, 1);
        assert_eq!(docs.len(), 1);
    }

    #[test]
    fn buscar_para_de_paginar_quando_pagina_nao_traz_link_novo() {
        // 2 respostas apenas: a 2ª repete os mesmos links da 1ª. Mesmo o
        // total relatado (1.234) sendo muito maior que os documentos
        // coletados, a busca deve parar em vez de tentar por sempre.
        let http = FakeHttp::novo(FIXTURE_HOME, &[FIXTURE_OK, FIXTURE_OK]);
        let repo = GedocRepositoryHttp::novo(http);

        let (total, docs) = repo
            .buscar("1998547", "1")
            .expect("não deve entrar em loop");

        assert_eq!(total, 1234);
        assert_eq!(docs.len(), 3);
        assert_eq!(
            repo.http.chamadas_post.borrow().len(),
            2,
            "não deve chamar POST de novo após página vazia"
        );
    }

    #[test]
    fn buscar_propaga_falha_de_layout_quando_home_nao_tem_ids() {
        let http = FakeHttp::novo("<html><body>sem nada aqui</body></html>", &[]);
        let repo = GedocRepositoryHttp::novo(http);

        let erro = repo.buscar("1998547", "1").unwrap_err();
        assert!(matches!(erro, AppError::FalhaPortal { .. }));
    }
}
