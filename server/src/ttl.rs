//! Limpeza por TTL (US2, FR-012). Tarefa em background que varre os
//! diretórios de sessão e remove os inativos além do TTL — nada de PII
//! persiste entre sessões (Princípio II/LGPD).

use std::time::Duration;

use crate::{sessao::now_secs, AppState};

/// Sobe a tarefa periódica de limpeza (intervalo fixo de 10 min).
pub fn spawn_cleanup(state: AppState) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(600));
        loop {
            ticker.tick().await;
            limpar_uma_vez(&state);
        }
    });
}

/// Uma passada de limpeza (também usada nos testes). Remove sessões cujo
/// `.last` seja mais antigo que o TTL.
pub fn limpar_uma_vez(state: &AppState) -> usize {
    let root = state.sessions_root();
    let ttl = state.session_ttl.as_secs();
    let agora = now_secs();
    let mut removidas = 0;

    let Ok(rd) = std::fs::read_dir(&root) else {
        return 0;
    };
    for entrada in rd.flatten() {
        let dir = entrada.path();
        if !dir.is_dir() {
            continue;
        }
        let last = std::fs::read_to_string(dir.join(".last"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);
        if agora.saturating_sub(last) > ttl {
            if std::fs::remove_dir_all(&dir).is_ok() {
                removidas += 1;
            }
        }
    }
    removidas
}
