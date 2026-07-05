//! Repository (GoF/DDD) — abstrai a fonte de documentos do portal GeDoc.
//!
//! A implementação real (sessão HTTP, `ViewState`, IDs JSF descobertos em
//! runtime — R8, paginação completa — FR-001) é **TODO** da US1
//! (`src-tauri/src/services/gedoc_repository.rs`, ainda não escrito). Este
//! MVP entrega apenas o contrato, para que `commands::buscar` já possa
//! depender de uma abstração testável por dublê em vez de um cliente HTTP
//! concreto.

use crate::domain::documento::Documento;
use crate::error::AppError;

/// Códigos de repositório aceitos pelo portal (ver `docs/ontology.yaml`
/// `RepositorioCodigo`): `0` Boletim, `1` GeDoc (padrão), `2` Site.
pub const REPOSITORIO_PADRAO: &str = "1";

pub trait GedocRepository {
    /// Busca todos os documentos (todas as páginas) para `termo` no
    /// `repositorio` informado. Retorna o total bruto relatado pelo portal
    /// e os documentos coletados (sem filtrar por SIAPE — isso é
    /// responsabilidade de `services::filtro`, R2).
    fn buscar(&self, termo: &str, repositorio: &str) -> Result<(u32, Vec<Documento>), AppError>;
}
