//! Resumidor (US6) — resume cada documento a partir do texto extraído do PDF
//! já baixado (`services::texto_pdf`) ou, na ausência de um PDF legível em
//! disco, do trecho da busca (fallback). Espelha `src/resumir_mistral.py`
//! (`SISTEMA`, `resumir`, `resolver_resumo`): mesmo prompt, mesmo cache por
//! link (R6) e a mesma filosofia de nunca abortar o lote por causa de 1
//! documento (R11) — uma falha na chamada de IA deixa `doc.resumo` em
//! `None` e o processamento segue para o próximo. Ao contrário da
//! classificação (`services::classificador`), não há fallback "sem IA" para
//! o resumo: sem `ChatIa` configurado, simplesmente não se resume (quem
//! chama só invoca `resumir_lote` quando há uma chave configurada).
//!
//! O texto extraído do PDF pode conter PII de terceiros (Princípio
//! II/LGPD); só existe em memória durante esta chamada — nunca é logado nem
//! persistido. Apenas o resumo final (já uma síntese) entra no cache, em
//! `app_data_dir`, fora do VCS.

use std::path::Path;

use serde_json::Value;

use crate::domain::documento::Documento;
use crate::domain::nome_arquivo::nome_arquivo;
use crate::error::AppError;
use crate::ports::ia::ChatIa;
use crate::services::cache::CacheArquivo;
use crate::services::downloader;
use crate::services::texto_pdf;

/// Prompt fixo (PT-BR), fiel a `SISTEMA` de `src/resumir_mistral.py`: 2-3
/// frases objetivas, sem inventar dados (R1), sem repetir o título.
const SISTEMA: &str = "Você resume documentos administrativos do IFES (portarias, despachos). \
Escreva de 2 a 3 frases objetivas em português, informando o que o documento \
determina, o órgão/campus, pessoas ou comissões envolvidas, datas e a \
finalidade. Não invente dados que não estejam no texto. Não repita o título.";

/// Marcador devolvido (e cacheado) quando não há nenhum texto-fonte
/// disponível (nem PDF, nem trecho) — espelha `"_(sem texto)_"` do Python;
/// sem asteriscos aqui porque a View em Vue não renderiza Markdown.
const SEM_TEXTO: &str = "(sem texto)";

/// Nº máximo de caracteres enviados por documento (espelha `MAX_CHARS = 6000`
/// do Python de referência), aplicado ao texto-fonte final — PDF ou trecho,
/// qualquer que seja a origem.
const MAX_CHARS: usize = 6000;

/// Spec 011 — nº de documentos por chamada de resumo em lote. Pequeno porque o
/// texto-fonte (PDF) é grande: 5 × `MAX_CHARS` já é um prompt considerável.
const TAMANHO_LOTE_RESUMO: usize = 5;

/// Temperatura do resumo (espelha `TEMPERATURE_RESUMO` do adapter) usada na
/// chamada de lote (`ChatIa::chat_lote`).
const TEMPERATURE_RESUMO: f64 = 0.2;

