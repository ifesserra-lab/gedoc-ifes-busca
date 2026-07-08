//! Controller Tauri do GeDoc IFES Toolkit.
//!
//! O núcleo (domínio/serviços/ports/use-cases) vive no crate `gedocs-core`
//! (sem Tauri, reutilizável pela web). Aqui ficam só os comandos
//! `#[tauri::command]` (Controller/IPC) que resolvem os diretórios do app e
//! delegam aos use-cases de `gedocs_core`.

pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::buscar::buscar_por_siape,
            commands::documento::baixar_documento,
            commands::documento::abrir_documento,
            commands::categorias::listar_categorias,
            commands::categorias::salvar_categorias,
            commands::exportar::gerar_relatorio,
            commands::exportar::baixar_zip,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar a aplicação Tauri");
}
