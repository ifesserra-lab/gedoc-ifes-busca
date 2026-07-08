//! Relatório consolidado (US7) — Markdown agrupado por categoria e a versão
//! HTML self-contained pronta para abrir no navegador/leitor padrão. Espelha
//! `src/resumir_mistral.py::gerar_markdown` (Python, referência legada) e o
//! CSS A4 de `src/md_para_pdf.py`. Puro: recebe a `ResultadoView` já pronta
//! (US5/US6) e devolve `String`s — nenhuma I/O, nenhum `AppHandle`, nenhum
//! dublê de teste toca disco ou rede (Princípio VII).
//!
//! **Decisão de PDF (US7)**: o script Python gera o PDF via Chrome headless
//! (`md_para_pdf.py::html_para_pdf`, `--print-to-pdf`), o que exigiria
//! depender de um binário externo instalado no SO — frágil (versão, caminho,
//! ausência em CI/máquinas sem Chrome) e fora do espírito "menor
//! dependência" do Tauri. Em vez disso, o relatório é gerado como **HTML
//! self-contained** (CSS inline, sem assets externos) e aberto com o app
//! padrão do sistema via `tauri-plugin-opener` (`commands::exportar`); no
//! navegador, "Imprimir → Salvar como PDF" produz um PDF equivalente, sem
//! nenhuma dependência nova pesada. O `.md` bruto também é salvo ao lado,
//! para quem preferir o texto puro.
//!
//! **Segurança do HTML**: título, categoria, data e resumo podem conter
//! texto arbitrário (PII de terceiros, ou — no modo `llm` — texto gerado por
//! IA a partir do PDF). O Markdown do CommonMark trata sequências como
//! `<script>` cruas como HTML bruto e as repassa **sem escapar** no render
//! final; por isso todo texto de origem externa é escapado (`&`, `<`, `>`)
//! antes de entrar no Markdown. Como o CommonMark também reconhece
//! referências de entidade (`&lt;` etc.) em texto comum e as decodifica de
//! volta ao caractere original antes do render HTML re-escapar o nó de
//! texto, o caractere original chega ao HTML final como entidade — nunca
//! como tag interpretável. Nome de arquivo vai dentro de um code span
//! (`` `arquivo` ``), que o CommonMark também não interpreta como Markdown;
//! o único risco ali é um back-tick literal fechando o span cedo, por isso é
//! substituído antes de compor a string.

use pulldown_cmark::{html, Options, Parser};

use crate::dto::ResultadoView;

const SEM_RESUMO: &str = "_(sem resumo)_";

