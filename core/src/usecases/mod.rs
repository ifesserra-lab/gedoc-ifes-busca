//! Casos de uso puros e testáveis. Recebem repositórios/portas e caminhos já
//! resolvidos por parâmetro — nenhum `AppHandle`, nenhuma decisão de onde
//! gravar. As bordas (comandos Tauri no desktop, handlers HTTP na web) apenas
//! resolvem os caminhos e chamam estes use-cases.

pub mod buscar;
pub mod documento;
pub mod exportar;
