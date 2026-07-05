//! R3 — nome determinístico do PDF: `AAAA_NUMERO_ASSUNTO.pdf`, derivado do
//! título (ano do título ou, na ausência, da data); colisões recebem sufixo.
//! Fonte de referência (legado): `src/buscar_gedoc.py::nome_arquivo` e
//! `_nome_unico`.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use super::documento::Documento;
use super::texto::{extrair_ano, extrair_numero, regex_numero_titulo};

static RE_PREFIXO_TIPO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*[A-Za-zÀ-ÿ]+\s*").unwrap());
static RE_ANO_PREFIXO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*-?\s*(?:19|20)\d{2}\s*-?\s*").unwrap());
static RE_CHARS_ILEGAIS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[/\\:*?"<>|\n\r\t]"#).unwrap());
static RE_ESPACOS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

const TAMANHO_MAX_NOME: usize = 180;

/// Deriva `AAAA_NUMERO_ASSUNTO.pdf` a partir do título (e, se necessário, da
/// data) do documento. Não sobrescreve nada em disco — só calcula o nome;
/// colisões são responsabilidade de [`nome_unico`].
pub fn nome_arquivo(doc: &Documento) -> String {
    let titulo = &doc.titulo;

    let numero = extrair_numero(titulo).unwrap_or_else(|| "0".to_string());
    let ano = extrair_ano(titulo, doc.data.as_deref())
        .map(|a| a.to_string())
        .unwrap_or_else(|| "0000".to_string());

    let mut assunto = RE_PREFIXO_TIPO.replace(titulo, "").into_owned();
    assunto = regex_numero_titulo().replacen(&assunto, 1, "").into_owned();
    assunto = RE_ANO_PREFIXO.replace(&assunto, "").into_owned();
    assunto = assunto
        .trim_matches(|c: char| " -–—".contains(c))
        .to_string();

    let bruto = format!("{ano}_{numero}_{assunto}");
    let saneado = RE_CHARS_ILEGAIS.replace_all(&bruto, "_");
    let normalizado = RE_ESPACOS
        .replace_all(&saneado, " ")
        .trim()
        .trim_end_matches('.')
        .to_string();

    let truncado: String = normalizado.chars().take(TAMANHO_MAX_NOME).collect();
    format!("{truncado}.pdf")
}

/// Evita sobrescrever arquivos com o mesmo nome derivado: acrescenta
/// ` (2)`, ` (3)`, ... até encontrar um nome livre em `usados`.
pub fn nome_unico(nome: &str, usados: &mut HashSet<String>) -> String {
    if usados.insert(nome.to_string()) {
        return nome.to_string();
    }
    let (base, ext) = match nome.rsplit_once('.') {
        Some((b, e)) => (b.to_string(), format!(".{e}")),
        None => (nome.to_string(), String::new()),
    };
    let mut i = 2;
    loop {
        let candidato = format!("{base} ({i}){ext}");
        if usados.insert(candidato.clone()) {
            return candidato;
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deriva_nome_com_ano_do_titulo_e_numero() {
        let d = Documento::novo(
            "link",
            "PORTARIA Nº 123 - 2024 - Progressão funcional de servidor",
        );
        assert_eq!(
            nome_arquivo(&d),
            "2024_123_Progressão funcional de servidor.pdf"
        );
    }

    #[test]
    fn usa_ano_da_data_quando_titulo_nao_tem_ano() {
        let mut d = Documento::novo("link", "DESPACHO Nº 45 - Anuência de férias");
        d.data = Some("05/03/2023".to_string());
        assert!(nome_arquivo(&d).starts_with("2023_45_"));
    }

    #[test]
    fn usa_valores_padrao_sem_numero_ano_ou_data() {
        let d = Documento::novo("link", "Comunicado interno qualquer");
        assert!(nome_arquivo(&d).starts_with("0000_0_"));
    }

    #[test]
    fn saneia_caracteres_invalidos() {
        let d = Documento::novo(
            "link",
            "PORTARIA Nº 7 - 2024 - Assunto/Com: caracteres*inválidos?",
        );
        let nome = nome_arquivo(&d);
        for c in ['/', '\\', ':', '*', '?', '"', '<', '>', '|'] {
            assert!(!nome.contains(c), "não deveria conter '{c}': {nome}");
        }
        assert!(nome.ends_with(".pdf"));
    }

    #[test]
    fn colisao_recebe_sufixo_sem_sobrescrever() {
        let mut usados = HashSet::new();
        let a = nome_unico("2024_1_Assunto.pdf", &mut usados);
        let b = nome_unico("2024_1_Assunto.pdf", &mut usados);
        let c = nome_unico("2024_1_Assunto.pdf", &mut usados);
        assert_eq!(a, "2024_1_Assunto.pdf");
        assert_eq!(b, "2024_1_Assunto (2).pdf");
        assert_eq!(c, "2024_1_Assunto (3).pdf");
        assert_eq!(usados.len(), 3);
    }

    #[test]
    fn nomes_diferentes_nao_colidem() {
        let mut usados = HashSet::new();
        let a = nome_unico("a.pdf", &mut usados);
        let b = nome_unico("b.pdf", &mut usados);
        assert_eq!(a, "a.pdf");
        assert_eq!(b, "b.pdf");
    }
}