/// Resume cada documento de `docs`, em lugar (`doc.resumo`). Mesma
/// orquestração de `services::classificador::classificar_lote`: cache por
/// link (R6) evita nova chamada de IA para um documento já resumido; a
/// falha ao resumir 1 documento não aborta o lote (R11) — o resumo fica
/// `None` e o processamento segue para o próximo.
pub fn resumir_lote<C: ChatIa + ?Sized>(
    docs: &mut [Documento],
    siape: &str,
    chat: &C,
    dir_documentos: &Path,
    mut cache: Option<&mut CacheArquivo>,
) {
    // 1) Cache hits (R6) e documentos sem texto-fonte ("(sem texto)") são
    //    resolvidos na hora, sem IA — não entram em nenhum lote. Os demais
    //    (têm texto e não estão cacheados) guardam (índice, texto) para o lote.
    let mut pendentes: Vec<(usize, String)> = Vec::new();
    for (i, doc) in docs.iter_mut().enumerate() {
        if let Some(resumo) = cache.as_deref().and_then(|c| c.obter(&doc.link)) {
            doc.resumo = Some(resumo.to_string());
            continue;
        }
        let texto = texto_fonte(doc, siape, dir_documentos);
        if texto.is_empty() {
            doc.resumo = Some(SEM_TEXTO.to_string());
            if let Some(c) = cache.as_deref_mut() {
                c.inserir(doc.link.clone(), SEM_TEXTO.to_string());
                let _ = c.salvar();
            }
            continue;
        }
        pendentes.push((i, texto));
    }

    // 2) Os pendentes vão em blocos de TAMANHO_LOTE_RESUMO — 1 chamada por
    //    bloco (spec 011) em vez de 1 por documento.
    for bloco in pendentes.chunks(TAMANHO_LOTE_RESUMO) {
        resumir_bloco_llm(
            docs,
            bloco,
            siape,
            chat,
            dir_documentos,
            cache.as_deref_mut(),
        );
    }
}

/// Resume um bloco (todos cache-miss, com texto) numa única chamada de IA
/// (`ChatIa::chat_lote`), ancorando cada resumo por índice (FR-002/spec 011).
/// Fidelidade (Princípio I): um índice ausente na resposta — ou a chamada do
/// bloco falhando — cai no resumo **por-documento** (`resolver_resumo`) só
/// para aquele item; nunca se aceita um resumo não confirmado como sendo de
/// outro documento.
fn resumir_bloco_llm<C: ChatIa + ?Sized>(
    docs: &mut [Documento],
    bloco: &[(usize, String)],
    siape: &str,
    chat: &C,
    dir_documentos: &Path,
    mut cache: Option<&mut CacheArquivo>,
) {
    let itens: Vec<(usize, &str)> = bloco
        .iter()
        .enumerate()
        .map(|(pos, (_, texto))| (pos, texto.as_str()))
        .collect();
    let (sistema, usuario) = montar_prompt_resumo_lote(&itens);
    let resultados = match chat.chat_lote(&sistema, &usuario, TEMPERATURE_RESUMO) {
        Ok(resposta) => extrair_resumos_lote(&resposta, bloco.len()),
        Err(_) => vec![None; bloco.len()],
    };

    for (pos, (i, _texto)) in bloco.iter().enumerate() {
        match resultados.get(pos).cloned().flatten() {
            Some(resumo) => {
                docs[*i].resumo = Some(resumo.clone());
                if let Some(c) = cache.as_deref_mut() {
                    c.inserir(docs[*i].link.clone(), resumo);
                    let _ = c.salvar();
                }
            }
            // Item não confirmado no lote → resumo por-documento (fidelidade).
            None => {
                docs[*i].resumo =
                    resolver_resumo(&docs[*i], siape, chat, dir_documentos, cache.as_deref_mut());
            }
        }
    }
}

/// Monta (sistema, usuário) para o resumo em lote: cada documento numerado por
/// índice `[i]`; a IA responde só com `{"itens":[{"i":<indice>,"resumo":"..."}]}`.
fn montar_prompt_resumo_lote(itens: &[(usize, &str)]) -> (String, String) {
    let sistema = format!(
        "{SISTEMA} Você recebe VÁRIOS documentos, cada um com um índice [i]. \
Resuma CADA documento (2-3 frases, fiel ao texto dele, sem misturar com os \
outros). Responda apenas em JSON: \
{{\"itens\":[{{\"i\":<indice>,\"resumo\":\"<resumo>\"}}]}}."
    );
    let corpo = itens
        .iter()
        .map(|(i, texto)| format!("[{i}]\n{texto}"))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");
    (sistema, format!("Documentos:\n\n{corpo}"))
}

