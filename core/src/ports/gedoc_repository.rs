//! Repository (GoF/DDD) — abstrai a fonte de documentos do portal GeDoc.
//!
//! A implementação real (sessão HTTP, `ViewState`, IDs JSF descobertos em
//! runtime — R8, paginação completa — FR-001) vive em
//! `services::gedoc_repository::GedocRepositoryHttp`. Depender deste contrato
//! (e não do cliente HTTP concreto) mantém `commands::buscar` testável por
//! dublê, sem tocar a rede (Princípio VII).

use crate::domain::documento::Documento;
use crate::error::AppError;

/// Códigos de repositório aceitos pelo portal (ver `docs/ontology.yaml`
/// `RepositorioCodigo`): `0` Boletim, `1` GeDoc, `2` Site.
///
/// Padrão = **todos** os repositórios (o portal agrega Boletim+GeDoc+Site).
/// Buscar só o GeDoc perdia documentos antigos que vivem no Boletim (ex.: atos
/// de 2006). Um código concreto (`0`/`1`/`2`) restringe a esse repositório; o
/// sentinela `"todos"` (default) faz `GedocRepositoryHttp` agregar os três.
pub const REPOSITORIO_PADRAO: &str = "todos";

pub trait GedocRepository {
    /// Busca todos os documentos (todas as páginas) para `termo` no
    /// `repositorio` informado. Retorna o total bruto relatado pelo portal
    /// e os documentos coletados (sem filtrar por SIAPE — isso é
    /// responsabilidade de `services::filtro`, R2).
    fn buscar(&self, termo: &str, repositorio: &str) -> Result<(u32, Vec<Documento>), AppError>;
}
