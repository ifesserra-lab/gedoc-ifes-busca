//! Extração de texto de PDF (US6) — via a crate `pdf-extract` (puro Rust,
//! sem depender de um binário externo como `pdftotext`, ao contrário do
//! script de referência `src/resumir_mistral.py::extrair_texto`, que chama
//! `pdftotext` via `subprocess`). Sem dependência de sistema: nenhum binário
//! externo precisa estar instalado na máquina do usuário.
//!
//! PDF ilegível, corrompido, criptografado, sem texto extraível ou vazio →
//! `None`: nunca panica, nunca propaga erro (R11 — quem chama cai no trecho
//! da busca). O texto extraído pode conter PII de terceiros (Princípio
//! II/LGPD); esta função nunca loga nem persiste o texto — só o devolve em
//! memória para quem chamou (`services::resumidor`) decidir o que fazer.

/// Nº máximo de caracteres do texto extraído (espelha `MAX_CHARS = 6000` do
/// Python de referência) — salvaguarda contra PDFs enormes. `services::
/// resumidor` aplica o mesmo limite ao texto-fonte final (PDF ou trecho),
/// qualquer que seja a origem, então esta truncagem aqui é redundante em
/// segurança, não em correção.
pub const MAX_CHARS: usize = 6000;

/// Extrai o texto de um PDF a partir dos bytes já em memória (esta função
/// nunca lê o disco; quem chama decide de onde vêm os bytes — ver
/// `services::resumidor::texto_do_pdf`). `None` em qualquer falha (bytes
/// inválidos, PDF corrompido/criptografado, sem texto) ou quando o
/// resultado fica vazio após `trim`.
pub fn extrair_texto(bytes: &[u8]) -> Option<String> {
    // `pdf-extract` tem muitos `assert!`/`unwrap()` internos que PANICAM em
    // PDFs estruturalmente válidos mas semanticamente corrompidos (ex.:
    // `/Widths` menor que `LastChar-FirstChar+1`). Sem `catch_unwind`, esse
    // panic subiria por `resumir_lote` e abortaria o lote inteiro dentro do
    // `spawn_blocking` — violando R11 e a promessa "nunca panica" acima.
    let extracao = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pdf_extract::extract_text_from_mem(bytes)
    }));
    let texto = match extracao {
        Ok(Ok(t)) => t,
        _ => return None, // erro de extração OU panic capturado
    };
    let texto = texto.trim();
    if texto.is_empty() {
        None
    } else {
        Some(truncar(texto, MAX_CHARS))
    }
}

/// Trunca por `char` (não por byte) para nunca quebrar um caractere UTF-8 no
/// meio.
fn truncar(texto: &str, max_chars: usize) -> String {
    texto.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PDF mínimo, gerado à mão (sem compressão, xref correto), com conteúdo
    /// fictício — ver `src-tauri/tests/fixtures/documento_teste.pdf`. Não
    /// contém nenhum dado real (Princípio II/LGPD: fixtures só com conteúdo
    /// genérico).
    const PDF_FIXTURE: &[u8] = include_bytes!("../../tests/fixtures/documento_teste.pdf");

    #[test]
    fn extrai_texto_de_um_pdf_valido() {
        let texto = extrair_texto(PDF_FIXTURE).expect("deve extrair texto do PDF fixture");
        assert!(
            texto.contains("Documento de teste"),
            "texto extraído inesperado: {texto:?}"
        );
    }

    #[test]
    fn bytes_invalidos_retornam_none_sem_panico() {
        assert_eq!(extrair_texto(b"isto nao e um pdf"), None);
    }

    #[test]
    fn bytes_vazios_retornam_none_sem_panico() {
        assert_eq!(extrair_texto(b""), None);
    }

    #[test]
    fn pdf_estruturalmente_quebrado_retorna_none_sem_panico() {
        // Metade do PDF válido: estrutura parcial que leva `pdf-extract` a
        // erro ou panic interno — `catch_unwind` garante `None`, não crash.
        let truncado = &PDF_FIXTURE[..PDF_FIXTURE.len() / 2];
        assert_eq!(extrair_texto(truncado), None);
        // Cabeçalho de PDF seguido de lixo (passa da checagem de assinatura).
        assert_eq!(
            extrair_texto(b"%PDF-1.4\n\xff\xff\xff garbage \x00\x01"),
            None
        );
    }

    #[test]
    fn truncar_respeita_o_limite_de_caracteres_sem_quebrar_utf8() {
        let texto = "á".repeat(10); // multi-byte, para provar corte por char
        assert_eq!(truncar(&texto, 3), "ááá");
        assert_eq!(truncar(&texto, 100), texto);
    }
}