/// Extrai os resumos do JSON de lote (`{"itens":[{"i","resumo"}]}`), ancorados
/// por índice. Devolve `Vec` de tamanho `n`: `Some(resumo)` para cada índice
/// confirmado (não vazio), `None` para ausente. JSON inválido → todos `None`
/// (o chamador faz o fallback por-documento). Nunca panica (R11).
fn extrair_resumos_lote(resposta: &str, n: usize) -> Vec<Option<String>> {
    let mut out = vec![None; n];
    let Ok(valor) = serde_json::from_str::<Value>(resposta) else {
        return out;
    };
    let Some(itens) = valor.get("itens").and_then(Value::as_array) else {
        return out;
    };
    for item in itens {
        let indice = item.get("i").and_then(Value::as_u64);
        let resumo = item.get("resumo").and_then(Value::as_str);
        if let (Some(indice), Some(resumo)) = (indice, resumo) {
            let indice = indice as usize;
            let resumo = resumo.trim();
            if indice < n && !resumo.is_empty() {
                out[indice] = Some(resumo.to_string());
            }
        }
    }
    out
}

/// Resolve o resumo de `doc`: cache (hit) ou IA (miss); só grava no cache o
/// resultado de uma chamada bem-sucedida (R6) — inclui o marcador
/// `SEM_TEXTO`, que também é um resultado "de sucesso" (não houve erro de
/// IA; simplesmente não havia o que resumir). Uma falha na chamada devolve
/// `None` sem cachear, permitindo nova tentativa numa busca futura.
fn resolver_resumo<C: ChatIa + ?Sized>(
    doc: &Documento,
    siape: &str,
    chat: &C,
    dir_documentos: &Path,
    cache: Option<&mut CacheArquivo>,
) -> Option<String> {
    if let Some(resumo) = cache.as_deref().and_then(|c| c.obter(&doc.link)) {
        return Some(resumo.to_string());
    }

    match resumir_via_ia(doc, siape, chat, dir_documentos) {
        Ok(resumo) => {
            if let Some(cache) = cache {
                cache.inserir(doc.link.clone(), resumo.clone());
                // Falha ao persistir o cache não pode abortar o resumo do
                // lote (R11) — na pior hipótese, resume de novo no futuro.
                let _ = cache.salvar();
            }
            Some(resumo)
        }
        Err(_) => None,
    }
}

/// Resume `doc` via IA a partir do texto-fonte resolvido (R1 — deriva do
/// texto real, nunca inventa); quando não há texto algum, devolve o
/// marcador `SEM_TEXTO` sem chamar a IA.
fn resumir_via_ia<C: ChatIa + ?Sized>(
    doc: &Documento,
    siape: &str,
    chat: &C,
    dir_documentos: &Path,
) -> Result<String, AppError> {
    let texto = texto_fonte(doc, siape, dir_documentos);
    if texto.is_empty() {
        return Ok(SEM_TEXTO.to_string());
    }
    chat.resumir(SISTEMA, &texto)
}

/// Texto-fonte de `doc`: texto extraído do PDF já baixado (se `doc.arquivo`
/// apontar para um arquivo existente em `dir_documentos/<siape>/`), senão o
/// trecho da busca. Truncado a `MAX_CHARS`, qualquer que seja a origem —
/// espelha `texto[:MAX_CHARS]` do Python de referência.
fn texto_fonte(doc: &Documento, siape: &str, dir_documentos: &Path) -> String {
    let texto = texto_do_pdf(doc, siape, dir_documentos)
        .or_else(|| doc.trecho.clone())
        .unwrap_or_default();
    truncar(texto.trim(), MAX_CHARS)
}