/// CSS A4 inline, para o HTML ficar bem formatado tanto na tela quanto ao
/// imprimir/"Salvar como PDF". Sem fontes/assets externos — self-contained
/// (Princípio II: nada deste HTML depende de rede para renderizar).
/// Tokens espelhados do design system do app (`app/src/assets/tokens.css`,
/// fonte da verdade em `specs/002-ui-nuxt-minimalista/design-tokens.md`):
/// acento verde-pinho IFES, neutros com viés verde, tipografia Inter. Como o
/// relatório é self-contained, os tokens são **embutidos** aqui (não dá para
/// referenciar o CSS do app). Tema claro (`:root`) + escuro
/// (`prefers-color-scheme`) + impressão (`@media print`, força claro para PDF
/// legível). `Inter` no topo do stack cai para fonte de sistema se ausente
/// (não embutimos o arquivo, para manter o HTML pequeno).
const CSS: &str = r#"
  :root {
    --paper:#f6f8f6; --surface:#ffffff; --surface-2:#f0f4f1;
    --ink:#14211b; --muted:#5e6b64; --border:#e4eae5;
    --accent:#17784e; --accent-soft:#e7f1ec; --accent-contrast:#ffffff;
    --sans:"Inter",-apple-system,"Segoe UI",Roboto,system-ui,sans-serif;
    --mono:ui-monospace,"SF Mono",Menlo,monospace;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --paper:#0e1512; --surface:#151e1a; --surface-2:#1b2621;
      --ink:#e8eeea; --muted:#93a79c; --border:#27332c;
      --accent:#34b37e; --accent-soft:#122a20; --accent-contrast:#0e1512;
    }
  }
  @page { size: A4; margin: 18mm 16mm; }
  * { box-sizing: border-box; }
  body { font-family: var(--sans); background: var(--paper); color: var(--ink);
         line-height: 1.55; font-size: 11pt; margin: 0 auto; max-width: 860px;
         padding: 32px; }
  h1 { font-size: 26px; color: var(--accent); letter-spacing: -0.01em;
       border-bottom: 2px solid var(--accent); padding-bottom: 8px; }
  h2 { font-size: 19px; color: var(--accent); margin-top: 28px;
       border-bottom: 1px solid var(--border); padding-bottom: 6px;
       page-break-before: always; }
  h2:first-of-type { page-break-before: avoid; }
  h3 { font-size: 15px; color: var(--ink); margin: 18px 0 4px;
       page-break-after: avoid; }
  p { margin: 4px 0 12px; }
  a { color: var(--accent); text-decoration: none; }
  table { border-collapse: collapse; width: 100%; margin: 12px 0; font-size: 14px; }
  th, td { border: 1px solid var(--border); padding: 8px 12px; text-align: left; }
  th { background: var(--surface-2); font-weight: 600; }
  td:last-child, th:last-child { text-align: right; font-variant-numeric: tabular-nums; }
  code { background: var(--surface-2); color: var(--muted); padding: 1px 6px;
         border-radius: 6px; font-family: var(--mono); font-size: 12px; }
  h3 + p { color: var(--muted); font-size: 13px; }
  /* Ação "Baixar PDF" (só tela; oculta na impressão via .no-print). */
  .acoes { display: flex; justify-content: flex-end; margin: 0 0 16px; }
  .btn-pdf { font: inherit; font-size: 14px; font-weight: 600;
             color: var(--accent-contrast); background: var(--accent);
             border: none; border-radius: 8px; padding: 8px 16px; cursor: pointer; }
  .btn-pdf:hover { filter: brightness(1.06); }
  /* Impressão: força claro (telas escuras imprimem legível) + sem padding. */
  @media print {
    :root { --paper:#ffffff; --surface:#ffffff; --surface-2:#f0f4f1;
            --ink:#14211b; --muted:#5e6b64; --border:#d7dde6; --accent:#125f3d;
            --accent-contrast:#ffffff; }
    body { padding: 0; }
    .no-print { display: none !important; }
  }
"#;

/// Gera o Markdown do relatório a partir de uma `ResultadoView` já montada
/// (US3/US5/US6): cabeçalho com tabela `| Categoria | Qtd |` e, na sequência,
/// uma seção `## Categoria (N)` por grupo com `### N. Título` + meta (Data ·
/// SIAPE · Original · Arquivo) + o resumo de cada documento — nessa ordem,
/// **sem reordenar** `resultado.categorias`/`itens` (já vêm agrupados e
/// ordenados por `commands::buscar::montar_resultado`; reordenar aqui
/// divergiria do que a tela mostra, R1). Documento sem resumo (`None`, modo
/// `keyword` ou falha isolada de IA) mostra o marcador `_(sem resumo)_` —
/// nunca um resumo inventado (R1).
pub fn gerar_markdown(resultado: &ResultadoView) -> String {
    let total_itens: usize = resultado.categorias.iter().map(|g| g.qtd).sum();

    let mut partes = vec![
        format!(
            "# Relatório de documentos — SIAPE {}",
            escapar_texto(&resultado.termo)
        ),
        String::new(),
        format!(
            "Total: **{total_itens}** documento(s) em **{}** categoria(s).",
            resultado.categorias.len()
        ),
        String::new(),
        "| Categoria | Qtd |".to_string(),
        "| --- | ---: |".to_string(),
    ];

    for grupo in &resultado.categorias {
        partes.push(format!(
            "| {} | {} |",
            escapar_celula(&grupo.categoria),
            grupo.qtd
        ));
    }
    partes.push(format!("| **Total** | **{total_itens}** |"));
    partes.push(String::new());

    for grupo in &resultado.categorias {
        partes.push(format!(
            "## {} ({})",
            escapar_texto(&grupo.categoria),
            grupo.qtd
        ));
        partes.push(String::new());

        for (indice, item) in grupo.itens.iter().enumerate() {
            partes.push(format!(
                "### {}. {}",
                indice + 1,
                escapar_texto(&item.titulo)
            ));
            partes.push(String::new());

            let data = item.data.as_deref().unwrap_or("-");
            let mut meta = format!(
                "**Data:** {} · **SIAPE:** {}",
                escapar_texto(data),
                escapar_texto(&resultado.termo),
            );
            // Só vira link clicável se o esquema for http/https — evita XSS via
            // `href="javascript:"`/`data:` num link malicioso vindo do portal.
            match link_seguro(&item.link) {
                Some(url) => meta.push_str(&format!(" · [Original]({url})")),
                None => meta.push_str(" · Original (link indisponível)"),
            }
            if let Some(arquivo) = &item.arquivo {
                meta.push_str(&format!(" · Arquivo: `{}`", escapar_code_span(arquivo)));
            }
            partes.push(meta);
            partes.push(String::new());

            match &item.resumo {
                Some(resumo) => partes.push(escapar_texto(resumo)),
                None => partes.push(SEM_RESUMO.to_string()),
            }
            partes.push(String::new());
        }
    }

    partes.join("\n")
}

/// Converte Markdown (CommonMark + tabelas) em HTML self-contained (CSS
/// inline, `<meta charset>`, sem assets externos) — pronto para abrir no
/// navegador/leitor padrão e, de lá, "Salvar como PDF" (ver decisão de PDF na
/// doc do módulo). `titulo` vira `<title>` da página; também é escapado
/// (mesmo raciocínio do módulo: nunca interpolar texto externo cru em HTML).
pub fn markdown_para_html(md: &str, titulo: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(md, options);

    let mut corpo = String::new();
    html::push_html(&mut corpo, parser);

    // Botão "Baixar PDF" (só tela; `.no-print` some na impressão). `onclick`
    // inline aciona o print-to-PDF do navegador sem `<script>` externo —
    // mantém o HTML self-contained (spec 008).
    let acoes = "<div class=\"acoes no-print\">\
                 <button class=\"btn-pdf\" type=\"button\" onclick=\"window.print()\">\
                 Baixar PDF</button></div>";
    format!(
        "<!doctype html>\n\
         <html lang=\"pt-BR\"><head><meta charset=\"utf-8\">\n\
         <title>{titulo}</title><style>{css}</style></head>\n\
         <body>{acoes}{corpo}</body></html>",
        titulo = escapar_texto(titulo),
        css = CSS,
        acoes = acoes,
        corpo = corpo,
    )
}

/// Escapa os 3 metacaracteres que dão a um texto poder de virar HTML/tag
/// (`&`, `<`, `>`). Suficiente para o Markdown resultante nunca abrir uma
/// tag: o CommonMark decodifica referências de entidade (`&lt;` etc.) em
/// texto comum de volta ao caractere original antes do render, e o
/// `html::push_html` volta a escapá-lo ao gerar o HTML — o caractere
/// original chega ao HTML final como entidade, nunca como tag.
fn escapar_texto(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// `escapar_texto` + escapa `|`, que dentro de uma célula de tabela GFM
/// romperia a coluna (não é uma questão de segurança, só de layout).
fn escapar_celula(s: &str) -> String {
    escapar_texto(s).replace('|', "\\|")
}

/// Sanitiza o nome de arquivo para dentro de um code span (`` `arquivo` ``):
/// o CommonMark não interpreta Markdown dentro de um code span (e o
/// `html::push_html` sempre escapa seu conteúdo), então o único risco é um
/// back-tick literal fechando o span cedo — substituído por aspas simples só
/// nesta representação textual (não altera o nome real do arquivo em disco).
fn escapar_code_span(s: &str) -> String {
    s.replace('`', "'")
}

/// Retorna a URL segura para `[Original](...)` — **apenas** se o esquema for
/// `http`/`https`. Faz percent-encoding de parênteses (que fechariam a sintaxe
/// do link Markdown cedo). Qualquer outro esquema (`javascript:`, `data:`,
/// `file:`, ...) devolve `None`: o relatório é HTML aberto no navegador, então
/// um `href` malicioso vindo do portal seria XSS clicável. `html::push_html`
/// escapa `&`/aspas do atributo, mas NÃO valida o esquema — por isso a checagem
/// aqui é a barreira real.
fn link_seguro(link: &str) -> Option<String> {
    let l = link.trim();
    let baixo = l.to_ascii_lowercase();
    if baixo.starts_with("http://") || baixo.starts_with("https://") {
        Some(l.replace('(', "%28").replace(')', "%29"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{CategoriaGrupo, DocView};

    fn doc(titulo: &str, resumo: Option<&str>) -> DocView {
        DocView {
            titulo: titulo.to_string(),
            data: Some("10/01/2024".to_string()),
            link: "https://gedoc.ifes.edu.br/documento/aaaa?inline".to_string(),
            arquivo: Some("2024_1_Assunto.pdf".to_string()),
            resumo: resumo.map(str::to_string),
        }
    }

    fn resultado_com(categorias: Vec<CategoriaGrupo>) -> ResultadoView {
        ResultadoView {
            termo: "1998547".to_string(),
            total: categorias.iter().map(|g| g.qtd as u32).sum(),
            categorias,
            tem_pdf: false,
        }
    }

    // --- gerar_markdown ---------------------------------------------------- //

    #[test]
    fn cabecalho_lista_cada_categoria_com_a_quantidade_e_o_total() {
        let resultado = resultado_com(vec![
            CategoriaGrupo {
                categoria: "Progressão".to_string(),
                qtd: 2,
                itens: vec![
                    doc("PORTARIA Nº 1 - 2024 - A", Some("Resumo A.")),
                    doc("PORTARIA Nº 2 - 2024 - B", Some("Resumo B.")),
                ],
            },
            CategoriaGrupo {
                categoria: "Diária".to_string(),
                qtd: 1,
                itens: vec![doc("PORTARIA Nº 3 - 2024 - C", Some("Resumo C."))],
            },
        ]);

        let md = gerar_markdown(&resultado);

        assert!(md.contains("| Categoria | Qtd |"));
        assert!(md.contains("| Progressão | 2 |"));
        assert!(md.contains("| Diária | 1 |"));
        assert!(md.contains("| **Total** | **3** |"));
    }

    #[test]
    fn cada_categoria_vira_uma_secao_com_o_titulo_e_a_quantidade() {
        let resultado = resultado_com(vec![CategoriaGrupo {
            categoria: "Progressão".to_string(),
            qtd: 1,
            itens: vec![doc("PORTARIA Nº 1 - 2024 - Progressão", Some("Resumo."))],
        }]);

        let md = gerar_markdown(&resultado);

        assert!(md.contains("## Progressão (1)"));
        assert!(md.contains("### 1. PORTARIA Nº 1 - 2024 - Progressão"));
    }

    #[test]
    fn item_traz_meta_com_data_siape_link_e_arquivo() {
        let resultado = resultado_com(vec![CategoriaGrupo {
            categoria: "Progressão".to_string(),
            qtd: 1,
            itens: vec![doc("PORTARIA Nº 1 - 2024 - X", Some("Resumo."))],
        }]);

        let md = gerar_markdown(&resultado);

        assert!(md.contains("**Data:** 10/01/2024"));
        assert!(md.contains("**SIAPE:** 1998547"));
        assert!(md.contains("[Original](https://gedoc.ifes.edu.br/documento/aaaa?inline)"));
        assert!(md.contains("Arquivo: `2024_1_Assunto.pdf`"));
    }

    #[test]
    fn resumo_aparece_literal_sem_ser_reescrito_r1() {
        let resultado = resultado_com(vec![CategoriaGrupo {
            categoria: "Progressão".to_string(),
            qtd: 1,
            itens: vec![doc(
                "PORTARIA Nº 1 - 2024 - X",
                Some("Determina a progressão do servidor a partir de 10/01/2024."),
            )],
        }]);

        let md = gerar_markdown(&resultado);

        assert!(md.contains("Determina a progressão do servidor a partir de 10/01/2024."));
    }

    #[test]
    fn documento_sem_resumo_mostra_marcador_em_vez_de_inventar_r1() {
        let resultado = resultado_com(vec![CategoriaGrupo {
            categoria: "Outros".to_string(),
            qtd: 1,
            itens: vec![doc("DESPACHO Nº 9 - 2024 - Y", None)],
        }]);

        let md = gerar_markdown(&resultado);

        assert!(md.contains(SEM_RESUMO));
    }

    #[test]
    fn documento_sem_arquivo_baixado_omite_o_campo_arquivo() {
        let mut d = doc("DESPACHO Nº 9 - 2024 - Y", Some("Resumo."));
        d.arquivo = None;
        let resultado = resultado_com(vec![CategoriaGrupo {
            categoria: "Outros".to_string(),
            qtd: 1,
            itens: vec![d],
        }]);

        let md = gerar_markdown(&resultado);

        assert!(!md.contains("Arquivo:"));
    }

    #[test]
    fn sem_categorias_gera_so_o_cabecalho_sem_quebrar() {
        let resultado = resultado_com(Vec::new());

        let md = gerar_markdown(&resultado);

        assert!(md.contains("| **Total** | **0** |"));
        assert!(!md.contains("##"));
    }

    #[test]
    fn preserva_a_ordem_ja_vinda_em_resultado_categorias() {
        let resultado = resultado_com(vec![
            CategoriaGrupo {
                categoria: "Zebra".to_string(),
                qtd: 1,
                itens: vec![doc("PORTARIA Nº 1 - 2024 - Z", Some("R."))],
            },
            CategoriaGrupo {
                categoria: "Alfa".to_string(),
                qtd: 1,
                itens: vec![doc("PORTARIA Nº 2 - 2024 - A", Some("R."))],
            },
        ]);

        let md = gerar_markdown(&resultado);
        let pos_zebra = md.find("## Zebra").expect("Zebra presente");
        let pos_alfa = md.find("## Alfa").expect("Alfa presente");

        assert!(
            pos_zebra < pos_alfa,
            "não deve reordenar alfabeticamente — segue a ordem de entrada"
        );
    }

    #[test]
    fn titulo_e_resumo_com_caracteres_html_sao_escapados_no_markdown() {
        let resultado = resultado_com(vec![CategoriaGrupo {
            categoria: "Outros".to_string(),
            qtd: 1,
            itens: vec![doc(
                "PORTARIA <script>alert(1)</script> Nº 1",
                Some("Resumo com <b>tag</b> & símbolo."),
            )],
        }]);

        let md = gerar_markdown(&resultado);

        assert!(!md.contains("<script>"));
        assert!(md.contains("&lt;script&gt;"));
        assert!(md.contains("&amp;"));
    }

    #[test]
    fn nome_de_categoria_com_pipe_nao_quebra_a_tabela() {
        let resultado = resultado_com(vec![CategoriaGrupo {
            categoria: "A | B".to_string(),
            qtd: 1,
            itens: vec![doc("PORTARIA Nº 1 - 2024 - X", Some("R."))],
        }]);

        let md = gerar_markdown(&resultado);

        assert!(md.contains("| A \\| B | 1 |"));
    }

    // --- markdown_para_html -------------------------------------------------- //

    #[test]
    fn html_e_self_contained_com_meta_charset_e_titulo() {
        let html = markdown_para_html("# Título\n\nTexto.", "Relatório SIAPE 1998547");

        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("<meta charset=\"utf-8\">"));
        assert!(html.contains("<title>Relatório SIAPE 1998547</title>"));
        assert!(html.contains("<style>"));
        assert!(!html.contains("<script")); // sem asset/JS externo
        assert!(!html.contains("http://") && !html.contains("https://"));
    }

    #[test]
    fn html_usa_o_design_system_do_app_e_suporta_tema_escuro() {
        let html = markdown_para_html("# T\n\ntexto", "Relatório");

        // Tokens do app (acento verde-pinho IFES) + tipografia Inter (FR-001).
        assert!(
            html.contains("--accent"),
            "deve usar os tokens de cor do app"
        );
        assert!(html.contains("#17784e"), "acento do design (claro)");
        assert!(html.contains("Inter"), "tipografia do app");
        // Tema escuro (FR-002) e impressão legível (FR-005).
        assert!(html.contains("prefers-color-scheme: dark"));
        assert!(html.contains("@media print"));
        // Self-contained (FR-003): nenhum import/asset externo de estilo/fonte.
        assert!(!html.contains("@import"));
        assert!(!html.contains("url(http"));
    }

    #[test]
    fn html_tem_botao_baixar_pdf_oculto_na_impressao() {
        let html = markdown_para_html("# T\n\ntexto", "Relatório");

        // Botão visível "Baixar PDF" que aciona o print-to-PDF do navegador.
        assert!(html.contains("Baixar PDF"));
        assert!(html.contains("onclick=\"window.print()\""));
        assert!(html.contains("class=\"acoes no-print\""));
        // Ocultado na impressão (não sai no PDF) — spec 008 FR-003.
        assert!(html.contains(".no-print"));
        // Sem `<script>` externo — self-contained mantido (onclick inline).
        assert!(!html.contains("<script"));
    }

    #[test]
    fn tabela_markdown_vira_tabela_html_real() {
        let md = "| Categoria | Qtd |\n| --- | ---: |\n| Progressão | 2 |\n";
        let html = markdown_para_html(md, "Relatório");

        assert!(html.contains("<table>"));
        assert!(html.contains("<th>Categoria</th>"));
        assert!(html.contains("<td>Progressão</td>"));
    }

    #[test]
    fn headings_markdown_viram_headings_html() {
        let md = "# H1\n\n## H2\n\n### H3\n";
        let html = markdown_para_html(md, "Relatório");

        assert!(html.contains("<h1>H1</h1>"));
        assert!(html.contains("<h2>H2</h2>"));
        assert!(html.contains("<h3>H3</h3>"));
    }

    #[test]
    fn titulo_da_pagina_e_escapado() {
        let html = markdown_para_html("texto", "<script>alert(1)</script>");

        assert!(!html.contains("<title><script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    // --- pipeline completo: gerar_markdown -> markdown_para_html (R1/XSS) ---- //

    #[test]
    fn pipeline_completo_nunca_produz_uma_tag_script_executavel() {
        let resultado = resultado_com(vec![CategoriaGrupo {
            categoria: "Outros".to_string(),
            qtd: 1,
            itens: vec![doc(
                "PORTARIA <script>alert('xss')</script> Nº 1",
                Some("Resumo <img src=x onerror=alert(1)> perigoso."),
            )],
        }]);

        let md = gerar_markdown(&resultado);
        let html = markdown_para_html(&md, "Relatório");

        assert!(!html.contains("<script>"));
        assert!(!html.contains("<img"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn link_com_esquema_perigoso_nunca_vira_href_clicavel() {
        // Vetor real: o `link` chega do portal e é aberto no navegador. Um
        // esquema `javascript:`/`data:` não pode virar um href executável.
        for esquema in [
            "javascript:alert(document.cookie)",
            "data:text/html,<script>alert(1)</script>",
        ] {
            let item = DocView {
                titulo: "PORTARIA Nº 1 - 2024 - X".to_string(),
                data: Some("10/01/2024".to_string()),
                link: format!("{esquema}//documento/0123456789abcdef0123456789abcdef"),
                arquivo: None,
                resumo: Some("Resumo.".to_string()),
            };
            let resultado = resultado_com(vec![CategoriaGrupo {
                categoria: "Outros".to_string(),
                qtd: 1,
                itens: vec![item],
            }]);

            let md = gerar_markdown(&resultado);
            let html = markdown_para_html(&md, "Relatório");

            assert!(
                !md.contains("[Original](javascript:"),
                "md não pode linkar javascript:"
            );
            assert!(
                !html.contains("href=\"javascript:"),
                "html: sem href javascript:"
            );
            assert!(!html.contains("href=\"data:"), "html: sem href data:");
            assert!(
                md.contains("Original (link indisponível)"),
                "link inseguro é rotulado, não clicável"
            );
        }
    }

    #[test]
    fn pipeline_completo_reflete_o_resumo_real_sem_inventar_r1() {
        let resultado = resultado_com(vec![CategoriaGrupo {
            categoria: "Progressão".to_string(),
            qtd: 1,
            itens: vec![doc(
                "PORTARIA Nº 1 - 2024 - X",
                Some("Determina a progressão funcional do servidor."),
            )],
        }]);

        let md = gerar_markdown(&resultado);
        let html = markdown_para_html(&md, "Relatório");

        assert!(html.contains("Determina a progressão funcional do servidor."));
    }
}
