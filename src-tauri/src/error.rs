//! `AppError` — erro único da aplicação, serializável para cruzar o IPC
//! (View em Vue recebe `{ tipo, mensagem }`). Ver contracts/ipc-commands.md.

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize, PartialEq)]
#[serde(tag = "tipo", content = "mensagem")]
pub enum AppError {
    /// R10: `Servidor.siape` e o termo de busca MUST casar `^[0-9]{5,8}$`.
    #[error("SIAPE inválido: '{termo}'. Informe de 5 a 8 dígitos numéricos.")]
    SiapeInvalido { termo: String },

    /// Falha ao comunicar com o portal GeDoc (rede, rate limit após retry,
    /// sessão expirada, layout mudou). Emitida por `services::gedoc_repository`
    /// e pelo adapter `ports::http`.
    #[error("Falha ao comunicar com o portal GeDoc: {motivo}")]
    FalhaPortal { motivo: String },

    /// Falha do serviço de classificação/resumo (IA). TODO em US5/US6.
    #[error("Falha no serviço de IA: {motivo}")]
    FalhaIA { motivo: String },

    /// R5: nome de categoria vazio ao salvar (US8, TODO).
    #[error("Categoria sem nome")]
    CategoriaSemNome,

    /// R5: nome de categoria duplicado ao salvar (US8, TODO).
    #[error("Categoria já existe: '{nome}'")]
    NomeDuplicado { nome: String },

    /// Funcionalidade fora do escopo deste MVP (US1/US2/US3); mensagem clara
    /// em vez de falha silenciosa ou pânico.
    #[error("Recurso ainda não implementado: {0}")]
    NaoImplementado(String),

    /// R7 — falha de I/O em disco (criar diretório, gravar/ler um PDF
    /// baixado) ou nome de arquivo inválido (path traversal: `/`, `\`, `..`
    /// ou vazio). US4.
    #[error("Falha ao acessar arquivo: {motivo}")]
    FalhaArquivo { motivo: String },
}