/// Lê e extrai o texto do PDF de `doc`, se ele já tiver sido baixado sob
/// `dir_documentos/<siape>/` (R7 — mesmo caminho seguro do download/abertura,
/// `downloader::caminho_seguro`). O nome do PDF é **determinístico** (R3):
/// derivamos com `nome_arquivo(doc)` — a mesma função que `downloader` usa
/// para gravar — em vez de depender de `doc.arquivo`, que não é populado no
/// fluxo de busca (só o download conhece o nome). Se `doc.arquivo` já vier
/// preenchido (chamadas futuras), ele tem prioridade. Qualquer falha (nome
/// inválido, arquivo ausente, PDF ilegível) vira `None` — o chamador cai no
/// trecho.
fn texto_do_pdf(doc: &Documento, siape: &str, dir_documentos: &Path) -> Option<String> {
    let arquivo = doc.arquivo.clone().unwrap_or_else(|| nome_arquivo(doc));
    let caminho = downloader::caminho_seguro(dir_documentos, siape, &arquivo).ok()?;
    let bytes = std::fs::read(caminho).ok()?;
    texto_pdf::extrair_texto(&bytes)
}

/// Trunca por `char` (não por byte), para nunca quebrar um caractere UTF-8.
fn truncar(texto: &str, max_chars: usize) -> String {
    texto.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    const PDF_FIXTURE: &[u8] = include_bytes!("../../tests/fixtures/documento_teste.pdf");
    const SIAPE_TESTE: &str = "1998547";

    /// Dublê com filas separadas: `lote` (respostas de `chat_lote`, spec 011)
    /// e `doc` (respostas do fallback por-documento, `resumir`). `chamadas`
    /// conta ambas.
    struct ChatFake {
        lote: RefCell<VecDeque<Result<String, AppError>>>,
        doc: RefCell<VecDeque<Result<String, AppError>>>,
        chamadas: RefCell<u32>,
    }

    impl ChatFake {
        fn novo(lote: Vec<Result<String, AppError>>, doc: Vec<Result<String, AppError>>) -> Self {
            Self {
                lote: RefCell::new(lote.into()),
                doc: RefCell::new(doc.into()),
                chamadas: RefCell::new(0),
            }
        }
    }

    impl ChatIa for ChatFake {
        fn chat(&self, _sistema: &str, _usuario: &str) -> Result<String, AppError> {
            unreachable!("resumir_lote usa chat_lote()/resumir(), não chat()")
        }

        fn resumir(&self, _sistema: &str, _usuario: &str) -> Result<String, AppError> {
            *self.chamadas.borrow_mut() += 1;
            self.doc
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Ok("resumo padrão".to_string()))
        }

        fn chat_lote(
            &self,
            _sistema: &str,
            _usuario: &str,
            _temperatura: f64,
        ) -> Result<String, AppError> {
            *self.chamadas.borrow_mut() += 1;
            self.lote
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Ok(r#"{"itens":[]}"#.to_string()))
        }
    }

    fn doc(link: &str, titulo: &str) -> Documento {
        Documento::novo(link, titulo)
    }

    // --- texto_fonte: PDF vs trecho ------------------------------------- //

    #[test]
    fn usa_o_trecho_quando_nao_ha_arquivo_baixado() {
        let dir = tempdir().expect("tempdir");
        let mut d = doc("l1", "PORTARIA Nº 1 - 2024 - Assunto");
        d.trecho = Some("Determina a progressão do servidor.".to_string());

        assert_eq!(
            texto_fonte(&d, SIAPE_TESTE, dir.path()),
            "Determina a progressão do servidor."
        );
    }

    #[test]
    fn usa_o_texto_extraido_do_pdf_quando_o_arquivo_existe_em_disco() {
        let dir = tempdir().expect("tempdir");
        let pasta_siape = dir.path().join(SIAPE_TESTE);
        fs::create_dir_all(&pasta_siape).expect("cria pasta do siape");
        fs::write(pasta_siape.join("doc.pdf"), PDF_FIXTURE).expect("grava fixture");

        let mut d = doc("l1", "PORTARIA Nº 1 - 2024 - Assunto");
        d.arquivo = Some("doc.pdf".to_string());
        d.trecho = Some("trecho não deveria ser usado quando há PDF".to_string());

        let texto = texto_fonte(&d, SIAPE_TESTE, dir.path());
        assert!(
            texto.contains("Documento de teste"),
            "deveria usar o texto do PDF, obteve: {texto:?}"
        );
    }

    #[test]
    fn usa_o_pdf_baixado_pelo_nome_deterministico_mesmo_sem_doc_arquivo() {
        // Fluxo real de produção: `doc.arquivo` é None (a busca não o popula),
        // mas o PDF foi baixado sob o nome determinístico (R3). O resumidor
        // deve encontrá-lo derivando o nome com `nome_arquivo`, não via
        // `doc.arquivo` — este é o bug que a revisão de #6 apontou.
        let dir = tempdir().expect("tempdir");
        let pasta_siape = dir.path().join(SIAPE_TESTE);
        fs::create_dir_all(&pasta_siape).expect("cria pasta do siape");

        let d = doc("l1", "PORTARIA Nº 1 - 2024 - Assunto");
        assert!(d.arquivo.is_none(), "cenário: busca não popula arquivo");
        let nome = crate::domain::nome_arquivo::nome_arquivo(&d);
        fs::write(pasta_siape.join(&nome), PDF_FIXTURE).expect("grava PDF baixado");

        let texto = texto_fonte(&d, SIAPE_TESTE, dir.path());
        assert!(
            texto.contains("Documento de teste"),
            "deveria achar o PDF pelo nome derivado, obteve: {texto:?}"
        );
    }

    #[test]
    fn cai_no_trecho_quando_arquivo_referenciado_nao_existe_em_disco() {
        let dir = tempdir().expect("tempdir");
        let mut d = doc("l1", "PORTARIA Nº 1 - 2024 - Assunto");
        d.arquivo = Some("nao_existe.pdf".to_string());
        d.trecho = Some("trecho de fallback".to_string());

        assert_eq!(
            texto_fonte(&d, SIAPE_TESTE, dir.path()),
            "trecho de fallback"
        );
    }

    // --- resumir_lote ----------------------------------------------------- //

    #[test]
    fn resume_com_sucesso_a_partir_do_trecho() {
        let dir = tempdir().expect("tempdir");
        let chat = ChatFake::novo(
            vec![Ok(
                r#"{"itens":[{"i":0,"resumo":"Resumo do trecho."}]}"#.to_string()
            )],
            vec![],
        );
        let mut docs = vec![doc("l1", "PORTARIA Nº 1 - 2024 - Assunto")];
        docs[0].trecho = Some("Determina a progressão do servidor.".to_string());

        resumir_lote(&mut docs, SIAPE_TESTE, &chat, dir.path(), None);

        assert_eq!(docs[0].resumo.as_deref(), Some("Resumo do trecho."));
        assert_eq!(*chat.chamadas.borrow(), 1);
    }

    #[test]
    fn sem_nenhum_texto_fonte_usa_o_marcador_sem_texto_sem_chamar_a_ia() {
        let dir = tempdir().expect("tempdir");
        let chat = ChatFake::novo(vec![], vec![]);
        let mut docs = vec![doc("l1", "PORTARIA Nº 1 - 2024 - Assunto")]; // sem trecho, sem arquivo

        resumir_lote(&mut docs, SIAPE_TESTE, &chat, dir.path(), None);

        assert_eq!(docs[0].resumo.as_deref(), Some(SEM_TEXTO));
        assert_eq!(*chat.chamadas.borrow(), 0, "não deve chamar a IA sem texto");
    }

    #[test]
    fn falha_de_um_documento_nao_aborta_o_lote_r11() {
        let dir = tempdir().expect("tempdir");
        // O lote confirma só o índice 1 → o doc 0 cai no fallback por-doc, que
        // aqui falha → resumo None (R11); o doc 1 vem do lote.
        let chat = ChatFake::novo(
            vec![Ok(
                r#"{"itens":[{"i":1,"resumo":"Resumo do segundo."}]}"#.to_string()
            )],
            vec![Err(AppError::FalhaIA {
                motivo: "instável".to_string(),
            })],
        );
        let mut docs = vec![
            doc("l1", "PORTARIA Nº 1 - 2024 - Assunto A"),
            doc("l2", "PORTARIA Nº 2 - 2024 - Assunto B"),
        ];
        docs[0].trecho = Some("trecho A".to_string());
        docs[1].trecho = Some("trecho B".to_string());

        resumir_lote(&mut docs, SIAPE_TESTE, &chat, dir.path(), None);

        assert_eq!(
            docs[0].resumo, None,
            "falha na IA deixa o resumo None (R11)"
        );
        assert_eq!(docs[1].resumo.as_deref(), Some("Resumo do segundo."));
    }

    #[test]
    fn cache_evita_chamar_a_ia_de_novo_para_o_mesmo_link_r6() {
        let dir = tempdir().expect("tempdir");
        let mut cache = CacheArquivo::carregar(dir.path().join("resumo.json"));
        let chat = ChatFake::novo(
            vec![Ok(
                r#"{"itens":[{"i":0,"resumo":"Resumo cacheado."}]}"#.to_string()
            )],
            vec![],
        );

        let mut primeira = vec![doc("l1", "PORTARIA Nº 1 - 2024 - Assunto")];
        primeira[0].trecho = Some("trecho".to_string());
        resumir_lote(
            &mut primeira,
            SIAPE_TESTE,
            &chat,
            dir.path(),
            Some(&mut cache),
        );
        assert_eq!(*chat.chamadas.borrow(), 1);

        let mut segunda = vec![doc("l1", "PORTARIA Nº 1 - 2024 - Assunto")]; // mesmo link
        segunda[0].trecho = Some("trecho".to_string());
        resumir_lote(
            &mut segunda,
            SIAPE_TESTE,
            &chat,
            dir.path(),
            Some(&mut cache),
        );

        assert_eq!(segunda[0].resumo.as_deref(), Some("Resumo cacheado."));
        assert_eq!(
            *chat.chamadas.borrow(),
            1,
            "cache hit não chama a IA de novo (R6)"
        );
    }

    #[test]
    fn nao_cacheia_resultado_de_falha_permitindo_nova_tentativa() {
        let dir = tempdir().expect("tempdir");
        let mut cache = CacheArquivo::carregar(dir.path().join("resumo.json"));
        // 1ª busca: lote falha e fallback por-doc falha → None, não cacheia.
        // 2ª busca: lote responde → cacheia.
        let chat = ChatFake::novo(
            vec![
                Err(AppError::FalhaIA {
                    motivo: "instável".to_string(),
                }),
                Ok(r#"{"itens":[{"i":0,"resumo":"Resumo na 2ª tentativa."}]}"#.to_string()),
            ],
            vec![Err(AppError::FalhaIA {
                motivo: "instável".to_string(),
            })],
        );

        let mut primeira = vec![doc("l1", "PORTARIA Nº 1 - 2024 - Assunto")];
        primeira[0].trecho = Some("trecho".to_string());
        resumir_lote(
            &mut primeira,
            SIAPE_TESTE,
            &chat,
            dir.path(),
            Some(&mut cache),
        );
        assert_eq!(cache.obter("l1"), None, "falha não deve ser cacheada");

        let mut segunda = vec![doc("l1", "PORTARIA Nº 1 - 2024 - Assunto")];
        segunda[0].trecho = Some("trecho".to_string());
        resumir_lote(
            &mut segunda,
            SIAPE_TESTE,
            &chat,
            dir.path(),
            Some(&mut cache),
        );

        assert_eq!(
            segunda[0].resumo.as_deref(),
            Some("Resumo na 2ª tentativa."),
            "sem cache de erro, a 2ª tentativa via lote resume e cacheia"
        );
    }
}
