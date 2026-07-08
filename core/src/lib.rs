//! Núcleo do GeDoc IFES Toolkit — **sem Tauri**.
//!
//! Camadas (Princípio V/IX — MVC):
//! - `domain`: Model — entidades e regras de negócio puras, sem I/O.
//! - `services`: orquestração de Models para um caso de uso.
//! - `ports`: contratos (Repository/Strategy) implementados pela infra.
//! - `dto`: estruturas de fronteira (input/output dos use-cases).
//! - `usecases`: casos de uso puros/testáveis (recebem caminhos/portas por
//!   parâmetro; nenhum `AppHandle`). Reusados pelo desktop (`src-tauri`,
//!   comandos Tauri) e pela web (`server`, handlers HTTP).
//! - `error`: `AppError` único, serializável.

pub mod domain;
pub mod dto;
pub mod error;
pub mod ports;
pub mod services;
pub mod usecases;
