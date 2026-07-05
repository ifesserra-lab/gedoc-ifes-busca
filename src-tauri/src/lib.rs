//! Backend Rust (Model + Controller) do GeDoc IFES Toolkit.
//!
//! Camadas (Princípio V/IX — MVC):
//! - `domain`: Model — entidades e regras de negócio puras, sem I/O.
//! - `services`: orquestração de Models para um caso de uso.
//! - `ports`: contratos (Repository/Strategy) implementados pela infra.
//! - `commands`: Controller — fronteira `#[tauri::command]` do IPC.
//! - `error`: `AppError` único, serializável, cruza o IPC.

pub mod commands;
pub mod domain;
pub mod error;
pub mod ports;
pub mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::buscar::buscar_por_siape])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar a aplicação Tauri");
}
