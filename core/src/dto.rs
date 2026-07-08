//! DTOs de fronteira dos use-cases — input/output que cruza a borda (IPC no
//! desktop, HTTP na web). Sem lógica de negócio; apenas dados + serde.

use serde::{Deserialize, Serialize};

use crate::domain::documento::Documento;

#[derive(Debug, Deserialize)]
pub struct BuscarPorSiapeInput {
    pub siape: String,
    pub repositorio: Option<String>,
    /// Estratégia de classificação (US5): `"keyword"` (default) ou `"llm"`.
    /// Ausente ou desconhecido => `keyword` (nunca falha por valor inesperado).
    pub modo: Option<String>,
}

// `Deserialize` também é necessário (não só `Serialize`) porque
// `usecases::exportar::executar_gerar_relatorio` recebe de volta a mesma
// `ResultadoView` que a busca devolveu à View — o relatório é gerado a partir
// do que já está na tela (R1), sem refazer a busca.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocView {
    pub titulo: String,
    pub data: Option<String>,
    pub link: String,
    pub arquivo: Option<String>,
    pub resumo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CategoriaGrupo {
    pub categoria: String,
    pub qtd: usize,
    pub itens: Vec<DocView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
            arquivo: doc.arquivo, // preenchido só se o download (US4) já rodou
            resumo: doc.resumo,   // preenchido pelo resumo (US6) no modo `llm`
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct BaixarDocumentoInput {
    pub siape: String,
    pub link: String,
    pub titulo: String,
    pub data: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AbrirDocumentoInput {
    pub siape: String,
    pub arquivo: String,
}
