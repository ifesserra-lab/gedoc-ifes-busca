// R10 — validação client-side do SIAPE, espelhando o backend
// (`src-tauri/src/domain/siape.rs`): `^[0-9]{5,8}$`. Repetida no front para
// dar feedback imediato (sem round-trip de IPC); a validação de verdade
// (fonte de verdade) continua no backend Rust.

export const SIAPE_REGEX = /^[0-9]{5,8}$/;

export function validarSiape(termo: string): boolean {
  return SIAPE_REGEX.test(termo);
}

export const MENSAGEM_SIAPE_INVALIDO =
  "Informe um SIAPE válido: de 5 a 8 dígitos numéricos.";
