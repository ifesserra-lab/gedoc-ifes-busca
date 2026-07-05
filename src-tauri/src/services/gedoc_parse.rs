//! Parser puro da resposta parcial (AJAX/PrimeFaces) do portal GeDoc.
//! Recebe a string de resposta e devolve structs — sem I/O, sem rede
//! (Princípio VII: testável com fixtures locais). Fonte de referência
//! (legado): `src/buscar_gedoc.py::parse_resposta`.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use crate::domain::documento::{Documento, TipoDocumento};
use crate::domain::texto::{extrair_ano, extrair_numero};
use crate::error::AppError;

static RE_ERRO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"ui-messages-error-detail">([^<]*)"#).unwrap());
static RE_TOTAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([\d.]+)\s*registro").unwrap());
static RE_DOC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)<a href="([^"]*?/documento/[0-9A-Fa-f]{32}[^"]*)"[^>]*class="resultadoBuscaLinhaAzul">(.*?)</a>"#,
    )
    .unwrap()
});
static RE_DATA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"resultadoBuscaLinhaVerde">\s*(\d{2}/\d{2}/\d{4})"#).unwrap());
static RE_HIGHLIGHT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)class="highlight">(.*?)</div>"#).unwrap());
static RE_SIAPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)SIAPE\s*(?:n?º?\s*|:\s*)?([0-9]{5,8})\b").unwrap());
static RE_TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static RE_ESPACOS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

/// Um documento tal como aparece na resposta do portal, ainda não
/// classificado nem filtrado (isso é papel de `domain`/`filtro`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentoParseado {
    pub link: String,
    pub titulo: String,
    pub data: Option<String>,
    pub trecho: String,
    pub siapes: Vec<String>,
}

impl DocumentoParseado {
    /// Converte para o Model de domínio (`Documento`), derivando tipo/número/
    /// ano do título (R1: dado deriva do conteúdo real).
    pub fn para_documento(self) -> Documento {
        let tipo = TipoDocumento::from_titulo(&self.titulo);
        let numero = extrair_numero(&self.titulo);
        let ano = extrair_ano(&self.titulo, self.data.as_deref());
        Documento {
            link: self.link,
            tipo,
            numero,
            ano,
            data: self.data,
            trecho: if self.trecho.is_empty() {
                None
            } else {
                Some(self.trecho)
            },
            siapes: self.siapes,
            arquivo: None,
            contem_siape: true,
            categoria: None,
            resumo: None,
            titulo: self.titulo,
        }
    }
}

/// Resultado bruto do parse de uma resposta parcial: total relatado pelo
/// portal (pode ser `None` se a resposta não trouxer contagem) e os
/// documentos únicos (deduplicados por link, preservando a ordem).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RespostaParseada {
    pub total: Option<u32>,
    pub documentos: Vec<DocumentoParseado>,
}

/// Extrai `(total, documentos)` de uma resposta parcial JSF/PrimeFaces.
///
/// Erros de negócio do portal (mensagem de erro do servidor, sessão expirada
/// via `<redirect>`) viram `AppError::FalhaPortal` em vez de pânico — a
/// chamada de rede real fica a cargo de `GedocRepository` (TODO).
pub fn parse_resposta(xml: &str) -> Result<RespostaParseada, AppError> {
    if let Some(cap) = RE_ERRO.captures(xml) {
        let msg = cap.get(1).map(|g| g.as_str().trim()).unwrap_or_default();
        return Err(AppError::FalhaPortal {
            motivo: format!("Erro do servidor: {msg}"),
        });
    }
    if xml.contains("<redirect") {
        return Err(AppError::FalhaPortal {
            motivo: "Sessão expirada (redirect). Refaça a busca.".to_string(),
        });
    }

    let total = RE_TOTAL.captures(xml).and_then(|c| {
        c.get(1)
            .and_then(|g| g.as_str().replace('.', "").parse::<u32>().ok())
    });

    let anchors: Vec<_> = RE_DOC.captures_iter(xml).collect();
    let mut vistos = HashSet::new();
    let mut documentos = Vec::with_capacity(anchors.len());

    for (i, cap) in anchors.iter().enumerate() {
        let whole = cap.get(0).unwrap();
        let fim = anchors
            .get(i + 1)
            .map(|c| c.get(0).unwrap().start())
            .unwrap_or(xml.len());
        let bloco = &xml[whole.end()..fim];

        let link = cap.get(1).unwrap().as_str().to_string();
        if !vistos.insert(link.clone()) {
            continue; // dedup preservando a ordem da primeira ocorrência
        }

        let titulo = texto_limpo(cap.get(2).unwrap().as_str());
        let data = RE_DATA
            .captures(bloco)
            .and_then(|c| c.get(1))
            .map(|g| g.as_str().to_string());
        let trecho = RE_HIGHLIGHT
            .captures(bloco)
            .and_then(|c| c.get(1))
            .map(|g| texto_limpo(g.as_str()))
            .unwrap_or_default();
        let siapes = RE_SIAPE
            .captures_iter(&trecho)
            .map(|c| c[1].to_string())
            .collect();

        documentos.push(DocumentoParseado {
            link,
            titulo,
            data,
            trecho,
            siapes,
        });
    }

    Ok(RespostaParseada { total, documentos })
}

