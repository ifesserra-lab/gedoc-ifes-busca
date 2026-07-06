//! Controllers (MVC) — fronteira do IPC. Funções `#[tauri::command]` que
//! validam a entrada, orquestram Models/Services e devolvem `Result<T,
//! AppError>` (serializável). Ver `contracts/ipc-commands.md`.

pub mod buscar;
pub mod categorias;
pub mod documento;
