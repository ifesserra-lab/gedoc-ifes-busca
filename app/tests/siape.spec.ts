// R10 — validação client-side do SIAPE (US3): espelha o backend
// (`^[0-9]{5,8}$`), dando feedback imediato antes de qualquer chamada IPC.
import { describe, expect, it } from "vitest";

import { validarSiape } from "@/utils/siape";

describe("validarSiape (R10)", () => {
  it("aceita SIAPE com 5 a 8 dígitos", () => {
    for (const termo of ["12345", "123456", "1234567", "12345678"]) {
      expect(validarSiape(termo)).toBe(true);
    }
  });

  it("rejeita menos de 5 dígitos", () => {
    expect(validarSiape("1234")).toBe(false);
  });

  it("rejeita mais de 8 dígitos", () => {
    expect(validarSiape("123456789")).toBe(false);
  });

  it("rejeita caracteres não numéricos, espaços e string vazia", () => {
    for (const termo of ["19985ab", "", " 123456", "123456 ", "12 345"]) {
      expect(validarSiape(termo)).toBe(false);
    }
  });
});
