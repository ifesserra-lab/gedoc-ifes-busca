//! Utilitários de extração de texto compartilhados entre `documento` e
//! `nome_arquivo` (Princípio V — DRY: uma única fonte para "número" e "ano").

use std::sync::LazyLock;

use regex::Regex;

static RE_NUM_TITULO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)N[ºo°]\s*(\d+)").unwrap());
static RE_ANO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(?:19|20)\d{2}\b").unwrap());

/// Número do ato, extraído após "Nº" no título (ex.: "PORTARIA Nº 123" -> "123").
pub fn extrair_numero(titulo: &str) -> Option<String> {
    RE_NUM_TITULO.captures(titulo).map(|c| c[1].to_string())
}

/// Ano do ato: do título; na ausência, o ano final de `data` (DD/MM/AAAA).
pub fn extrair_ano(titulo: &str, data: Option<&str>) -> Option<u16> {
    if let Some(m) = RE_ANO.find(titulo) {
        return m.as_str().parse().ok();
    }
    data.and_then(|d| d.rsplit('/').next())
        .and_then(|s| s.parse().ok())
}

/// Regex bruta de "Nº NUM", reutilizada por `nome_arquivo` para remover o
/// trecho do número ao montar o "assunto" do arquivo (Princípio V — DRY).
pub(crate) fn regex_numero_titulo() -> &'static Regex {
    &RE_NUM_TITULO
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrai_numero_apos_simbolo_numero() {
        assert_eq!(extrair_numero("PORTARIA Nº 123 - 2024"), Some("123".into()));
        assert_eq!(extrair_numero("DESPACHO No 45"), Some("45".into()));
        assert_eq!(extrair_numero("sem numero aqui"), None);
    }

    #[test]
    fn extrai_ano_do_titulo_ou_da_data() {
        assert_eq!(extrair_ano("PORTARIA Nº 1 - 2024 - Assunto", None), Some(2024));
        assert_eq!(extrair_ano("sem ano no titulo", Some("05/03/2023")), Some(2023));
        assert_eq!(extrair_ano("sem ano nem data", None), None);
    }
}
