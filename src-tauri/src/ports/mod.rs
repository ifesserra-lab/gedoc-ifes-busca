//! Ports (Dependency Inversion) — contratos que o domínio define e a infra
//! implementa. Permitem dublês em teste (Princípio VII: sem rede nos testes)
//! e trocar estratégias sem alterar quem consome (Princípio IX).

pub mod classificador;
pub mod gedoc_repository;