/// Remove tags HTML, decodifica entidades comuns e normaliza espaços —
/// equivalente a `buscar_gedoc.py::_texto`.
fn texto_limpo(fragmento: &str) -> String {
    let sem_tags = RE_TAGS.replace_all(fragmento, " ");
    let sem_entidades = decodificar_entidades(&sem_tags);
    RE_ESPACOS
        .replace_all(&sem_entidades, " ")
        .trim()
        .to_string()
}

/// Decodifica as entidades HTML mais comuns nas páginas do portal. Não é um
/// decodificador HTML completo (YAGNI) — cobre o necessário para os textos
/// vistos no GeDoc (acentuação, `&amp;`, `&nbsp;`).
fn decodificar_entidades(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_OK: &str = include_str!("../../tests/fixtures/resposta_ok.xml");
    const FIXTURE_ERRO: &str = include_str!("../../tests/fixtures/resposta_erro.xml");
    const FIXTURE_REDIRECT: &str = include_str!("../../tests/fixtures/resposta_redirect.xml");
    const FIXTURE_DUPLICADA: &str = include_str!("../../tests/fixtures/resposta_duplicada.xml");

    #[test]
    fn texto_limpo_remove_tags_e_decodifica_entidades() {
        assert_eq!(
            texto_limpo("<b>Servidor</b>  &amp;  Comiss\u{00e3}o"),
            "Servidor & Comissão"
        );
    }

    #[test]
    fn texto_limpo_colapsa_espacos_multiplos() {
        assert_eq!(texto_limpo("  a   b\n\tc  "), "a b c");
    }

    #[test]
    fn extrai_total_e_documentos_da_resposta_ok() {
        let r = parse_resposta(FIXTURE_OK).expect("deve parsear sem erro");
        assert_eq!(r.total, Some(1234));
        assert_eq!(r.documentos.len(), 3);

        let d1 = &r.documentos[0];
        assert!(d1.link.contains("0123456789abcdef0123456789abcdef"));
        assert_eq!(d1.titulo, "PORTARIA Nº 10 - 2024 - Progressão de servidor");
        assert_eq!(d1.data.as_deref(), Some("10/01/2024"));
        assert!(d1.trecho.contains("SIAPE 1998547"));
        assert_eq!(d1.siapes, vec!["1998547".to_string()]);
    }

    #[test]
    fn documento_sem_rotulo_siape_nao_extrai_siapes_mas_mantem_trecho() {
        let r = parse_resposta(FIXTURE_OK).unwrap();
        let d3 = &r.documentos[2];
        assert!(
            d3.siapes.is_empty(),
            "sem a palavra SIAPE, siapes[] fica vazio"
        );
        assert!(d3.trecho.contains("1998547"));
    }

    #[test]
    fn documento_com_numero_de_processo_nao_referencia_siape_no_trecho() {
        let r = parse_resposta(FIXTURE_OK).unwrap();
        let d2 = &r.documentos[1];
        assert!(d2.siapes.is_empty());
        assert!(!d2.trecho.contains("1998547"));
    }

    #[test]
    fn erro_do_servidor_vira_falha_portal() {
        let erro = parse_resposta(FIXTURE_ERRO).unwrap_err();
        match erro {
            AppError::FalhaPortal { motivo } => assert!(motivo.contains("Sessão expirada")),
            other => panic!("esperava FalhaPortal, veio {other:?}"),
        }
    }

    #[test]
    fn redirect_vira_falha_portal() {
        let erro = parse_resposta(FIXTURE_REDIRECT).unwrap_err();
        assert!(matches!(erro, AppError::FalhaPortal { .. }));
    }

    #[test]
    fn documentos_duplicados_sao_deduplicados_por_link() {
        let r = parse_resposta(FIXTURE_DUPLICADA).expect("deve parsear");
        assert_eq!(r.documentos.len(), 1);
    }

    #[test]
    fn para_documento_converte_mantendo_fidelidade_ao_titulo() {
        let parseado = DocumentoParseado {
            link: "https://gedoc.ifes.edu.br/documento/aaaa?inline".into(),
            titulo: "PORTARIA Nº 5 - 2024 - Assunto qualquer".into(),
            data: Some("01/02/2024".into()),
            trecho: "SIAPE 1998547 designado".into(),
            siapes: vec!["1998547".into()],
        };
        let doc = parseado.para_documento();
        assert_eq!(doc.tipo, TipoDocumento::Portaria);
        assert_eq!(doc.numero.as_deref(), Some("5"));
        assert_eq!(doc.ano, Some(2024));
        assert_eq!(doc.trecho.as_deref(), Some("SIAPE 1998547 designado"));
        assert!(
            doc.contem_siape,
            "contem_siape inicial é true; filtro ajusta depois"
        );
    }
}
