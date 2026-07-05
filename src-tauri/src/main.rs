// Previne uma janela de console adicional no Windows em builds release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    gedocs_lib::run();
}
